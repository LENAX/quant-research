use std::sync::Arc;

use chrono::{offset::Local, DateTime};
use getset::{Getters, MutGetters, Setters};
use serde_json::Value;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{
    domain::synchronization::sync_task::SyncTask,
    infrastructure::mq::{
        message_bus::{
            BroadcastingMessageBusReceiver, BroadcastingMessageBusSender, MessageBusReceiver,
            MessageBusSender, MpscMessageBus, SpmcMessageBusReceiver, SpmcMessageBusSender,
            StaticAsyncComponent,
        },
        tokio_channel_mq::{
            TokioBroadcastingMessageBusReceiver, TokioBroadcastingMessageBusSender,
            TokioMpscMessageBusReceiver, TokioSpmcMessageBusReceiver, TokioSpmcMessageBusSender,
        },
    },
};

use super::{workers::errors::SyncWorkerError, GetTaskRequest};

/// Some common traits used by components in the sync module
///

pub trait SyncTaskMPSCReceiver:
    MessageBusReceiver<Arc<Mutex<SyncTask>>> + MpscMessageBus + StaticAsyncComponent
{
}
impl SyncTaskMPSCReceiver for TokioMpscMessageBusReceiver<Arc<Mutex<SyncTask>>> {}

pub trait TaskRequestMPMCSender:
    MessageBusSender<GetTaskRequest>
    + StaticAsyncComponent
    + BroadcastingMessageBusSender<GetTaskRequest>
{
    fn clone_boxed(&self) -> Box<dyn TaskRequestMPMCSender>;
}

pub trait TaskRequestMPMCReceiver:
    MessageBusReceiver<GetTaskRequest> + StaticAsyncComponent + BroadcastingMessageBusReceiver
{
    fn clone_boxed(&self) -> Box<dyn TaskRequestMPMCReceiver>;
}

impl StaticAsyncComponent for TokioBroadcastingMessageBusSender<GetTaskRequest> {}

impl TaskRequestMPMCSender for TokioBroadcastingMessageBusSender<GetTaskRequest> {
    fn clone_boxed(&self) -> Box<dyn TaskRequestMPMCSender> {
        Box::new(self.clone())
    }
}
impl TaskRequestMPMCReceiver for TokioBroadcastingMessageBusReceiver<GetTaskRequest> {
    fn clone_boxed(&self) -> Box<dyn TaskRequestMPMCReceiver> {
        Box::new(self.clone())
    }
}

pub trait SyncTaskMPMCSender:
    MessageBusSender<Arc<Mutex<SyncTask>>> + StaticAsyncComponent + BroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>
{
}

pub trait SyncTaskMPMCReceiver:
    MessageBusReceiver<Arc<Mutex<SyncTask>>> + StaticAsyncComponent + BroadcastingMessageBusReceiver
{
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPMCReceiver>;
}

impl StaticAsyncComponent for TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>> {}

impl SyncTaskMPMCSender for TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>> {}
impl SyncTaskMPMCReceiver for TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>> {
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPMCReceiver> {
        Box::new(self.clone())
    }
}

#[derive(Debug, Clone, Getters, Setters, MutGetters)]
pub struct StreamingData {
    sync_plan_id: Uuid,
    task_id: Uuid,
    data: Option<Value>,
    received_time: DateTime<Local>,
}

impl StreamingData {
    pub fn new(
        plan_id: Uuid,
        task_id: Uuid,
        data: Option<Value>,
        received_time: DateTime<Local>,
    ) -> Self {
        Self {
            sync_plan_id: plan_id,
            task_id: task_id,
            data: data,
            received_time: received_time,
        }
    }
}

pub trait StreamingDataMPMCSender:
    MessageBusSender<StreamingData> + StaticAsyncComponent + BroadcastingMessageBusSender<StreamingData>
{
}

pub trait StreamingDataMPMCReceiver:
    MessageBusReceiver<StreamingData> + StaticAsyncComponent + BroadcastingMessageBusReceiver
{
    fn clone_boxed(&self) -> Box<dyn StreamingDataMPMCReceiver>;
}

impl StaticAsyncComponent for TokioBroadcastingMessageBusSender<StreamingData> {}

impl StreamingDataMPMCSender for TokioBroadcastingMessageBusSender<StreamingData> {}
impl StreamingDataMPMCReceiver for TokioBroadcastingMessageBusReceiver<StreamingData> {
    fn clone_boxed(&self) -> Box<dyn StreamingDataMPMCReceiver> {
        Box::new(self.clone())
    }
}

pub trait SyncWorkerErrorMPMCSender:
    MessageBusSender<SyncWorkerError>
    + StaticAsyncComponent
    + BroadcastingMessageBusSender<SyncWorkerError>
{
}

pub trait SyncWorkerErrorMPMCReceiver:
    MessageBusReceiver<SyncWorkerError> + StaticAsyncComponent + BroadcastingMessageBusReceiver
{
    fn clone_boxed(&self) -> Box<dyn SyncWorkerErrorMPMCReceiver>;
}

impl StaticAsyncComponent for TokioBroadcastingMessageBusSender<SyncWorkerError> {}

impl SyncWorkerErrorMPMCSender for TokioBroadcastingMessageBusSender<SyncWorkerError> {}
impl SyncWorkerErrorMPMCReceiver for TokioBroadcastingMessageBusReceiver<SyncWorkerError> {
    fn clone_boxed(&self) -> Box<dyn SyncWorkerErrorMPMCReceiver> {
        Box::new(self.clone())
    }
}

/// FailedTaskSPMCReceiver and FailedTaskSPMCReceiver
/// Single Producer Multiple Receiver channel trait for sending back failed task to retry
pub type FailedTask = (Uuid, Arc<Mutex<SyncTask>>);
pub trait FailedTaskSPMCReceiver:
    MessageBusReceiver<FailedTask> + StaticAsyncComponent + SpmcMessageBusReceiver
{
    fn clone_boxed(&self) -> Box<dyn FailedTaskSPMCReceiver>;
}

impl FailedTaskSPMCReceiver for TokioSpmcMessageBusReceiver<FailedTask> {
    fn clone_boxed(&self) -> Box<dyn FailedTaskSPMCReceiver> {
        Box::new(self.clone())
    }
}

pub trait FailedTaskSPMCSender:
    MessageBusSender<FailedTask> + StaticAsyncComponent + SpmcMessageBusSender<FailedTask>
{
}

impl FailedTaskSPMCSender for TokioSpmcMessageBusSender<FailedTask> {}
