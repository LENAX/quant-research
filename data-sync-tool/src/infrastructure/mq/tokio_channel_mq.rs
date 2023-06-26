use std::error::Error;

use async_trait::async_trait;
use tokio::sync::mpsc;
use core::fmt::Debug;

use super::message_bus::MessageBus;

pub struct TokioMpscMessageBus<T: Send + Sync + 'static + Debug> {
    sender: mpsc::Sender<T>,
    receiver: mpsc::Receiver<T>,
}


impl<T: Debug + Send + Sync + 'static> TokioMpscMessageBus<T> {
    pub fn new(size: usize) -> Self {
        let (sender, receiver) = mpsc::channel(size);
        Self { sender, receiver }
    }
}

#[async_trait]
impl<T: Debug + Send + Sync + 'static> MessageBus<T> for TokioMpscMessageBus<T> {
    async fn send(&self, message: T) -> Result<(), Box<dyn Error>> {
        self.sender.send(message).await?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Option<T>, Box<dyn Error>> {
        let received_value = self.receiver.recv().await.ok_or("Failed to receive message")?;
        Ok(Some(received_value))
    }
}