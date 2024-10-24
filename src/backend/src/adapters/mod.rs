#[cfg(not(feature = "lambda"))]
mod kafka_adapter;

#[cfg(not(feature = "lambda"))]
pub use kafka_adapter::{KafkaConnection, KafkaCredentials};

// #[cfg(feature = "lambda")]
// mod lambda_adapter;