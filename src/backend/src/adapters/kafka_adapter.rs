use loyalty_core::ApplicationAdapters;
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext};
use rdkafka::message::BorrowedMessage;
use rdkafka::Message;
use tracing::{error, info};

pub struct CustomContext;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {}

type LoggingConsumer = StreamConsumer<CustomContext>;

pub struct KafkaConnection {
    pub consumer: LoggingConsumer,
    adapters: ApplicationAdapters,
}

pub struct KafkaCredentials {
    pub username: String,
    pub password: String,
}

impl KafkaConnection {
    #[tracing::instrument(name = "new_kafka_connection", skip(broker, credentials, adapters))]
    pub fn new(
        broker: String,
        group_id: String,
        credentials: Option<KafkaCredentials>,
        adapters: ApplicationAdapters,
    ) -> KafkaConnection {
        let context = CustomContext;

        let consumer: LoggingConsumer = match credentials {
            Some(creds) => ClientConfig::new()
                .set("group.id", group_id)
                .set("bootstrap.servers", broker)
                .set("security.protocol", "SASL_SSL")
                .set("sasl.mechanisms", "PLAIN")
                .set("sasl.username", creds.username)
                .set("sasl.password", creds.password)
                .set_log_level(RDKafkaLogLevel::Debug)
                .create_with_context(context)
                .expect("Consumer creation failed"),
            None => ClientConfig::new()
                .set("group.id", group_id)
                .set("bootstrap.servers", broker)
                .set_log_level(RDKafkaLogLevel::Debug)
                .create_with_context(context)
                .expect("Consumer creation failed"),
        };

        Self {
            consumer: consumer,
            adapters: adapters,
        }
    }

    pub async fn process(&self) {
        info!("Wait for receive");
        match &self.consumer.recv().await {
            Err(e) => tracing::warn!("Kafka error: {}", e),
            Ok(m) => {
                info!("Received message");
                let _ = &self.process_message(m).await;
            }
        }
    }

    #[tracing::instrument(name = "process_message", skip(self))]
    pub async fn process_message(&self, m: &BorrowedMessage<'_>) {
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
                        self.consumer.commit_message(&m, CommitMode::Async).unwrap();
                    }
                    Err(_) => error!("Failure processing 'OrderConfirmed' event"),
                }
            }
            Err(_) => {
                error!("Failure parsing payload to 'OrderConfirmed' event")
            }
        }
    }

    pub async fn subscribe(&self, message_channel_name: &str) {
        let channels = vec![message_channel_name];
        self.consumer
            .subscribe(&channels)
            .expect("Can't subscribe to specified topics");
    }
}

impl Drop for KafkaConnection {
    fn drop(&mut self) {
        let _ = self.consumer.unassign();
    }
}
