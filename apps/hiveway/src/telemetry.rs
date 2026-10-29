use opentelemetry::{KeyValue, global, trace::TracerProvider};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_sdk::{
    Resource, logs::SdkLoggerProvider, propagation::TraceContextPropagator,
    trace::SdkTracerProvider,
};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

pub struct Telemetry {
    traces: SdkTracerProvider,
    logs: SdkLoggerProvider,
}

impl Telemetry {
    /// Flush telemetry before the Tokio runtime exits.
    ///
    /// # Errors
    /// Returns an error if an exporter fails to flush.
    pub fn shutdown(self) -> anyhow::Result<()> {
        let traces = self.traces.shutdown();
        let logs = self.logs.shutdown();
        traces?;
        logs?;
        Ok(())
    }
}

/// Configure JSON console logs and OTLP logs and traces for `OpenObserve`.
///
/// # Errors
/// Returns an error if the exporter or tracing subscriber cannot be initialized.
pub fn init() -> anyhow::Result<Telemetry> {
    let resource = Resource::builder()
        .with_service_name("hiveway")
        .with_attributes([
            KeyValue::new("service.namespace", "hive"),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        ])
        .build();
    let traces = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(
            opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .build()?,
        )
        .build();
    let logs = SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(
            opentelemetry_otlp::LogExporter::builder()
                .with_http()
                .build()?,
        )
        .build();
    global::set_text_map_propagator(TraceContextPropagator::new());
    global::set_tracer_provider(traces.clone());
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new("info,hyper=warn,reqwest=warn,opentelemetry=warn")
            }),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_current_span(true),
        )
        .with(tracing_opentelemetry::layer().with_tracer(traces.tracer("hiveway")))
        .with(OpenTelemetryTracingBridge::new(&logs))
        .try_init()?;
    Ok(Telemetry { traces, logs })
}
