use loyalty_core::ApplicationAdpaters;
use loyalty_core::{
    dd_observability, log_observability, otlp_observability, use_datadog, use_otlp,
};
use opentelemetry_sdk::trace::TracerProvider;
use tracing::info;
use tracing::subscriber::set_global_default;
use tracing::subscriber::SetGlobalDefaultError;

#[cfg(not(feature = "lambda"))]
use adapters::{KafkaConnection, KafkaCredentials};

#[cfg(not(feature = "lambda"))]
use tokio::signal;

#[cfg(feature = "lambda")]
use aws_lambda_events::kafka::KafkaEvent;
#[cfg(feature = "lambda")]
use base64::prelude::*;
#[cfg(feature = "lambda")]
use lambda_runtime::{run, service_fn, Error, LambdaEvent};

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
    let adapters = ApplicationAdpaters::new().await;

    let (_, _) = configure_instrumentation();

    run(service_fn(|evt| function_handler(evt, &adapters))).await
}

fn configure_instrumentation() -> (
    Option<Result<(), SetGlobalDefaultError>>,
    Option<TracerProvider>,
) {
    let service_name = "loyalty-backend";

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

#[cfg(feature = "lambda")]
async fn function_handler(
    event: LambdaEvent<KafkaEvent>,
    adapters: &ApplicationAdpaters,
) -> Result<(), Error> {
    for (key, val) in event.payload.records {
        for ele in val {
            let decoded = BASE64_STANDARD.decode(ele.value.unwrap()).unwrap();
            info!(
                "Decoded payload: {}",
                String::from_utf8(decoded.clone()).unwrap()
            );
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
