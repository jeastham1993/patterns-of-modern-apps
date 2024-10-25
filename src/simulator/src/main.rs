use rand::Rng;
use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
};
use serde::Serialize;
use uuid::Uuid;
use std::time::Duration;
use tokio::signal;
use tracing::info;

#[derive(Serialize)]
pub struct OrderConfirmed {
    customer_id: String,
    order_id: String,
    order_value: f32,
}

async fn process() {
    let username = std::env::var("KAFKA_USERNAME");
    let password = std::env::var("KAFKA_PASSWORD");
    let broker = std::env::var("BROKER").expect("Broker should be set");

    let producer: FutureProducer = match username {
        Ok(username) => {
            info!("Publishing to {}", &broker);
            ClientConfig::new()
                .set("bootstrap.servers", broker)
                .set("security.protocol", "SASL_SSL")
                .set("sasl.mechanisms", "PLAIN")
                .set("sasl.username", username)
                .set(
                    "sasl.password",
                    password.expect("Password should be set if user is"),
                )
                .create()
                .expect("Producer creation failed")
        }
        Err(_) => ClientConfig::new()
            .set("bootstrap.servers", broker)
            .create()
            .expect("Producer creation failed"),
    };

    loop {
        let customer_under_test = format!("test_{}", Uuid::new_v4());
        info!("Producing for {}", customer_under_test);

        let order_num = rand::thread_rng().gen_range(0..100);

        let order_value_int = rand::thread_rng().gen_range(0..10000);
        let order_value: f32 = order_value_int as f32;

        let data = OrderConfirmed {
            customer_id: customer_under_test,
            order_id: format!("ORD{}", order_num),
            order_value: order_value / 100.00,
        };

        let serialized = serde_json::to_string(&data).unwrap();

        let _ = producer
            .send(
                FutureRecord::to("order-completed")
                    .payload(&serialized)
                    .key(&data.order_id),
                Duration::from_secs(0),
            )
            .await;

        std::thread::sleep(Duration::from_millis(100));
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    tokio::spawn(async move {
        process().await;
    });
    // tokio::spawn(async move {
    //     process().await;
    // });
    // tokio::spawn(async move {
    //     process().await;
    // });
    // tokio::spawn(async move {
    //     process().await;
    // });

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
