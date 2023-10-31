use async_trait::async_trait;
use derivative::Derivative;
use uuid::Uuid;

use crate::infrastructure::{
    mq::{
        message_bus::{
            MessageBusReceiver, MessageBusSender, MpscMessageBus, StaticAsyncComponent,
            StaticClonableMpscMQ,
        },
        tokio_channel_mq::{TokioMpscMessageBusReceiver, TokioMpscMessageBusSender},
    },
    sync::shared_traits::StreamingData,
};

use super::errors::SyncWorkerError;

/**
 * Synchronization worker traits
 */

// TODO: Change workers to pull based model
// Workers will actively request task managers for sync task instead of passively accepting sync task

#[derive(Derivative, PartialEq, Eq, Clone, Copy)]
#[derivative(Default(bound = ""))]
pub enum WorkerState {
    #[derivative(Default)]
    // start with this state
    // change from working to Idle when `pause` is called
    Idle,
    // Becomes working state when `start_sync` is called
    Working,
    // Becomes this state when `stop` is called
    Stopped,
}

pub enum WorkerCommand {
    Start,
    Pause,
    Resume,
    Cancel,
    Assign(Uuid)
}

#[async_trait]
pub trait SyncWorker: Send + Sync {
    type BuilderType;

    // handles sync task, then updates its states and result
    // async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<&mut SyncTask, Box<dyn Error>>;
    // async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), SyncWorkerError>;
    async fn start_sync(&mut self) -> Result<(), SyncWorkerError>;
    fn pause(&mut self) -> Result<(), SyncWorkerError>;
    fn stop(&mut self) -> Result<(), SyncWorkerError>;
    fn assign_sync_plan(&mut self, sync_plan_id: &Uuid) -> Result<(), SyncWorkerError>;
    fn current_state(&self) -> WorkerState;
}

/// A marker trait that marks a long running worker
pub trait LongTaskHandlingWorker {}

/// A market trait for workers handling short tasks
pub trait ShortTaskHandlingWorker {}
pub trait ShortRunningWorker: SyncWorker + ShortTaskHandlingWorker {}
pub trait LongRunningWorker: SyncWorker + LongTaskHandlingWorker {}

trait SyncTaskStreamingDataMpscReceiver:
    MessageBusReceiver<StreamingData> + StaticClonableMpscMQ + Clone
{
}
trait SyncTaskStreamingDataMpscSender:
    MessageBusSender<StreamingData> + StaticClonableMpscMQ + Clone
{
}

trait SyncWorkerErrorMessageMpscReceiver:
    MessageBusReceiver<SyncWorkerError> + StaticClonableMpscMQ + Clone
{
}
trait SyncWorkerErrorMessageMpscSender:
    MessageBusSender<SyncWorkerError> + StaticClonableMpscMQ + Clone
{
}

impl SyncTaskStreamingDataMpscSender for TokioMpscMessageBusSender<StreamingData> {}
impl SyncWorkerErrorMessageMpscSender for TokioMpscMessageBusSender<SyncWorkerError> {}

// Try to work around the object safety restriction
// Traits extend from Clone is not object-safe
// Use this trait in external modules and use clone_boxed method to clone the object

pub trait SyncTaskStreamingDataMPSCReceiver:
    MessageBusReceiver<StreamingData> + MpscMessageBus + StaticAsyncComponent
{
}
pub trait SyncTaskStreamingDataMPSCSender:
    MessageBusSender<StreamingData> + MpscMessageBus + StaticAsyncComponent
{
}

pub trait SyncWorkerErrorMessageMPSCReceiver:
    MessageBusReceiver<SyncWorkerError> + MpscMessageBus + StaticAsyncComponent
{
}

pub trait SyncWorkerErrorMessageMPSCSender:
    MessageBusSender<SyncWorkerError> + MpscMessageBus + StaticAsyncComponent
{
    fn clone_boxed(&self) -> Box<dyn SyncWorkerErrorMessageMPSCSender>;
}


impl SyncWorkerErrorMessageMPSCSender for TokioMpscMessageBusSender<SyncWorkerError> {
    fn clone_boxed(&self) -> Box<dyn SyncWorkerErrorMessageMPSCSender> {
        Box::new(self.clone())
    }
    // implement the methods required by SyncWorkerErrorMessageMPSCSender here
}

impl SyncTaskStreamingDataMPSCReceiver for TokioMpscMessageBusReceiver<StreamingData> {}
impl SyncWorkerErrorMessageMPSCReceiver for TokioMpscMessageBusReceiver<SyncWorkerError> {}
