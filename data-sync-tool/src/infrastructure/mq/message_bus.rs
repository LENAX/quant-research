use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait MessageBus<T: Send + Sync + 'static> {
    async fn send(&self, message: T) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn receive(&mut self) -> Option<T>;
    async fn close(&mut self);
}
