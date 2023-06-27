use async_trait::async_trait;
// use tokio::sync::mpsc;
use std::error::Error;

#[async_trait]
pub trait MessageBus<T: Send + Sync + 'static> {
    async fn send(&self, message: T) -> Result<(), Box<dyn Error>>;
    async fn receive(&mut self) -> Result<Option<T>, Box<dyn Error>>;
    async fn close(&self);
}
