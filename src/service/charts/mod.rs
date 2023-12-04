mod errors;

pub mod handlers;

use autometrics::autometrics;
use mondrian_charts::*;
use once_cell::sync::Lazy;
use std::path::PathBuf;
use tracing::{debug, instrument};

pub use errors::{ChartHandlerError, ChartServiceError};

const INTER_FONT: &[u8] = include_bytes!("../../../assets/Inter-Regular.ttf");

static FONTS: Lazy<fontdb::Database> = Lazy::new(|| {
    let mut fonts = fontdb::Database::new();
    fonts.load_font_data(INTER_FONT.to_vec());
    fonts.set_sans_serif_family("Inter");
    fonts
});

const GRID_STROKE_COLOR: &str = "#e7e7e7"; // colorBase300
const TICK_COLOR: &str = "#a4a4a4"; // colorBase500

// colorSupport*400
const SHAPE_COLORS: &[&str] = &[
    "#c00eae", "#23304a", "#cf3411", "#5f4509", "#1e6378", "#446e02", "#117548", "#943e5c",
    "#661c28", "#802677", "#4f18f4",
];

fn get_shape_list_color<S>(_shape: &S, index: usize) -> &str {
    SHAPE_COLORS[index % SHAPE_COLORS.len()]
}

#[derive(clap::Args, Debug)]
pub struct ChartServiceConfig {
    /// Directory where charts will be stored.
    #[clap(long, env, required = true, help_heading = "Chart storage")]
    storage_dir: PathBuf,
}

#[cfg(test)]
impl ChartServiceConfig {
    pub fn new_test_config() -> Self {
        Self {
            storage_dir: PathBuf::from("/tmp"),
        }
    }
}
pub struct ChartService {
    config: ChartServiceConfig,
}

impl ChartService {
    pub fn new(config: ChartServiceConfig) -> Self {
        Self { config }
    }

    /// FIXME: Generating charts is a relatively heavy task that can potentially
    ///        take a few seconds including rendering. This could block the
    ///        Tokio event loop if multiple charts are being generated
    ///        concurrently. I don't *think* this is an issue currently, since
    ///        the event handler processes events sequentially, and this
    ///        function is only called from the event handler, meaning only one
    ///        chart will be generated at a time.
    ///        However, this still generates a bit of a bottleneck in case
    ///        multiple SLOs are triggering alerts and we would ideally
    ///        parallize chart generation. But if we want to fix that, we should
    ///        still be careful to *only* parallize requests for different
    ///        alerts, and handle everything for the same alert serially, or we
    ///        risk dropping updates to Slack (for instance, when Alertmanager
    ///        quickly resolves an alert we're still generating the chart for).
    pub fn create_chart(
        &self,
        slo: &str,
        time_range: TimeRange,
        timeseries_data: Vec<Timeseries>,
    ) -> Result<Vec<u8>, ChartServiceError> {
        let y_formatter = if slo.starts_with("latency-") {
            FormatterKind::Duration
        } else if slo.starts_with("success-rate-") {
            FormatterKind::Percentage
        } else {
            FormatterKind::Exponent
        };

        let chart = generate(CombinedSourceData {
            graph_type: GraphType::Line,
            stacking_type: StackingType::None,
            timeseries_data: &timeseries_data.iter().collect::<Vec<_>>(),
            events: &[],
            target_latency: None, // Do we want to include the target for latency SLOs?
            time_range,
        })
        .ok_or(ChartServiceError::Generation)?;

        let chart_options = ChartOptions {
            width: 1180,
            height: 400,
            area_gradient_shown: false,
            axis_lines_shown: true,
            grid_columns_shown: true,
            grid_rows_shown: true,
            grid_stroke_color: GRID_STROKE_COLOR,
            grid_stroke_dasharray: Default::default(),
            shape_stroke_width: Some(4.0),
            get_shape_list_color: &get_shape_list_color,
            tick_color: TICK_COLOR,
            x_formatter: Some(FormatterKind::Time),
            y_formatter: Some(y_formatter),
        };

        let image_options = ImageOptions {
            fonts: FONTS.clone(),
            format: ImageFormat::Png,
            background_color: "white".to_owned(),
        };

        let image_data = chart_to_image(&chart, &chart_options, &image_options)
            .map_err(|err| ChartServiceError::Render(err.to_string()))?;

        Ok(image_data)
    }

    #[autometrics]
    #[instrument(err, skip(self))]
    pub async fn create_and_store_chart(
        &self,
        slo: &str,
        time_range: TimeRange,
        timeseries_data: Vec<Timeseries>,
    ) -> Result<String, ChartServiceError> {
        let chart_filename = format!("{ts}-{slo}.png", ts = time_range.to);

        debug!(?chart_filename, "Creating chart");

        let image = Self::create_chart(self, slo, time_range, timeseries_data)?;

        tokio::fs::write(&self.config.storage_dir.join(&chart_filename), image).await?;

        Ok(chart_filename)
    }
}
