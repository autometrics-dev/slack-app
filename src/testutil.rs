use crate::db::Db;
use crate::events::Event;
use crate::service::{ChartServiceConfig, PrometheusServiceConfig, Service, SlackServiceConfig};
use futures::{Future, FutureExt};
use sqlx::pool::PoolConnection;
use sqlx::{Sqlite, SqlitePool};
use std::panic::AssertUnwindSafe;
use tokio::sync::mpsc::Receiver;
use url::Url;

#[macro_export]
macro_rules! assert_matches {
    ($expression:expr, $pattern:pat $( if $guard: expr )? $(,)?) => {
        match $expression {
            $pattern $( if $guard )? => (),
            o => ::core::panic!("match did not pass; got: {:?}", o)
        }
    }
}

pub async fn run_test<S, T, X, Y, Z>(
    setup: impl FnOnce() -> X,
    cleanup: impl FnOnce(T) -> Y,
    test: impl FnOnce(S) -> Z,
) where
    X: Future<Output = (S, T)>,
    Y: Future<Output = ()>,
    Z: Future<Output = ()>,
{
    // Setup
    let (test_ctx, teardown_ctx) = setup().await;

    // Test
    let fut = AssertUnwindSafe(test(test_ctx));
    let result = fut.catch_unwind().await;

    // Teardown
    cleanup(teardown_ctx).await;
    assert!(result.is_ok())
}

pub struct ServiceContext {
    pub service: Service,
    pub db: Db,
}

pub struct ServiceCleanup {
    db_cleanup: SqliteCleanup,
    // The receiver is kept so the channel doesn't close before cleanup.
    _event_receiver: Receiver<Event>,
}

pub async fn service_setup() -> (ServiceContext, ServiceCleanup) {
    let (pool, db_cleanup) = sqlite_setup().await;

    let db = Db::new(pool.clone());

    let (event_sender, event_receiver) = tokio::sync::mpsc::channel::<Event>(16);
    let service = Service::new(
        Url::parse("http://localhost:3031").unwrap(),
        ChartServiceConfig::new_test_config(),
        db.clone(),
        event_sender,
        Some(Url::parse("http://explorer.pmmp.dev").unwrap()),
        PrometheusServiceConfig::new_test_config(),
        SlackServiceConfig::new_test_config("12345678".to_owned()),
    );

    let service_context = ServiceContext { service, db };

    let service_cleanup = ServiceCleanup {
        db_cleanup,
        _event_receiver: event_receiver,
    };

    (service_context, service_cleanup)
}

pub async fn service_cleanup(context: ServiceCleanup) {
    sqlite_cleanup(context.db_cleanup).await;
}

struct SqliteCleanup {
    pool: SqlitePool,
}

async fn sqlite_setup() -> (SqlitePool, SqliteCleanup) {
    // Randomize test DBs to avoid collissions between parallel tests.
    let pool = SqlitePool::connect(":memory:")
        .await
        .expect("Could not create DB");

    let mut connection: PoolConnection<Sqlite> = pool.acquire().await.unwrap();
    sqlx::migrate!("./migrations")
        .run(&mut connection)
        .await
        .expect("Could not initialize DB");

    (pool.clone(), SqliteCleanup { pool })
}

async fn sqlite_cleanup(cleanup: SqliteCleanup) {
    // make sure the pool is closed
    cleanup.pool.close().await;
}
