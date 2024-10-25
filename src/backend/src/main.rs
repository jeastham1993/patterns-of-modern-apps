use axum::http::StatusCode;
use axum::Router;
use axum::routing::get;
use loyalty_core::ApplicationAdpaters;
use loyalty_core::{
    dd_observability, log_observability, otlp_observability, use_datadog, use_otlp,
};
use opentelemetry_sdk::trace::TracerProvider;
use tracing::info;
use tracing::subscriber::set_global_default;
use tracing::subscriber::SetGlobalDefaultError;

use adapters::{KafkaConnection, KafkaCredentials};
use tokio::signal;

mod adapters;

async fn process(receiver: &KafkaConnection, topic: &str) {
    info!("Subscribing");

    receiver.subscribe(topic).await;

    loop {
        info!("Receiving");

        receiver.process().await
    }
}

//TODO: Update this implementation to dynamically switch to Lambda based on the Lambda runtime environment variables
#[tokio::main]
async fn main() {
    let (subscriber, provider) = configure_instrumentation();

    let username = std::env::var("KAFKA_USERNAME");
    let password = std::env::var("KAFKA_PASSWORD");

    let credentials = match username {
        Ok(user) => Some(KafkaCredentials {
            username: user,
            password: password.expect("Password to be set if user is"),
        }),
        Err(_) => None,
    };

    let application_adapters = ApplicationAdpaters::new().await;

    let connection = KafkaConnection::new(
        std::env::var("BROKER").unwrap(),
        std::env::var("GROUP_ID").unwrap(),
        credentials,
        application_adapters,
    );

    tokio::spawn(async move {
        process(&connection, "order-completed").await;
    });

    tokio::spawn(async move {
        let app = Router::new()
        .route("/health", get(health));

        let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
    });

    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutting down");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }

    info!("Shutting down");
}

async fn health() -> StatusCode {
    StatusCode::OK
}

fn configure_instrumentation() -> (
    Option<Result<(), SetGlobalDefaultError>>,
    Option<TracerProvider>,
) {
    let service_name = std::env::var("SERVICE_NAME").unwrap_or("loyalty-backend".to_string());

    let mut subscribe: Option<Result<(), SetGlobalDefaultError>> = None;
    let mut provider: Option<TracerProvider> = None;

    if use_otlp() {
        println!("Configuring OTLP");
        let (trace_provider, subscriber) = otlp_observability(&service_name);
        subscribe = Some(set_global_default(subscriber));
        provider = Some(trace_provider)
    } else if use_datadog() {
        println!("Configuring Datadog");
        let (trace_provider, dd_subscriber) = dd_observability();
        subscribe = Some(set_global_default(dd_subscriber));
        provider = Some(trace_provider);
    } else {
        println!("Configuring basic log subscriber");
        let log_subscriber = log_observability(&service_name);
        subscribe = Some(set_global_default(log_subscriber));
    }

    (subscribe, provider)
}
