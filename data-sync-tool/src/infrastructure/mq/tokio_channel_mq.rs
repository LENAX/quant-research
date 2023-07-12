use std::error::Error;

use async_trait::async_trait;
use futures::channel::mpsc::TryRecvError;
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
    /// TODO: Error Handling
    /// TODO: Graceful shutdown
    async fn send(&self, message: T) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.sender.send(message).await?;
        Ok(())
    }

    async fn receive(&mut self) -> Option<T> {
        // let received_value = self.receiver.recv().await.ok_or("Failed to receive message")?;
        let received_value = self.receiver.try_recv();
        match received_value {
            Ok(value) => {
                Some(value)
            },
            TryRecvError => {
                None
            }
        }
        
    }

    async fn close(&mut self) {
        self.receiver.close();
        self.sender.closed().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::future::{join_all, join};
    use tokio::runtime::Runtime;
    use tokio::sync::Mutex;
    use std::fmt::Debug;
    use std::error::Error;
    use std::sync::Arc;
    use async_trait::async_trait;

    #[tokio::test]
    async fn test_send_receive() {
        let mut message_bus: TokioMpscMessageBus<String> = TokioMpscMessageBus::new(10);

        // Test sending a message
        let send_result = message_bus.send(String::from("test message")).await;
        assert!(send_result.is_ok(), "Failed to send message");

        // Test receiving a message
        let received_message = message_bus.receive().await;
        assert!(received_message.is_none(), "Failed to receive message");
        assert_eq!(received_message, Some(String::from("test message")), "Received unexpected message");
    }

    #[tokio::test]
    async fn test_producer_consumer() {
        let message_bus: Arc<Mutex<TokioMpscMessageBus<String>>> = Arc::new(Mutex::new(TokioMpscMessageBus::new(10)));
        
        let producer_bus = Arc::clone(&message_bus);
        let consumer_bus = Arc::clone(&message_bus);

        let producer = tokio::spawn(async move {
            for i in 0..10 {
                let msg = format!("Message {}", i);
                producer_bus.lock().await.send(msg).await.unwrap();
            }
        });

        let consumer = tokio::spawn(async move {
            for _ in 0..10 {
                let msg = consumer_bus.lock().await.receive().await.unwrap();
                println!("Received: {:?}", msg);
            }
        });

        let _ = tokio::try_join!(producer, consumer);
    }

    #[tokio::test]
    async fn test_multiple_producers_consumers() {
        let message_bus: Arc<Mutex<TokioMpscMessageBus<String>>> = Arc::new(Mutex::new(TokioMpscMessageBus::new(100)));

        let producers: Vec<_> = (0..10).map(|i| {
            let producer_bus = Arc::clone(&message_bus);
            tokio::spawn(async move {
                for j in 0..5 {
                    let msg = format!("Message {} from producer {}", j, i);
                    producer_bus.lock().await.send(msg).await.unwrap();
                }
            })
        }).collect();

        let consumers: Vec<_> = (0..10).map(|_| {
            let consumer_bus = Arc::clone(&message_bus);
            tokio::spawn(async move {
                for _ in 0..5 {
                    let msg = consumer_bus.lock().await.receive().await.unwrap();
                    println!("Received: {:?}", msg);
                }
            })
        }).collect();
        let _ = join_all(producers).await;
        let _ = join_all(consumers).await;

        // let _ = tokio::try_join!(join_all(producers), join_all(consumers));
    }
}