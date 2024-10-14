use async_trait::async_trait;

#[async_trait]
pub trait OrderConfirmedEventReceiver {
    async fn process(&self);
    async fn subscribe(&self, message_channel_name: &str);
}
