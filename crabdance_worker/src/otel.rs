//! OpenTelemetry / OTLP span export (feature `otel`).
//!
//! This module is compiled only when the `otel` feature is enabled. It wires the
//! native `tracing` spans this SDK already emits to an OTLP collector via
//! `tracing-opentelemetry`, and installs the W3C trace-context propagator as the
//! global OpenTelemetry propagator so `traceparent` headers interoperate across
//! the workflow → activity / child-workflow boundary (the headers carried on the
//! wire by [`W3CTraceContextPropagator`](crabdance_core::W3CTraceContextPropagator)).
//!
//! ```no_run
//! // Behind `--features otel`:
//! # #[cfg(feature = "otel")]
//! # fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! crabdance_worker::otel::init_otlp("http://localhost:4317", "my-worker")?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;

use opentelemetry::propagation::Extractor;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::Resource;
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crabdance_proto::shared::Header;

/// Install a global `tracing` subscriber that exports spans over OTLP (gRPC) to
/// `endpoint` (e.g. `http://localhost:4317`), tagging them with `service_name`,
/// and set the W3C trace-context propagator as the global OpenTelemetry
/// propagator.
///
/// Call once at process start, before constructing the worker. With a
/// `W3CTraceContextPropagator` registered in `WorkerOptions::context_propagators`,
/// the `traceparent` of the active workflow span is injected into outbound
/// activity / child-workflow headers and extracted on the receiving side, so
/// activity spans join the workflow's trace.
pub fn init_otlp(endpoint: &str, service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    let resource = Resource::new(vec![KeyValue::new(
        "service.name",
        service_name.to_string(),
    )]);

    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer(service_name.to_string());
    opentelemetry::global::set_tracer_provider(provider);

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with(otel_layer)
        .try_init()?;

    Ok(())
}

/// View over a Cadence header that decodes byte values as UTF-8 strings for the
/// W3C text-map propagator.
struct HeaderExtractor<'a>(HashMap<&'a str, String>);

impl<'a> HeaderExtractor<'a> {
    fn new(header: &'a Header) -> Self {
        let map = header
            .fields
            .iter()
            .filter_map(|(k, v)| {
                std::str::from_utf8(v)
                    .ok()
                    .map(|s| (k.as_str(), s.to_string()))
            })
            .collect();
        Self(map)
    }
}

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }
    fn keys(&self) -> Vec<&str> {
        self.0.keys().copied().collect()
    }
}

/// Parent `span` to the remote trace carried in `header`'s `traceparent`, so an
/// activity span joins the workflow's trace. No-op when the header is absent or
/// carries no trace context.
pub fn set_remote_parent(span: &tracing::Span, header: Option<&Header>) {
    let Some(header) = header else {
        return;
    };
    let extractor = HeaderExtractor::new(header);
    let parent_cx =
        opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&extractor));
    span.set_parent(parent_cx);
}
