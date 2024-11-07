use axum::http::StatusCode;
use axum::Router;
use axum::routing::get;
use loyalty_core::{configure_instrumentation, ApplicationAdapters};
use tracing::info;

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

#[tokio::main]
async fn main() {
    let _ = configure_instrumentation();

    let username = std::env::var("KAFKA_USERNAME");
    let password = std::env::var("KAFKA_PASSWORD");

    let credentials = match username {
        Ok(user) => Some(KafkaCredentials {
            username: user,
            password: password.expect("Password to be set if user is"),
        }),
        Err(_) => None,
    };

    let application_adapters = ApplicationAdapters::new().await;

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