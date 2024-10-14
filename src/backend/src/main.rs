use adapters::{
    order_confirmed_event_adapter::OrderConfirmedEventAdapter,
    KafkaConnection,
};
use loyalty_core::ApplicationAdpaters;
use ports::OrderConfirmedEventReceiver;
use tokio::signal;
use tracing::info;

mod adapters;
mod ports;

async fn process<T: OrderConfirmedEventReceiver>(receiver: T, topic: &str) {
    info!("Subscribing");

    receiver.subscribe(topic).await;

    loop {
        info!("Receiving");

        receiver.process().await
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let connection = KafkaConnection::new(
        std::env::var("BROKER").unwrap(),
        std::env::var("GROUP_ID").unwrap(),
    );

    let application_adapters = ApplicationAdpaters::new().await;

    let processor = OrderConfirmedEventAdapter::new(connection, application_adapters);

    tokio::spawn(async move {
        process(processor, "order-completed").await;
    });

    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Shutting down");
        }
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
            // we also shut down in case of error
        }
    }

    info!("Shutting down");
}
