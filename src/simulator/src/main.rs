use rand::Rng;
use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
};
use serde::Serialize;
use std::time::Duration;
use tokio::signal;
use tracing::info;

#[derive(Serialize)]
pub struct OrderConfirmed {
    customer_id: String,
    order_id: String,
    order_value: f32,
}

async fn process(producer: FutureProducer, topic: &str) {
    loop {
        info!("Producing");

        let order_num = rand::thread_rng().gen_range(0..100);

        let order_value_int = rand::thread_rng().gen_range(0..10000);
        let order_value: f32 = order_value_int as f32;

        let data = OrderConfirmed {
            customer_id: "james".to_string(),
            order_id: format!("ORD{}", order_num),
            order_value: order_value / 100.00,
        };

        let serialized = serde_json::to_string(&data).unwrap();

        // The send operation on the topic returns a future, which will be
        // completed once the result or failure from Kafka is received.
        let _ = producer
            .send(
                FutureRecord::to(topic)
                    .payload(&serialized)
                    .key(&data.order_id),
                Duration::from_secs(0),
            )
            .await;

        std::thread::sleep(Duration::from_secs(1));
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", std::env::var("BROKER").unwrap())
        .set("security.protocol", "SASL_SSL")
        .set("sasl.mechanisms", "PLAIN")
        .set("sasl.username", "S3DY2DBTLR4ICJ42")
        .set(
            "sasl.password",
            "vgQU7OkJPJFxdqVykhYWcwz/HpixvQ16pWUdqJcnb8vmwedPN5vYQ+u1xcJrysKH",
        )
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");

    tokio::spawn(async move {
        process(producer, "order-completed").await;
    });

    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutting down");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        }
    }

    info!("Shutting down");
}
