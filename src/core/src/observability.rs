use opentelemetry::{global, trace::TracerProvider as _};
use opentelemetry::KeyValue;
use opentelemetry_datadog::new_pipeline;
use opentelemetry_otlp::{new_exporter, SpanExporterBuilder, WithExportConfig};
use opentelemetry_sdk::{
    runtime::{self, Tokio},
    trace::{Config, TracerProvider},
    Resource,
};
use std::env;
use tracing::{
    level_filters::LevelFilter,
    subscriber::{set_global_default, SetGlobalDefaultError},
    Subscriber,
};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub fn use_datadog() -> bool {
    env::var("DD_SERVICE").is_ok()
}

pub fn use_otlp() -> bool {
    env::var("OTLP_ENDPOINT").is_ok()
}

pub fn dd_observability() -> (TracerProvider, impl Subscriber + Send + Sync) {
    let tracer: opentelemetry_sdk::trace::Tracer = new_pipeline()
        .with_service_name(env::var("DD_SERVICE").expect("DD_SERVICE is not set"))
        .with_agent_endpoint(
            env::var("DD_SERVICE_ENDPOINT").unwrap_or("http://127.0.0.1:8126".to_string()),
        )
        .with_trace_config(
            opentelemetry_sdk::trace::config()
                .with_sampler(opentelemetry_sdk::trace::Sampler::AlwaysOn)
                .with_id_generator(opentelemetry_sdk::trace::RandomIdGenerator::default()),
        )
        .with_api_version(opentelemetry_datadog::ApiVersion::Version05)
        .with_http_client(reqwest::Client::default())
        .install_batch(Tokio)
        .unwrap();

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());
    let logger = tracing_subscriber::fmt::layer().json().flatten_event(true);
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .without_time();

    global::set_tracer_provider(tracer.provider().unwrap());

    (
        tracer.provider().unwrap(),
        Registry::default()
            .with(fmt_layer)
            .with(telemetry_layer)
            .with(logger)
            .with(EnvFilter::from_default_env()),
    )
}

pub fn otlp_observability(service_name: &str) -> (TracerProvider, impl Subscriber + Send + Sync) {
    let tonic_exporter = new_exporter()
        .tonic()
        .with_endpoint(env::var("OTLP_ENDPOINT").unwrap());

    let provider: TracerProvider = TracerProvider::builder()
        .with_config(
            Config::default().with_resource(Resource::new(vec![KeyValue::new(
                opentelemetry_semantic_conventions::resource::SERVICE_NAME.to_string(),
                service_name.to_string(),
            )])),
        )
        .with_batch_exporter(
            SpanExporterBuilder::Tonic(tonic_exporter)
                .build_span_exporter()
                .unwrap(),
            runtime::Tokio,
        )
        .build();
    let tracer = provider.tracer(service_name.to_string());

    global::set_tracer_provider(tracer.provider().unwrap());

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let formatting_layer = BunyanFormattingLayer::new("web".to_string(), std::io::stdout);
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .without_time();

    (
        provider,
        Registry::default()
            .with(env_filter)
            .with(JsonStorageLayer)
            .with(formatting_layer)
            .with(fmt_layer)
            .with(telemetry_layer)
            .with(LevelFilter::DEBUG),
    )
}

pub fn log_observability() -> impl Subscriber + Send + Sync {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let formatting_layer = BunyanFormattingLayer::new("web".to_string(), std::io::stdout);
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .without_time();

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
        .with(fmt_layer)
        .with(LevelFilter::DEBUG)
}

pub fn configure_instrumentation() -> Option<Result<(), SetGlobalDefaultError>> {
    let service_name = std::env::var("SERVICE_NAME").unwrap_or("loyalty-backend".to_string());

    let mut subscribe: Option<Result<(), SetGlobalDefaultError>> = None;

    if use_otlp() {
        println!("Configuring OTLP");
        let (_, subscriber) = otlp_observability(&service_name);
        subscribe = Some(set_global_default(subscriber));
    } else if use_datadog() {
        println!("Configuring Datadog");
        let (_, dd_subscriber) = dd_observability();
        subscribe = Some(set_global_default(dd_subscriber));
    } else {
        println!("Configuring basic log subscriber");
        let _ = log_observability();
    }

    subscribe
}
