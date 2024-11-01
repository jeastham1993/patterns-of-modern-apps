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

async fn product_order_completed_message(customer_ids: Vec<String>) {
    let username = std::env::var("KAFKA_USERNAME");
    let password = std::env::var("KAFKA_PASSWORD");
    let broker = std::env::var("BROKER").expect("Broker should be set");

    let producer: FutureProducer = match username {
        Ok(username) => {
            // info!("Publishing to {}", &broker);
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
        let customer_id_for_call = rand::thread_rng().gen_range(1..customer_ids.len());
        let customer = &customer_ids[customer_id_for_call - 1];
        // info!("Producing for {}", customer);

        let order_num = rand::thread_rng().gen_range(0..100);

        let order_value_int = rand::thread_rng().gen_range(0..10000);
        let order_value: f32 = order_value_int as f32;

        let data = OrderConfirmed {
            customer_id: customer.clone(),
            order_id: format!("ORD{}", order_num),
            order_value: order_value / 100.00,
        };

        let serialized = serde_json::to_string(&data).unwrap();

        let _ = producer
            .send(
                FutureRecord::to("order-completed")
                    .payload(&serialized)
                    .key(customer),
                Duration::from_secs(0),
            )
            .await;

        std::thread::sleep(Duration::from_millis(100));
    }
}

async fn get_loyalty_points(api_endpoint: &str, customer_ids: Vec<String>) {
    let client = reqwest::Client::default();

    loop {
        let customer_id_for_call = rand::thread_rng().gen_range(1..customer_ids.len());
        let customer = &customer_ids[customer_id_for_call - 1];

        let calling_api_endpoint = format!("{}/loyalty/{}", api_endpoint, customer);

        let res = client.get(calling_api_endpoint).send().await;

        // tracing::info!("Get result for {} is ok: {:?}", customer, res.is_ok());

        std::thread::sleep(Duration::from_millis(500));
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let lambda_api_endpoint = std::env::var("LAMBDA_API_ENDPOINT");
    let fargate_api_endpoint = std::env::var("FARGATE_API_ENDPOINT");
    let aca_api_endpoint = std::env::var("ACA_API_ENDPOINT");
    let gcp_api_endpoint = std::env::var("GCP_API_ENDPOINT");
    let kafka_broker = std::env::var("BROKER");

    match lambda_api_endpoint {
        Ok(lambda_api_endpoint) => {
            tokio::spawn(async move {
                get_loyalty_points(&lambda_api_endpoint, get_customer_list()).await;
                get_loyalty_points(&lambda_api_endpoint, get_customer_list()).await;
            });
        }
        Err(_) => {
            info!("Lambda endpoint not configured")
        }
    }

    match fargate_api_endpoint {
        Ok(fargate_api_endpoint) => {
            tokio::spawn(async move {
                get_loyalty_points(&fargate_api_endpoint, get_customer_list()).await;
            });
        }
        Err(_) => {
            info!("Fargate endpoint not configured")
        }
    }

    match aca_api_endpoint {
        Ok(aca_api_endpoint) => {
            tokio::spawn(async move {
                get_loyalty_points(&aca_api_endpoint, get_customer_list()).await;
            });
        }
        Err(_) => {
            info!("ACA endpoint not configured")
        }
    }

    match gcp_api_endpoint {
        Ok(gcp_api_endpoint) => {
            tokio::spawn(async move {
                get_loyalty_points(&gcp_api_endpoint, get_customer_list()).await;
            });
        }
        Err(_) => {
            info!("Google Cloud Run endpoint not configured")
        }
    }

    match kafka_broker {
        Ok(_) => {
            tokio::spawn(async move {
                product_order_completed_message(get_customer_list()).await;
            });
        }
        Err(_) => {
            info!("Broker not configured, events will not be published")
        }
    }

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

fn get_customer_list() -> Vec<String> {
    vec![
        "james".to_string(),
        "christina".to_string(),
        "mark".to_string(),
        "ruben".to_string(),
        "rubi".to_string(),
        "nell".to_string(),
        "john".to_string(),
        "jane".to_string(),
        "janice".to_string(),
        "paul".to_string(),
        "chris".to_string(),
        "scott".to_string(),
        "luca".to_string(),
        "roger".to_string(),
        "florence".to_string(),
    ]
}
