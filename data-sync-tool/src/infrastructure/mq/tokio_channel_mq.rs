use async_trait::async_trait;
use log::{error, info, warn};
use tokio::sync::{mpsc::{self, error::TrySendError}, broadcast, watch, Mutex};
use uuid::Uuid;
use core::fmt::Debug;
use getset::{Getters, Setters, MutGetters};

use super::message_bus::{MessageBus, MessageBusReceiver, MessageBusError, MessageBusSender, MpscMessageBus, SpmcMessageBus, MessageBusFailureCause, StaticClonableMpscMQ, StaticClonableAsyncComponent, StaticMpscMQReceiver};


pub fn create_tokio_mpsc_channel<T>(
    n: usize
) -> (TokioMpscMessageBusSender<T>, TokioMpscMessageBusReceiver<T>){
    let (tx, mut rx) = mpsc::channel::<T>(n);
    let channel_id = Uuid::new_v4();
    let sender = TokioMpscMessageBusSender::new(channel_id, tx);
    let receiver = TokioMpscMessageBusReceiver::new(channel_id, rx);

    return (sender, receiver)
}

pub fn create_tokio_spmc_channel<T: std::clone::Clone>(
    n: usize
) -> (TokioSpmcMessageBusSender<T>, TokioSpmcMessageBusReceiver<T>){
    let (tx, mut rx) = broadcast::channel::<T>(n);
    let channel_id = Uuid::new_v4();
    let sender = TokioSpmcMessageBusSender::new(channel_id, tx);
    let receiver = TokioSpmcMessageBusReceiver::new(channel_id, rx);

    return (sender, receiver)
}

pub fn create_tokio_broadcasting_channel<T: std::clone::Clone>(
    n: usize
) -> (TokioBroadcastingMessageBusSender<T>, TokioBroadcastingMessageBusReceiver<T>){
    let (tx, mut rx) = broadcast::channel::<T>(n);
    let channel_id = Uuid::new_v4();
    let sender = TokioBroadcastingMessageBusSender::new(channel_id, tx);
    let receiver = TokioBroadcastingMessageBusReceiver::new(channel_id, rx);

    return (sender, receiver)
}

// TokioMpscMessageBusSender
#[derive(Clone, Debug, Getters, Setters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct TokioMpscMessageBusSender<T> {
    channel_id: Uuid,
    sender: mpsc::Sender<T>,
}

impl<T> TokioMpscMessageBusSender<T> {
    pub fn new(channel_id: Uuid, sender: mpsc::Sender<T>) -> Self {
        Self { channel_id, sender }
    }
}

impl<T: std::marker::Send + std::marker::Sync + std::fmt::Debug> MpscMessageBus for TokioMpscMessageBusSender<T> {}
impl<T: std::clone::Clone + std::marker::Send + std::marker::Sync + std::fmt::Debug +'static> StaticClonableMpscMQ for TokioMpscMessageBusSender<T> {}

#[async_trait]
impl<T: std::marker::Send> MessageBusSender<T> for TokioMpscMessageBusSender<T> {
    async fn send(&self, message: T) -> Result<(), MessageBusError<T>> {
        let result = self.sender.send(message).await;
        if let Err(e) = result {
            return Err(MessageBusError::SendFailed(e.0, MessageBusFailureCause::Unknown))
        }
        Ok(())
    }

    fn try_send(&self, message: T) -> Result<(), MessageBusError<T>> {
        let result = self.sender.try_send(message);
        if let Err(e) = result {
            match e {
                TrySendError::Full(t) => {
                    return Err(MessageBusError::SendFailed(t, MessageBusFailureCause::Full));
                },
                TrySendError::Closed(t) => {
                    return Err(MessageBusError::SendFailed(t, MessageBusFailureCause::Closed));
                }
            }
        }
        Ok(())
    }

    async fn close(&mut self) {
        self.sender.closed().await;
    }

    fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

#[derive(Debug, Getters, Setters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct TokioMpscMessageBusReceiver<T> {
    channel_id: Uuid,
    receiver: mpsc::Receiver<T>,
}

impl<T> TokioMpscMessageBusReceiver<T> {
    pub fn new(channel_id: Uuid, receiver: mpsc::Receiver<T>) -> Self {
        Self { channel_id, receiver }
    }
}

impl<T: std::marker::Send + std::marker::Sync + std::fmt::Debug> MpscMessageBus for TokioMpscMessageBusReceiver<T> {}
impl<T> StaticMpscMQReceiver for TokioMpscMessageBusReceiver<T>
where T: std::marker::Send + std::marker::Sync + std::fmt::Debug +'static {}

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


#[derive(Debug, Getters, Setters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct TokioSpmcMessageBusSender<T> {
    channel_id: Uuid,
    sender: Option<broadcast::Sender<T>>,
    closed: bool
}

impl<T> TokioSpmcMessageBusSender<T> {
    pub fn new(channel_id: Uuid, sender: broadcast::Sender<T>) -> Self {
        Self { channel_id, sender: Some(sender), closed: false }
    }

    fn inner_send(&self, message: T) -> Result<(), MessageBusError<T>> {
        if self.closed {
            return Err(MessageBusError::SenderClosed);
        }

        match self.sender() {
            None => {
                return Err(MessageBusError::SenderClosed);
            },
            Some(inner_sender) => {
                let r = inner_sender.send(message);
                match r {
                    Ok(n_receiver) => {
                        info!("{} receivers are listening to channel {}", n_receiver, self.channel_id());
                        info!("message sent!");
                        return Ok(());
                    },
                    Err(e) => {
                        error!("send failed in channel {}, reason: {}", self.channel_id(), e);
                        return Err(MessageBusError::SendFailed(e.0, MessageBusFailureCause::Unknown));
                    }
                }
            }
        }
    }
}

impl<T: std::clone::Clone + std::marker::Send + 'static> SpmcMessageBus<T> for TokioSpmcMessageBusSender<T> {
    fn sub(&self) -> Result<Box<dyn MessageBusReceiver<T>>, MessageBusError<T>> {
        match self.sender() {
            Some(inner_sender) => {
                let new_receiver = TokioSpmcMessageBusReceiver::new(
                    self.channel_id, inner_sender.subscribe());
                Ok(Box::new(new_receiver))
            },
            None => {
                Err(MessageBusError::SenderClosed)
            }
        }
    }

    fn receiver_count(&self) -> Result<usize, MessageBusError<T>> {
        if let Some(inner_sender) = self.sender() {
            Ok(inner_sender.receiver_count())
        } else {
            Err(MessageBusError::SenderClosed)
        }
    }

    fn same_channel(&self, other: &Self) -> Result<bool, MessageBusError<T>> {
        // self.sender.same_channel(&other.sender)
        if let Some(inner_sender) = self.sender() {
            if other.is_closed() {
                return Ok(false);
            }
            if let Some(other_sender_inner) = other.sender() {
                return Ok(inner_sender.same_channel(other_sender_inner));
            } else {
                return Ok(false);
            }
        } else {
            Err(MessageBusError::SenderClosed)
        }
    }
}

#[async_trait]
impl<T> MessageBusSender<T> for TokioSpmcMessageBusSender<T>
where T: std::marker::Send
{
    async fn send(&self, message: T) -> Result<(), MessageBusError<T>> {
        let result = self.inner_send(message);
        match result {
            Ok(()) => { return Ok(()) },
            Err(e) => { Err(e) }
        }
    }

    fn try_send(&self, message: T) -> Result<(), MessageBusError<T>> {
        let result = self.inner_send(message);
        match result {
            Ok(()) => { return Ok(()) },
            Err(e) => { Err(e) }
        }
    }
    
    async fn close(&mut self) {
        self.sender = None;
        self.closed = true;
    }
    
    fn is_closed(&self) -> bool {
        self.closed && self.sender.is_none()
    }
}

