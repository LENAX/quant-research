use async_trait::async_trait;
use std::{error::Error, fmt::{Display, Formatter}};
use std::fmt;

#[derive(Debug)]
pub enum MessageBusError {
    SendFailed(String),
    ReceiveFailed(String)
}

impl Display for MessageBusError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl Error for MessageBusError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}


/// Message Queue Trait
/// TODO: Consider separate receivers and senders as different related traits
/// Separating receivers and senders can reduce the amount locks required
#[async_trait]
pub trait MessageBus<T: Send + Sync + 'static> {
    async fn send(&self, message: T) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn receive(&mut self) -> Option<T>;
    async fn close(&mut self);
}

#[async_trait]
pub trait MessageBusSender<T: Send + Sync> {
    async fn send(&self, message: T) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn close(&self);
    async fn is_closed(&self) -> bool;
}

#[async_trait]
pub trait MessageBusReceiver<T: Send + Sync> {
    async fn receive(&mut self) -> Option<T>;
    async fn close(&mut self);
}

// Marks Message Bus as Multiple Producer Single Consumer Message Bus
pub trait MpscMessageBus {}

// Marks a single consumer single producer message bus
pub trait OneshotMessageBus {}

// Marks a multiple consumer multiple producer message bus
pub trait BroadcastMessageBus {}

// Marks a single producer multiple consumer message bus
pub trait SpmcMessageBus {}