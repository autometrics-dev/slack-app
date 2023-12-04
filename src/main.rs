#![forbid(unsafe_code)]

#[cfg(test)]
#[macro_use]
mod testutil;

mod db;
mod events;
mod service;

use anyhow::{bail, Context, Result};
use autometrics::prometheus_exporter;
use axum::Server;
use clap::{Parser, ValueEnum};
use db::Db;
use events::Event;
use opentelemetry::sdk::{trace, Resource};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use service::event_loop::handle_events;
use service::{ChartServiceConfig, PrometheusServiceConfig, Service, SlackServiceConfig};
use std::net::IpAddr;
use std::process::ExitCode;
use std::{env, io};
use tokio::select;
use tracing::{error, info};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::{Layer, SubscriberExt};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Registry};
use url::Url;

/// Slack app for sending Slack alerts
#[derive(Parser)]
#[clap(
    name = "Fiberplane API",
    author = "Team Fiberplane",
    version = clap::crate_version!()
)]
struct CliArguments {
    /// Log using JSON.
    #[clap(long, env = "LOG_JSON")]
    json: bool,

    /// Enable tracing support.
    ///
    /// Use `--otlp-endpoint` to specify where the traces should be sent.
    #[clap(long, env)]
    tracing: bool,

    /// Endpoint of the OTLP collector.
    #[clap(long, env, default_value = "http://localhost:4317")]
    otlp_endpoint: Url,

    #[clap(flatten)]
    serve_args: ServeArguments,
}

#[derive(Parser)]
struct DbArguments {
    /// Sqlite connection string.
    #[clap(long, env, default_value = "sqlite:///tmp/slack-app.db?mode=rwc")]
    db_connection_string: String,
}

#[derive(Parser)]
struct ServeArguments {
    #[clap(flatten)]
    chart_config: ChartServiceConfig,

    #[clap(flatten)]
    db: DbArguments,

    /// Server port number
    #[clap(long, short, env, default_value = "3031")]
    port: u16,

    /// Hostname to listen on
    #[clap(long, short = 'H', env, default_value = "127.0.0.1")]
    listen_host: IpAddr,

    /// Base URL on which the service will be hosted.
    #[clap(long, env, default_value = "http://localhost:3031")]
    base_url: Url,

    /// Base URL on which Explorer will be hosted.
    #[clap(long, env)]
    explorer_url: Option<Url>,

    #[clap(flatten)]
    slack_config: SlackServiceConfig,

    #[clap(flatten)]
    prometheus_config: PrometheusServiceConfig,
}

#[derive(ValueEnum, Debug, Copy, Clone, Eq, Ord, PartialOrd, PartialEq)]
enum EmailProvider {
    /// Send emails using Amazon SES
    Ses,
    /// Only log emails instead of actually sending them
    Log,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    // Do not use try_parse here.
    // try_parse will fail if we pass --version flag, as it will not contain
    // any subcommand.
    let args = CliArguments::parse();

    let result = initialize_logger(&args);
    if let Err(err) = result {
        error!(%err, "Unable to initialize logger");
        return ExitCode::FAILURE;
    }

    if let Err(err) = prometheus_exporter::try_init() {
        error!(?err, "Failed to initialize Prometheus exporter");
        return ExitCode::FAILURE;
    };

    let result = handle_serve(args.serve_args).await;

    if let Err(err) = result {
        error!(%err, "Command executed unsuccessfully");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

fn initialize_logger(args: &CliArguments) -> Result<()> {
    // The filter layer controls which log levels to display.
    let filter_layer = EnvFilter::from_default_env();

    // The log layer controls the output of log events to stderr. Depending on the
    // `json` flag, it will either be human readable or json encoded.
    let log_layer = tracing_subscriber::fmt::layer().with_writer(io::stderr);
    let log_layer = if args.json {
        log_layer.json().boxed()
    } else {
        log_layer.boxed()
    };

    // The trace layer will send traces to the configured tracing backend
    // depending on the `tracing` flag.
    let trace_layer = if args.tracing {
        // This tracer is responsible for sending the actual traces.
        let tracer =
            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(args.otlp_endpoint.to_string()),
                )
                .with_trace_config(trace::config().with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "slack-app"),
                ])))
                .install_batch(opentelemetry::runtime::Tokio)
                .context("unable to install tracer")?;

        // This layer will take the traces from the `tracing` crate and send
        // them to the tracer specified above.
        Some(OpenTelemetryLayer::new(tracer))
    } else {
        None
    };

    Registry::default()
        .with(filter_layer)
        .with(log_layer)
        .with(trace_layer)
        .try_init()
        .context("unable to initialize logger")?;

    Ok(())
}

async fn handle_serve(args: ServeArguments) -> Result<()> {
    let connection_string = &args.db.db_connection_string;
    info!(connection_string, "Opening Sqlite DB");

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(connection_string)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let db = Db::new(pool.clone());

    let commit = option_env!("GITHUB_SHA").unwrap_or("unknown");

    info!(
        port = ?args.port,
        listen_host = ?args.listen_host,
        ?commit,
        base_url = %args.base_url,
        "Starting server"
    );

    let (event_sender, event_receiver) = tokio::sync::mpsc::channel::<Event>(64);
    let service = Service::new(
        args.base_url,
        args.chart_config,
        db,
        event_sender.clone(),
        args.explorer_url,
        args.prometheus_config,
        args.slack_config,
    );

    let app = service::router::create_router(service.clone());

    let service_task = tokio::spawn(async move {
        let mut service = service;

        handle_events(&mut service, event_receiver)
            .await
            .expect("Unable to handle event");
    });

    let (shutdown_trigger, mut shutdown_signal) = tokio::sync::mpsc::channel::<()>(1);
    let addr = (args.listen_host, args.port).into();
    let server_task = tokio::spawn(async move {
        Server::bind(&addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(async {
                shutdown_signal.recv().await;
                info!("graceful shutdown request received");
            })
            .await
            .expect("server error");
    });

    // Graceful shutdown detection
    match tokio::signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {err}");
            // we also shut down in case of error
        }
    };

    event_sender
        .send(Event::Shutdown)
        .await
        .expect("Could not send shutdown event");
    shutdown_trigger
        .send(())
        .await
        .expect("Could not trigger shutdown signal");

    let api_tasks = futures::future::join_all(vec![service_task, server_task]);

    select! {
        _ = tokio::signal::ctrl_c() => {
            bail!("forced shutdown from additional signal")
        }
        task_result = api_tasks => {
            match (task_result.get(0).unwrap(), task_result.get(1).unwrap()) {
                (Ok(_), Ok(_)) => info!("shutdown complete"),
                (Ok(_), Err(server_error)) => {
                    error!(?server_error, "server error during shutdown");
                    bail!("server error during shutdown: {server_error}")
                }
                (Err(service_error), Ok(_)) => {
                    error!(?service_error, "service error during shutdown");
                    bail!("service error during shutdown: {service_error}");
                }
                (Err(service_error), Err(server_error)) => {
                    error!(
                        ?service_error,
                        ?server_error,
                        "server and service errored during shutdown"
                    );
                    bail!("server and service errored during shutdown: {server_error}; {service_error}");
                }
            };
            Ok(())
        }
    }
}
