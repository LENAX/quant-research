use std::error::Error;

use async_trait::async_trait;
use tokio::sync::mpsc;
use core::fmt::Debug;

use super::message_bus::MessageBus;

/// A Message Bus implemented by Tokio Mpsc Channel
pub struct TokioMpscMessageBus<T: Send + Sync + 'static + Debug> {
    sender: mpsc::Sender<T>,
    receiver: mpsc::Receiver<T>,
}

/// FIXME: sharing single receiver and sender may cause a lot of lock contentions
/// Please separate senders and receivers.
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
        println!("Sending message: {:?}", message);
        self.sender.send(message).await?;
        // self.sender.try_send(message)?;
        Ok(())
    }

    async fn receive(&mut self) -> Option<T> {
        // let received_value = self.receiver.recv().await;

        // return received_value;
        let received_value = self.receiver.try_recv();
        match received_value {
            Ok(value) => {
                println!("Received value: {:?}", value);
                Some(value)
            },
            try_recv_error => {
                println!("When trying to receive data, error occurred: {:?}", try_recv_error);
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
    use futures::future::join_all;
    use tokio::sync::Mutex;
    use std::sync::Arc;

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