#[derive(Debug, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct TokioSpmcMessageBusReceiver<T> {
    // use broadcast channel to leverage its buffering ability
    id: Uuid,
    channel_id: Uuid,
    receiver: Option<broadcast::Receiver<T>>,
    closed: bool
}

impl<T: Clone> TokioSpmcMessageBusReceiver<T> {
    pub fn new(channel_id: Uuid, receiver: broadcast::Receiver<T>) -> Self {
        Self {id: Uuid::new_v4(),channel_id, receiver: Some(receiver), closed: false }
    }

    fn inner(&mut self) -> Result<&mut broadcast::Receiver<T>, MessageBusError<T>>  {
        if self.closed {
            return Err(MessageBusError::ReceiverClosed);
        }
        
        let (receiver_id, channel_id) = (self.id.clone(), self.channel_id.clone());
        match self.receiver_mut() {
            Some(inner_receiver) => {
                return Ok(inner_receiver);
            },
            None => {
                error!("Attempt to call recv on closed receiver {} of channel id {}",
                        receiver_id, channel_id);
                return Err(MessageBusError::ReceiverClosed);
            }
        }
    }

    async fn inner_receive(&mut self) -> Result<T, MessageBusError<T>> {
        let inner_receiver = self.inner()?;
        let r = inner_receiver.recv().await;
        match r {
            Ok(data) => { return Ok(data); },
            Err(e) => { return Err(MessageBusError::ReceiveFailed(e.to_string())); }
        }
    }
}

impl<T: std::clone::Clone + std::marker::Send + std::marker::Sync + std::fmt::Debug +'static> StaticClonableAsyncComponent
for TokioSpmcMessageBusReceiver<T> {}

impl<T: Clone> Clone for TokioSpmcMessageBusReceiver<T> {
    fn clone(&self) -> TokioSpmcMessageBusReceiver<T> {
        match self.receiver() {
            Some(inner_receiver) => {
                return TokioSpmcMessageBusReceiver::new(self.channel_id, inner_receiver.resubscribe());
            },
            None => {
                warn!("Attempt to clone a closed receiver {} of channel id {}", self.id(), self.channel_id());
                return TokioSpmcMessageBusReceiver { 
                    id: Uuid::new_v4(), channel_id: self.channel_id, receiver: None, closed: true };
            }
        }
    }
}

#[async_trait]
impl<T: Clone + std::marker::Send> MessageBusReceiver<T> for TokioSpmcMessageBusReceiver<T> {
    async fn receive(&mut self) -> Option<T> {
        let r = self.inner_receive().await;
        match r {
            Ok(data) => { Some(data) },
            Err(e) => { 
                error!("receive error: {}", e);
                None 
            }
        }
    }
    
    fn try_recv(&mut self) -> Result<T, MessageBusError<T>> {
        let data = self.inner()?.try_recv();
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
        self.receiver = None;
        self.closed = true;
        info!("Receiver")
    }
}


#[derive(Debug, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct TokioBroadcastingMessageBusSender<T> {
    id: Uuid,
    channel_id: Uuid,
    sender: Option<broadcast::Sender<T>>,
    closed: bool
}

impl<T> TokioBroadcastingMessageBusSender<T> {
    pub fn new(channel_id: Uuid, sender: broadcast::Sender<T>) -> Self {
        Self { id: Uuid::new_v4(), channel_id, sender: Some(sender), closed: false }
    }

    fn inner_sender(&self) -> Result<&broadcast::Sender<T>, MessageBusError<T>>  {
        if self.closed {
            return Err(MessageBusError::ReceiverClosed);
        }

        match self.sender() {
            Some(_inner_sender) => {
                return Ok(_inner_sender);
            },
            None => {
                error!("Attempt to call recv on closed receiver {} of channel id {}",
                       self.id(), self.channel_id());
                return Err(MessageBusError::ReceiverClosed);
            }
        }
    }
}

