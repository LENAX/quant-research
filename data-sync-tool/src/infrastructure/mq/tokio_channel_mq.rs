use std::error::Error;

use async_trait::async_trait;
use derivative::Derivative;
use tokio::sync::{mpsc, broadcast, watch};
use core::fmt::Debug;

use super::message_bus::{MessageBus, MessageBusReceiver, MessageBusError, MessageBusSender};

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


pub fn create_tokio_mpsc_channel<T>(n: usize) -> (TokioMpscMessageBusSender<T>, TokioMpscMessageBusReceiver<T>){
    let (tx, mut rx) = mpsc::channel::<T>(100);
    let sender = TokioMpscMessageBusSender{ sender: tx };
    let receiver = TokioMpscMessageBusReceiver { receiver: rx };

    return (sender, receiver)
}

pub fn create_tokio_spmc_channel<T: std::clone::Clone>(n: usize) -> (TokioSpmcMessageBusSender<T>, TokioSpmcMessageBusReceiver<T>){
    let (tx, mut rx) = broadcast::channel::<T>(n);
    let sender = TokioSpmcMessageBusSender{ sender: tx, closed: false };
    let receiver = TokioSpmcMessageBusReceiver { receiver: rx, closed: false };

    return (sender, receiver)
}

#[async_trait]
impl<T: Debug + Send + Sync + 'static> MessageBus<T> for TokioMpscMessageBus<T> {
    /// TODO: Error Handling
    /// TODO: Graceful shutdown
    async fn send(&self, message: T) -> Result<(), MessageBusError<T>> {
        println!("Sending message: {:?}", message);
        let r = self.sender.send(message).await;
        if let Err(e) = r {
            return Err(MessageBusError::SendFailed(e.0));
        }
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

#[derive(Debug)]
pub struct TokioMpscMessageBusReceiver<T> {
    receiver: mpsc::Receiver<T>,
}

#[async_trait]
impl<T: std::marker::Send> MessageBusReceiver<T> for TokioMpscMessageBusReceiver<T> {
    async fn receive(&mut self) -> Option<T> {
        let data = self.receiver.recv().await;
        return data;
    }

    fn try_recv(&mut self) -> Result<T, MessageBusError<T>> {
        let data = self.receiver.try_recv();

        match data {
            Ok(d) => {
                Ok(d)
            },
            Err(e) => {
                Err(MessageBusError::ReceiveFailed(e.to_string()))
            }
        }
    }

    fn close(&mut self) {
        self.receiver.close();
    }
}

#[derive(Clone, Debug)]
pub struct TokioMpscMessageBusSender<T> {
    sender: mpsc::Sender<T>,
}

#[async_trait]
impl<T> MessageBusSender<T> for TokioMpscMessageBusSender<T> 
where T: std::marker::Send + std::convert::From<tokio::sync::mpsc::error::TrySendError<T>>
{
    async fn send(&self, message: T) -> Result<(), MessageBusError<T>> {
        let result = self.sender.send(message).await;
        if let Err(e) = result {
            return Err(MessageBusError::SendFailed(e.0))
        }
        Ok(())
    }

    fn try_send(&self, message: T) -> Result<(), MessageBusError<T>> {
        let result = self.sender.try_send(message);
        if let Err(e) = result {
            return Err(MessageBusError::SendFailed(e.into()));
        }
        Ok(())
    }

    async fn close(&self) {
        self.sender.closed().await;
    }

    fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

#[derive(Debug)]
pub struct TokioSpmcMessageBusSender<T> {
    sender: broadcast::Sender<T>,
    closed: bool
}


#[async_trait]
impl<T> MessageBusSender<T> for TokioSpmcMessageBusSender<T>
where T: std::marker::Send + std::convert::From<tokio::sync::mpsc::error::TrySendError<T>>
{
    async fn send(&self, message: T) -> Result<(), MessageBusError<T>> {
        let result = self.sender.send(message);
        if let Err(e) = result {
            return Err(MessageBusError::SendFailed(e.0))
        }
        Ok(())
    }

    fn try_send(&self, message: T) -> Result<(), MessageBusError<T>> {
        let result = self.sender.send(message);
        if let Err(e) = result {
            return Err(MessageBusError::SendFailed(e.0))
        }
        Ok(())
    }
    
    async fn close(&self) {
        drop(self.sender);
        self.closed = true;
    }
    
    fn is_closed(&self) -> bool {
        self.closed
    }
    
}

#[derive(Debug)]
pub struct TokioSpmcMessageBusReceiver<T> {
    // use broadcast channel to leverage its buffering ability
    receiver: broadcast::Receiver<T>,
    closed: bool
}

#[async_trait]
impl<T: Clone + std::marker::Send> MessageBusReceiver<T> for TokioSpmcMessageBusReceiver<T> {
    async fn receive(&mut self) -> Option<T> {
        let r = self.receiver.recv().await;
        match r {
            Ok(data) => { Some(data) },
            Err(e) => { None }
        }
    }
    
    fn try_recv(&mut self) -> Result<T, MessageBusError<T>> {
        let data = self.receiver.try_recv();

        match data {
            Ok(d) => {
                Ok(d)
            },
            Err(e) => {
                Err(MessageBusError::ReceiveFailed(e.to_string()))
            }
        }
    }
    
    fn close(&mut self) {
        drop(self.receiver);
        self.closed = true;
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