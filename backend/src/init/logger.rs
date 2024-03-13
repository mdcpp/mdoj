use opentelemetry::global;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::reader::MetricReader;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::{
    metrics::{
        reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
        MeterProvider, PeriodicReader,
    },
    runtime,
    trace::{BatchConfig, RandomIdGenerator, Sampler, Tracer},
};
use opentelemetry_semantic_conventions::{
    resource::{DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use tracing::Level;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::init::config::GlobalConfig;

pub static PACKAGE_NAME: &str = "mdoj-backend";

fn resource() -> Resource {
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, PACKAGE_NAME),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            #[cfg(debug_assertions)]
            KeyValue::new(DEPLOYMENT_ENVIRONMENT, "development"),
            #[cfg(not(debug_assertions))]
            KeyValue::new(DEPLOYMENT_ENVIRONMENT, "production"),
        ],
        SCHEMA_URL,
    )
}

// Construct MeterProvider for MetricsLayer
fn init_meter_provider(reader: impl MetricReader) -> MeterProvider {
    let meter_provider = MeterProvider::builder()
        .with_resource(resource())
        .with_reader(reader)
        .build();

    global::set_meter_provider(meter_provider.clone());

    meter_provider
}

// Construct Tracer for OpenTelemetryLayer
fn init_tracer(endpoint: &str) -> super::Result<Tracer> {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                // Customize sampling strategy
                .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                    1.0,
                ))))
                // If export trace to AWS X-Ray, you can use XrayIdGenerator
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource()),
        )
        .with_batch_config(BatchConfig::default())
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .install_batch(runtime::Tokio)
        .map_err(|err| err.into())
}

// Initialize tracing-subscriber and return OtelGuard for opentelemetry-related termination processing
fn init_tracing_subscriber(level: Level, opentelemetry: Option<&str>) -> super::Result<OtelGuard> {
    let meter_provider = init_meter_provider(match opentelemetry {
        Some(_) => {
            let exporter = opentelemetry_otlp::new_exporter()
                .tonic()
                .build_metrics_exporter(
                    Box::new(DefaultAggregationSelector::new()),
                    Box::new(DefaultTemporalitySelector::new()),
                )
                .unwrap();
            PeriodicReader::builder(exporter, runtime::Tokio)
                .with_interval(std::time::Duration::from_secs(30))
                .build()
        }
        None => PeriodicReader::builder(
            opentelemetry_stdout::MetricsExporter::default(),
            runtime::Tokio,
        )
        .build(),
    });

    match opentelemetry {
        Some(endpoint) => tracing_subscriber::registry()
            .with(tracing_subscriber::filter::LevelFilter::from_level(level))
            .with(tracing_subscriber::fmt::layer())
            .with(MetricsLayer::new(meter_provider.clone()))
            .with(OpenTelemetryLayer::new(init_tracer(endpoint)?))
            .init(),
        None => tracing_subscriber::registry()
            .with(tracing_subscriber::filter::LevelFilter::from_level(level))
            .with(tracing_subscriber::fmt::layer())
            .with(MetricsLayer::new(meter_provider.clone()))
            .init(),
    };

    Ok(OtelGuard { meter_provider })
}

pub struct OtelGuard {
    pub meter_provider: MeterProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(err) = self.meter_provider.shutdown() {
            eprintln!("{err:?}");
        }
        opentelemetry::global::shutdown_tracer_provider();
    }
}

fn init_panic_hook() {
    std::panic::set_hook(Box::new(|panic| {
        if let Some(location) = panic.location() {
            tracing::error!(
                message = %panic,
                panic.file = location.file(),
                panic.line = location.line(),
                panic.column = location.column(),
            );
        } else {
            tracing::error!(message = %panic);
        }
    }));
}

pub fn init(config: &GlobalConfig) -> super::Result<OtelGuard> {
    init_panic_hook();

    let level = match config.log_level {
        0 => Level::TRACE,
        1 => Level::DEBUG,
        2 => Level::INFO,
        3 => Level::WARN,
        4 => Level::ERROR,
        _ => Level::INFO,
    };

    init_tracing_subscriber(level, config.opentelemetry.as_ref().map(|x| x.as_str()))
}
