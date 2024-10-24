use loyalty_core::ApplicationAdpaters;
use loyalty_core::{
    dd_observability, log_observability, otlp_observability, use_datadog, use_otlp,
};
use opentelemetry_sdk::trace::TracerProvider;
use tracing::info;
use tracing::subscriber::set_global_default;
use tracing::subscriber::SetGlobalDefaultError;

use aws_lambda_events::kafka::KafkaEvent;
use base64::prelude::*;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};

mod adapters;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let adapters = ApplicationAdpaters::new().await;

    let (_, _) = configure_instrumentation();

    run(service_fn(|evt| function_handler(evt, &adapters))).await
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
                        Err(_) => tracing::error!("Failure processing 'OrderConfirmed' event"),
                    }
                }
                Err(_) => {
                    tracing::error!("Failure parsing payload to 'OrderConfirmed' event")
                }
            }
        }
    }

    Ok(())
}
