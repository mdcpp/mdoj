use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::{
    metrics::{
        reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
        MeterProvider, PeriodicReader,
    },
    runtime,
    trace::{BatchConfig, RandomIdGenerator, Sampler, Tracer},
    Resource,
};
use opentelemetry_semantic_conventions::{
    resource::{DEPLOYMENT_ENVIRONMENT, SERVICE_NAME, SERVICE_VERSION},
    SCHEMA_URL,
};
use tracing::Level;
use tracing_opentelemetry::{MetricsLayer, OpenTelemetryLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use super::config::GlobalConfig;

fn resource() -> Resource {
    Resource::from_schema_url(
        [
            KeyValue::new(SERVICE_NAME, "mdoj-backend"),
            KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
            #[cfg(debug_assertions)]
            KeyValue::new(DEPLOYMENT_ENVIRONMENT, "develop"),
            #[cfg(not(debug_assertions))]
            KeyValue::new(DEPLOYMENT_ENVIRONMENT, "production"),
        ],
        SCHEMA_URL,
    )
}

fn init_tracer() -> Tracer {
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
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .install_batch(runtime::Tokio)
        .unwrap()
}

fn init_meter_provider() -> MeterProvider {
    #[cfg(debug_assertions)]
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .build_metrics_exporter(
            Box::new(DefaultAggregationSelector::new()),
            Box::new(DefaultTemporalitySelector::new()),
        )
        .unwrap();
    #[cfg(debug_assertions)]
    let otlp_reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(std::time::Duration::from_secs(30))
        .build();

    // For debugging in development
    let stdout_reader = PeriodicReader::builder(
        opentelemetry_stdout::MetricsExporter::default(),
        runtime::Tokio,
    )
    .build();

    let meter_provider = MeterProvider::builder()
        .with_resource(resource())
        .with_reader(stdout_reader);

    #[cfg(debug_assertions)]
    let meter_provider = meter_provider.with_reader(otlp_reader);

    let meter_provider = meter_provider.build();

    global::set_meter_provider(meter_provider.clone());

    meter_provider
}

pub struct OtelGuard {
    meter_provider: MeterProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(err) = self.meter_provider.shutdown() {
            eprintln!("{err:?}");
        }
        opentelemetry::global::shutdown_tracer_provider();
    }
}
fn init_tracing_subscriber(level: Level) -> OtelGuard {
    let meter_provider = init_meter_provider();

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from_level(level))
        .with(tracing_subscriber::fmt::layer())
        .with(MetricsLayer::new(meter_provider.clone()))
        .with(OpenTelemetryLayer::new(init_tracer()))
        .init();

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

    OtelGuard { meter_provider }
}

pub fn init(config: &GlobalConfig) -> OtelGuard {
    let level = match config.log_level {
        0 => Level::TRACE,
        1 => Level::DEBUG,
        2 => Level::INFO,
        3 => Level::WARN,
        4 => Level::ERROR,
        _ => Level::INFO,
    };

    init_tracing_subscriber(level)
}
