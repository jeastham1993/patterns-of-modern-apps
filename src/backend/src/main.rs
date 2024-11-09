use axum::http::StatusCode;
use axum::Router;
use axum::routing::get;
use loyalty_core::{configure_instrumentation, ApplicationAdapters, LoyaltyPoints, PostgresLoyaltyPoints};
use tracing::info;

use adapters::{KafkaConnection, KafkaCredentials};
use tokio::signal;

mod adapters;

async fn process<T: LoyaltyPoints + Send + Sync>(receiver: &KafkaConnection<T>, topic: &str) {
    info!("Subscribing");

    receiver.subscribe(topic).await;

    loop {
        info!("Receiving");

        receiver.process().await
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let _ = configure_instrumentation();

    let username = std::env::var("KAFKA_USERNAME");
    let password = std::env::var("KAFKA_PASSWORD");
    let broker = std::env::var("BROKER").expect("'BROKER' environment variable is not set");
    let group_id = std::env::var("GROUP_ID").expect("'GROUP_ID' environment variable is not set");

    let credentials = match username {
        Ok(user) => Some(KafkaCredentials {
            username: user,
            password: password.expect("Password to be set if user is"),
        }),
        Err(_) => None,
    };

    let database = PostgresLoyaltyPoints::new().await?;

    let application_adapters = ApplicationAdapters::new(database).await;

    let connection = KafkaConnection::new(
        broker,
        group_id,
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
    };

    info!("Shutting down");

    Ok(())
}

async fn health() -> StatusCode {
    StatusCode::OK
}