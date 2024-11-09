use std::time::Duration;

use chrono::{DateTime, Utc};
use rand::Rng;
use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[derive(Serialize)]
pub struct OrderConfirmed {
    customer_id: String,
    order_id: String,
    order_value: f32,
}

#[derive(Deserialize)]
pub struct LoyaltyDto {
    pub customer_id: String,
    pub current_points: f32,
    pub transactions: Vec<LoyaltyAccountTransaction>,
}

#[derive(Deserialize, Clone)]
#[allow(dead_code)]
pub struct LoyaltyAccountTransaction {
    date: DateTime<Utc>,
    order_number: String,
    change: f32,
}

#[tokio::test]
async fn when_event_published_points_are_added() {
    tracing_subscriber::fmt().json().init();

    let customer_under_test = format!("test_{}", Uuid::new_v4());

    info!("Customer under test is {}", &customer_under_test);

    produce_event(&customer_under_test, 100.00).await;

    std::thread::sleep(Duration::from_secs(5));

    let api_endpoint = std::env::var("API_ENDPOINT").unwrap_or("http://localhost:8080".to_string());

    info!("Querying {}", api_endpoint);

    let client = reqwest::Client::new();

    let get_points = client
        .get(format!("{}/loyalty/{}", api_endpoint, customer_under_test))
        .send()
        .await
        .expect("Loyalty Account should exist");
    let body = get_points.text().await.unwrap();
    info!(body);
    let account = serde_json::from_str::<LoyaltyDto>(&body).unwrap();

    assert_eq!(account.customer_id, customer_under_test);
    assert!(account.current_points > 0.0);
    assert_eq!(account.transactions.len(), 1);

    let spend_points = client
        .post(format!("{}/loyalty/{}/spend", api_endpoint, customer_under_test))
        .header("Content-Type", "application/json")
        .body(serde_json::json!({"customerId": customer_under_test, "orderNumber": "ORD999", "spend": 5}).to_string())
        .send()
        .await
        .expect("Spend points should be successful");

    assert_eq!(spend_points.status(), 200);

    let spend_body = spend_points.text().await.unwrap();
    let account_after_spend = serde_json::from_str::<LoyaltyDto>(&spend_body).unwrap();

    assert_eq!(account_after_spend.customer_id, customer_under_test);
    assert!(
        account_after_spend.current_points < account.current_points
    );
    assert_eq!(account_after_spend.transactions.len(), 2);
}

async fn produce_event(customer_under_test: &str, order_value: f32) {
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

    info!("Publishing message");

    let order_num = rand::thread_rng().gen_range(0..100);

    let data = OrderConfirmed {
        customer_id: customer_under_test.to_string(),
        order_id: format!("ORD{}", order_num),
        order_value,
    };

    let serialized = serde_json::to_string(&data).unwrap();

    let res = producer
        .send(
            FutureRecord::to("order-completed")
                .payload(&serialized)
                .key(&data.order_id),
            Duration::from_secs(0),
        )
        .await;

    match res {
        Ok(_) => info!("Publish success"),
        Err((e, _)) => tracing::error!("Kafka publish failed: {}", e),
    }
}
