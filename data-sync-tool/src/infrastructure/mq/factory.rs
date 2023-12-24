use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use super::{
    message_bus::{MessageBusReceiver, MessageBusSender},
    tokio_channel_mq::{
        TokioBroadcastingMessageBusReceiver, TokioBroadcastingMessageBusSender,
        TokioMpscMessageBusReceiver, TokioMpscMessageBusSender, TokioSpmcMessageBusReceiver,
        TokioSpmcMessageBusSender,
    },
};

/// Factory methods for message queues
///

// Factory
#[derive(Debug, Clone, Copy)]
pub enum SupportedMQImpl {
    InMemoryTokioChannel,
    // todo
    // RabbitMQ,
    // Redis,
    // Kafka,
}

#[derive(Debug, Clone, Copy)]
pub enum MQType {
    Mpsc,
    Spmc,
    Broadcast,
}

pub fn create_tokio_mpsc_channel<T>(
    n: usize,
) -> (TokioMpscMessageBusSender<T>, TokioMpscMessageBusReceiver<T>) {
    let (tx, rx) = mpsc::channel::<T>(n);
    let channel_id = Uuid::new_v4();
    let sender = TokioMpscMessageBusSender::new(channel_id, tx);
    let receiver = TokioMpscMessageBusReceiver::new(channel_id, rx);

    return (sender, receiver);
}

pub fn create_tokio_spmc_channel<T: std::clone::Clone>(
    n: usize,
) -> (TokioSpmcMessageBusSender<T>, TokioSpmcMessageBusReceiver<T>) {
    let (tx, rx) = broadcast::channel::<T>(n);
    let channel_id = Uuid::new_v4();
    let sender = TokioSpmcMessageBusSender::new(channel_id, tx);
    let receiver = TokioSpmcMessageBusReceiver::new(channel_id, rx);

    return (sender, receiver);
}

pub fn create_tokio_broadcasting_channel<T: std::clone::Clone>(
    n: usize,
) -> (
    TokioBroadcastingMessageBusSender<T>,
    TokioBroadcastingMessageBusReceiver<T>,
) {
    let (tx, rx) = broadcast::channel::<T>(n);
    let channel_id = Uuid::new_v4();
    let sender = TokioBroadcastingMessageBusSender::new(channel_id, tx);
    let receiver = TokioBroadcastingMessageBusReceiver::new(channel_id, rx);

    return (sender, receiver);
}

pub type TokioMpscFactory<T> =
    Box<dyn Fn(usize) -> (TokioMpscMessageBusSender<T>, TokioMpscMessageBusReceiver<T>)>;
pub type TokioSpmcFactory<T> =
    Box<dyn Fn(usize) -> (TokioSpmcMessageBusSender<T>, TokioSpmcMessageBusReceiver<T>)>;
pub type TokioBroadcastingFactory<T> = Box<
    dyn Fn(
        usize,
    ) -> (
        TokioBroadcastingMessageBusSender<T>,
        TokioBroadcastingMessageBusReceiver<T>,
    ),
>;

pub enum TokioMQFactory<T> {
    MpscFactory(TokioMpscFactory<T>),
    SpmcFactory(TokioSpmcFactory<T>),
    BroadcastFactory(TokioBroadcastingFactory<T>),
}

type MQFactory<T> =
    Box<dyn Fn(usize) -> (Box<dyn MessageBusSender<T>>, Box<dyn MessageBusReceiver<T>>)>;

pub fn create_tokio_mpsc_factory<T: 'static>() -> TokioMpscFactory<T> {
    Box::new(create_tokio_mpsc_channel)
}

pub fn create_tokio_spmc_factory<T: std::clone::Clone + 'static>() -> TokioSpmcFactory<T> {
    Box::new(create_tokio_spmc_channel)
}

pub fn create_tokio_broadcast_channel_factory<T: std::clone::Clone + 'static>(
) -> TokioBroadcastingFactory<T> {
    Box::new(create_tokio_broadcasting_channel)
}

pub fn get_tokio_mq_factory<T: std::clone::Clone + 'static>(mq_type: MQType) -> TokioMQFactory<T> {
    match mq_type {
        MQType::Mpsc => {
            let factory = create_tokio_mpsc_factory();
            return TokioMQFactory::MpscFactory(factory);
        }
        MQType::Spmc => {
            let factory = create_tokio_spmc_factory();
            return TokioMQFactory::SpmcFactory(factory);
        }
        MQType::Broadcast => {
            let factory = create_tokio_broadcast_channel_factory();
            return TokioMQFactory::BroadcastFactory(factory);
        }
    }
}

pub fn get_mq_factory<T: std::marker::Send + std::clone::Clone + 'static>(
    mq_impl: SupportedMQImpl,
    mq_type: MQType,
) -> MQFactory<T> {
    match mq_impl {
        SupportedMQImpl::InMemoryTokioChannel => match mq_type {
            MQType::Mpsc => {
                let factory = create_tokio_mpsc_factory();
                Box::new(move |n| {
                    let (sender, receiver) = factory(n);
                    (Box::new(sender), Box::new(receiver))
                })
            }
            MQType::Spmc => {
                let factory = create_tokio_spmc_factory();
                Box::new(move |n| {
                    let (sender, receiver) = factory(n);
                    (Box::new(sender), Box::new(receiver))
                })
            }
            MQType::Broadcast => {
                let factory = create_tokio_broadcast_channel_factory();
                Box::new(move |n| {
                    let (sender, receiver) = factory(n);
                    (Box::new(sender), Box::new(receiver))
                })
            }
        },
        // SupportedMQImpl::RabbitMQ => todo!(),
        // SupportedMQImpl::Redis => todo!(),
        // SupportedMQImpl::Kafka => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    struct Data {
        field1: i32,
        field2: f64,
    }

    #[test]
    fn it_should_create_a_mspc_channel() {
        let channel_size: usize = 10;
        let mpsc_channel_factory =
            get_mq_factory::<Data>(SupportedMQImpl::InMemoryTokioChannel, MQType::Mpsc);
        let (_mpsc_sender, _mpsc_receiver) = mpsc_channel_factory(channel_size);
    }
}
