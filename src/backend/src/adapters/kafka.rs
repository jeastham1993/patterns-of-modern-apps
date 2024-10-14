use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{Consumer, ConsumerContext};

pub struct CustomContext;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {}

type LoggingConsumer = StreamConsumer<CustomContext>;

pub struct KafkaConnection {
    pub consumer: LoggingConsumer,
}

impl KafkaConnection {
    pub fn new(broker: String, group_id: String) -> KafkaConnection {
        let context = CustomContext;

        let consumer: LoggingConsumer = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", broker)
            // .set("security.protocol", "SASL_SSL")
            // .set("sasl.mechanisms", "PLAIN")
            // .set("sasl.username", "S3DY2DBTLR4ICJ42")
            // .set("sasl.password", "vgQU7OkJPJFxdqVykhYWcwz/HpixvQ16pWUdqJcnb8vmwedPN5vYQ+u1xcJrysKH")
            .set_log_level(RDKafkaLogLevel::Debug)
            .create_with_context(context)
            .expect("Consumer creation failed");

        Self {
            consumer: consumer
        }
    }
}

impl Drop for KafkaConnection {
    fn drop(&mut self) {
        let _ = self.consumer.unassign();
    }
}