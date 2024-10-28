use loyalty_core::{configure_instrumentation, ApplicationAdapters};
use tracing::info;

use aws_lambda_events::kafka::{KafkaEvent, KafkaRecord};
use base64::prelude::*;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};

mod adapters;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let adapters = ApplicationAdapters::new().await;

    let (_, _) = configure_instrumentation();

    run(service_fn(|evt| function_handler(evt, &adapters))).await
}

async fn function_handler(
    event: LambdaEvent<KafkaEvent>,
    adapters: &ApplicationAdapters,
) -> Result<(), Error> {
    for (_, val) in event.payload.records {
        for ele in val {
            process_message(adapters, ele).await
        }
    }

    Ok(())
}

#[tracing::instrument(name = "process_message", skip(adapters, ele))]
async fn process_message(adapters: &ApplicationAdapters, ele: KafkaRecord) {
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
