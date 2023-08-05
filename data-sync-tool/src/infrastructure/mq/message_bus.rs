use async_trait::async_trait;
use futures::channel::mpsc::TryRecvError;
use std::fmt;
use std::{
    error::Error,
    fmt::{Display, Formatter},
};
use tokio::sync::broadcast;

// Errors
#[derive(Debug)]
pub enum MessageBusFailureCause {
    Full,
    Closed,
    Unknown,
}

#[derive(Debug)]
pub enum MessageBusError<T> {
    SendFailed(T, MessageBusFailureCause),
    ReceiveFailed(String),
    SenderClosed,
    ReceiverClosed,
}

impl<T> Display for MessageBusError<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl<T: std::fmt::Debug> Error for MessageBusError<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

// Interfaces
pub trait StaticMpscMQReceiver: MpscMessageBus + Sync + Send + 'static {}
pub trait StaticMpscMQSender: MpscMessageBus + Sync + Send + 'static {}
pub trait StaticClonableMpscMQ: MpscMessageBus + Sync + Send + Clone + 'static {}
pub trait StaticClonableAsyncComponent: Sync + Send + Clone + 'static {}
pub trait StaticAsyncComponent: Sync + Send + 'static {}

/// Message Queue Trait
/// TODO: Consider separate receivers and senders as different related traits
/// Separating receivers and senders can reduce the amount locks required
#[async_trait]
pub trait MessageBus<T: Send + Sync + 'static> {
    async fn send(&self, message: T) -> Result<(), MessageBusError<T>>;
    async fn receive(&mut self) -> Option<T>;
    async fn close(&mut self);
}

#[async_trait]
pub trait MessageBusSender<T> {
    async fn send(&self, message: T) -> Result<(), MessageBusError<T>>;
    fn try_send(&self, message: T) -> Result<(), MessageBusError<T>>;
    async fn close(&mut self);
    fn is_closed(&self) -> bool;
}

#[async_trait]
pub trait MessageBusReceiver<T> {
    async fn receive(&mut self) -> Option<T>;
    fn try_recv(&mut self) -> Result<T, MessageBusError<T>>;
    fn close(&mut self);
}

// Marks Message Bus as Multiple Producer Single Consumer Message Bus
pub trait MpscMessageBus {}

// Marks a single consumer single producer message bus
pub trait OneshotMessageBus {}

// Marks a multiple consumer multiple producer message bus sender
pub trait BroadcastingMessageBusSender<T> {
    fn subsribe(&self) -> Result<Box<dyn MessageBusReceiver<T>>, MessageBusError<T>>;
    fn receiver_count(&self) -> Result<usize, MessageBusError<T>>;
    fn same_channel(
        &self,
        other: &dyn BroadcastingMessageBusSender<T>,
    ) -> Result<bool, MessageBusError<T>>;
    fn is_closed(&self) -> bool;
    fn inner(&self) -> Option<&broadcast::Sender<T>>;
}

// Marks a multiple consumer multiple producer message bus sender
// pub trait BroadcastingMessageBusSender {}
// Marks a multiple consumer multiple producer message bus receiver
pub trait BroadcastingMessageBusReceiver {}

// Marks a single producer multiple consumer message bus sender
pub trait SpmcMessageBusSender<T> {
    fn subscribe(&self) -> Result<Box<dyn MessageBusReceiver<T>>, MessageBusError<T>>;
    fn receiver_count(&self) -> Result<usize, MessageBusError<T>>;
    fn same_channel(&self, other: &dyn SpmcMessageBusSender<T>)
        -> Result<bool, MessageBusError<T>>;
    fn is_closed(&self) -> bool;
    fn sender(&self) -> Option<&broadcast::Sender<T>>;
}

// Marks a single producer multiple consumer message bus receiver
pub trait SpmcMessageBusReceiver {}