#[async_trait]
impl<T> MessageBusSender<T> for TokioBroadcastingMessageBusSender<T>
where T: std::marker::Send
{
    async fn send(&self, message: T) -> Result<(), MessageBusError<T>> {
        let result = self.inner_sender()?.send(message);
        if let Err(e) = result {
            return Err(MessageBusError::SendFailed(e.0, MessageBusFailureCause::Unknown))
        }
        Ok(())
    }

    fn try_send(&self, message: T) -> Result<(), MessageBusError<T>> {
        let result = self.inner_sender()?.send(message);
        if let Err(e) = result {
            return Err(MessageBusError::SendFailed(e.0, MessageBusFailureCause::Unknown))
        }
        Ok(())
    }
    
    async fn close(&mut self) {
        self.sender = None;
        self.closed = true;
    }
    
    fn is_closed(&self) -> bool {
        self.closed
    }
}


#[derive(Debug, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct TokioBroadcastingMessageBusReceiver<T> {
    id: Uuid,
    channel_id: Uuid,
    receiver: Option<broadcast::Receiver<T>>,
    closed: bool
}

impl<T: Clone> TokioBroadcastingMessageBusReceiver<T> {
    pub fn new(channel_id: Uuid, receiver: broadcast::Receiver<T>) -> Self {
        Self { id: Uuid::new_v4(), channel_id, receiver: Some(receiver), closed: false }
    }

    fn inner_receiver(&mut self) -> Result<&mut broadcast::Receiver<T>, MessageBusError<T>>  {
        if self.closed {
            return Err(MessageBusError::ReceiverClosed);
        }

        let (receiver_id, channel_id) = (self.id.clone(), self.channel_id.clone());
        match self.receiver_mut() {
            Some(_inner_receiver) => {
                return Ok(_inner_receiver);
            },
            None => {
                error!("Attempt to call recv on closed receiver {} of channel id {}",
                        receiver_id, channel_id);
                return Err(MessageBusError::ReceiverClosed);
            }
        }
    }

    async fn inner_receive(&mut self) -> Result<T, MessageBusError<T>> {
        let inner_receiver = self.inner_receiver()?;
        let r = inner_receiver.recv().await;
        match r {
            Ok(data) => { return Ok(data); },
            Err(e) => { return Err(MessageBusError::ReceiveFailed(e.to_string())); }
        }
    }
}

impl<T: std::clone::Clone + std::marker::Send + std::marker::Sync + std::fmt::Debug +'static> StaticClonableAsyncComponent
for TokioBroadcastingMessageBusReceiver<T> {}

impl<T: Clone> Clone for TokioBroadcastingMessageBusReceiver<T> {
    fn clone(&self) -> TokioBroadcastingMessageBusReceiver<T> {
        match self.receiver() {
            Some(inner_receiver) => {
                return TokioBroadcastingMessageBusReceiver::new(self.channel_id, inner_receiver.resubscribe());
            },
            None => {
                warn!("Attempt to clone a closed receiver {} of channel id {}", self.id(), self.channel_id());
                return TokioBroadcastingMessageBusReceiver { 
                    id: Uuid::new_v4(), channel_id: self.channel_id, receiver: None, closed: true };
            }
        }
    }
}



#[async_trait]
impl<T: Clone + std::marker::Send> MessageBusReceiver<T> for TokioBroadcastingMessageBusReceiver<T> {
    async fn receive(&mut self) -> Option<T> {
        if *self.closed() {
            return None;
        }
        let r = self.inner_receive().await;
        match r {
            Ok(data) => { Some(data) },
            Err(e) => { 
                error!("receive error: {}", e);
                None 
            }
        }
    }
    
    fn try_recv(&mut self) -> Result<T, MessageBusError<T>> {
        let data = self.inner_receiver()?.try_recv();
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
        self.receiver = None;
        self.closed = true;
    }
}
