use async_trait::async_trait;
use loyalty_core::ApplicationAdpaters;
use tracing::info;

use crate::ports::OrderConfirmedEventReceiver;

pub struct LambdaOrderConfirmedEventAdapter {
    adapters: ApplicationAdpaters,
}

impl LambdaOrderConfirmedEventAdapter {
    pub fn new(adapters: ApplicationAdpaters) -> Self {
        Self {
            adapters
        }
    }
}

#[async_trait]
impl OrderConfirmedEventReceiver for LambdaOrderConfirmedEventAdapter {
    async fn process(&self) {
        info!("Wait for receive");

        match &self.connection.consumer.recv().await {
            Err(e) => tracing::warn!("Kafka error: {}", e),
            Ok(m) => {
                info!("Received message");

                let payload = match m.payload_view::<str>() {
                    None => "",
                    Some(Ok(s)) => s,
                    Some(Err(e)) => {
                        tracing::warn!("Error while deserializing message payload: {:?}", e);
                        ""
                    }
                };

                let evt_payload = serde_json::from_str(&payload);

                match evt_payload {
                    Ok(evt) => {
                        let handle_result = &self.adapters.order_confirmed_handler.handle(evt).await;

                        match handle_result {
                            Ok(_) => {
                                self.connection
                                    .consumer
                                    .commit_message(&m, CommitMode::Async)
                                    .unwrap();
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
    }

    async fn subscribe(&self, message_channel_name: &str) {
        let channels = vec![message_channel_name];
        self.connection
            .consumer
            .subscribe(&channels)
            .expect("Can't subscribe to specified topics");
    }
}
