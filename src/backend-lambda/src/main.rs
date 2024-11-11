use loyalty_adapters::{
    configure_instrumentation, ApplicationAdapters, PostgresLoyaltyPoints,
};
use loyalty_core::{LoyaltyPoints, OrderConfirmedEventHandler};
use tracing::info;

use aws_lambda_events::kafka::{KafkaEvent, KafkaRecord};
use base64::prelude::*;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let _ = configure_instrumentation();

    let postgres_loyalty = PostgresLoyaltyPoints::new().await?;

    let adapters = ApplicationAdapters::new(postgres_loyalty).await;

    run(service_fn(|evt| function_handler(evt, &adapters))).await
}

async fn function_handler<T: LoyaltyPoints + Send + Sync>(
    event: LambdaEvent<KafkaEvent>,
    adapters: &ApplicationAdapters<T>,
) -> Result<(), Error> {
    for (_, val) in event.payload.records {
        for record in val {
            let _ = process_message(adapters, record).await;

            // TODO: Implement dead letter handling
            // Current implementation will move forward IF a message can't be processed. There is no way
            // currently with the Kafka<>Lambda integration to batch 'success' messages. If there is an error
            // the problematic message should be moved to a seperate persistence store. SQS?
        }
    }

    Ok(())
}

#[tracing::instrument(name = "process_message", skip(application, record))]
async fn process_message<T: LoyaltyPoints + Send + Sync>(
    application: &ApplicationAdapters<T>,
    record: KafkaRecord,
) -> Result<(), ()> {
    let message_value = match record.value {
        Some(val) => val,
        None => {
            tracing::warn!("Empty message received, skipping");
            return Ok(())
        },
    };

    let decoded = BASE64_STANDARD.decode(message_value).map_err(|e| {
        tracing::error!("Failure decoding message: {}", e);
    })?;

    let evt_payload = serde_json::from_slice(&decoded);

    match evt_payload {
        Ok(evt) => {
            let handle_result =
                OrderConfirmedEventHandler::handle(&application.loyalty_points, &evt).await;

            match handle_result {
                Ok(_) => {
                    info!("Processed successfully");

                    Ok(())
                }
                Err(_) => {
                    tracing::error!("Failure processing 'OrderConfirmed' event");
                    Err(())
                }
            }
        }
        Err(_) => {
            tracing::error!("Failure parsing payload to 'OrderConfirmed' event");
            Err(())
        }
    }
}
