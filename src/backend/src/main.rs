use clap::{App, Arg};

use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext};
use rdkafka::message::{Headers, Message};
use rdkafka::util::get_rdkafka_version;
use tracing::info;

// A context can be used to change the behavior of producers and consumers by adding callbacks
// that will be executed by librdkafka.
// This particular context sets up custom callbacks to log rebalancing events.
struct CustomContext;

impl ClientContext for CustomContext {}

impl ConsumerContext for CustomContext {}

// A type alias with your custom consumer can be created for convenience.
type LoggingConsumer = StreamConsumer<CustomContext>;

async fn consume_and_print(topics: &[&str]) {
    let context = CustomContext;

    info!("Starting consumption");

    let consumer: LoggingConsumer = ClientConfig::new()
        .set("group.id", "rust-kafka-backend")
        .set("bootstrap.servers", "pkc-41mxj.uksouth.azure.confluent.cloud:9092")
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("security.protocol", "SASL_SSL")
        .set("sasl.mechanisms", "PLAIN")
        .set("sasl.username", "S3DY2DBTLR4ICJ42")
        .set("sasl.password", "vgQU7OkJPJFxdqVykhYWcwz/HpixvQ16pWUdqJcnb8vmwedPN5vYQ+u1xcJrysKH")
        .set("session.timeout.ms", "45000")
        //.set("statistics.interval.ms", "30000")
        //.set("auto.offset.reset", "smallest")
        .set_log_level(RDKafkaLogLevel::Debug)
        .create_with_context(context)
        .expect("Consumer creation failed");

        info!("Subscribing");

    consumer
        .subscribe(&topics.to_vec())
        .expect("Can't subscribe to specified topics");

    loop {
        info!("Receiving");

        match consumer.recv().await {
            Err(e) => tracing::warn!("Kafka error: {}", e),
            Ok(m) => {
                let payload = match m.payload_view::<str>() {
                    None => "",
                    Some(Ok(s)) => s,
                    Some(Err(e)) => {
                        tracing::warn!("Error while deserializing message payload: {:?}", e);
                        ""
                    }
                };
                tracing::info!("key: '{:?}', payload: '{}', topic: {}, partition: {}, offset: {}, timestamp: {:?}",
                      m.key(), payload, m.topic(), m.partition(), m.offset(), m.timestamp());
                if let Some(headers) = m.headers() {
                    for header in headers.iter() {
                        tracing::info!("  Header {:#?}: {:?}", header.key, header.value);
                    }
                }
                consumer.commit_message(&m, CommitMode::Async).unwrap();
            }
        };
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();
    
    let (version_n, version_s) = get_rdkafka_version();
    tracing::info!("rd_kafka_version: 0x{:08x}, {}", version_n, version_s);

    consume_and_print(&["rust-kafka"]).await
}