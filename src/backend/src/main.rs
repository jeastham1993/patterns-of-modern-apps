use loyalty_core::ApplicationAdpaters;
use tokio::signal;
use tracing::info;

#[cfg(not(feature = "lambda"))]
use adapters::{
    KafkaConnection, KafkaCredentials,
};

#[cfg(feature = "lambda")]
use aws_lambda_events::kafka::KafkaEvent;
#[cfg(feature = "lambda")]
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
#[cfg(feature = "lambda")]
use base64::prelude::*;
#[cfg(feature = "lambda")]
use loyalty_core::observability;
#[cfg(feature = "lambda")]
use tracing_subscriber::util::SubscriberInitExt;

mod adapters;

#[cfg(not(feature = "lambda"))]
async fn process(receiver: &KafkaConnection, topic: &str) {
    info!("Subscribing");

    receiver.subscribe(topic).await;

    loop {
        info!("Receiving");

        receiver.process().await
    }
}

#[tokio::main]
#[cfg(not(feature = "lambda"))]
async fn main() {
    tracing_subscriber::fmt().init();

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
        application_adapters
    );

    tokio::spawn(async move {
        process(&connection, "order-completed").await;
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

#[tokio::main]
#[cfg(feature = "lambda")]
async fn main() -> Result<(), Error> {
    observability().init();

    let adapters = ApplicationAdpaters::new().await;

    run(service_fn(|evt| function_handler(evt, &adapters))).await
}

#[cfg(feature = "lambda")]
async fn function_handler(
    event: LambdaEvent<KafkaEvent>,
    adapters: &ApplicationAdpaters,
) -> Result<(), Error> {
    for (key, val) in event.payload.records {
        for ele in val {
            let decoded = BASE64_STANDARD.decode(ele.value.unwrap()).unwrap();
            info!("Decoded payload: {}", String::from_utf8(decoded.clone()).unwrap());
            let evt_payload = serde_json::from_slice(&decoded);

            match evt_payload {
                Ok(evt) => {
                    let handle_result = adapters.order_confirmed_handler.handle(evt).await;

                    match handle_result {
                        Ok(_) => {
                            info!("Processed successfully")
                        }
                        Err(_) => error!("Failure processing 'OrderConfirmed' event"),
                    }
                }
                Err(_) => {
                    error!("Failure parsing payload to 'OrderConfirmed' event")
                }
            }
        }
    }

    Ok(())
}
