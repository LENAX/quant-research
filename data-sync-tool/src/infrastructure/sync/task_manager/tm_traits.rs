//! Specialized traits used by Task Manager

use std::collections::HashMap;

use async_trait::async_trait;
use derivative::Derivative;
use getset::{Getters, Setters};
use uuid::Uuid;

use crate::domain::synchronization::rate_limiter::RateLimiter;
use crate::domain::synchronization::sync_plan::SyncPlan;
use crate::infrastructure::mq::message_bus::{
    BroadcastingMessageBusReceiver, BroadcastingMessageBusSender, MessageBusReceiver,
    MpscMessageBus, SpmcMessageBusReceiver, SpmcMessageBusSender, StaticAsyncComponent,
    StaticClonableAsyncComponent, StaticClonableMpscMQ, StaticMpscMQReceiver,
};
use crate::infrastructure::mq::tokio_channel_mq::{
    TokioBroadcastingMessageBusReceiver, TokioBroadcastingMessageBusSender,
    TokioMpscMessageBusReceiver, TokioMpscMessageBusSender, TokioSpmcMessageBusReceiver,
    TokioSpmcMessageBusSender,
};
use crate::{
    domain::synchronization::sync_task::SyncTask, infrastructure::mq::message_bus::MessageBusSender,
};

use super::errors::TaskManagerError;

trait SyncTaskMpscSender: MessageBusSender<SyncTask> + StaticClonableMpscMQ {}
trait SyncTaskMpscReceiver: MessageBusReceiver<SyncTask> + StaticMpscMQReceiver {}

impl SyncTaskMpscSender for TokioMpscMessageBusSender<SyncTask> {}
impl SyncTaskMpscReceiver for TokioMpscMessageBusReceiver<SyncTask> {}

pub trait SyncTaskMPSCSender:
    MessageBusSender<SyncTask> + MpscMessageBus + StaticAsyncComponent
{
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPSCSender>;
}

impl SyncTaskMPSCSender for TokioMpscMessageBusSender<SyncTask> {
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPSCSender> {
        Box::new(self.clone())
    }
}

pub trait SyncTaskMPSCReceiver:
    MessageBusReceiver<SyncTask> + MpscMessageBus + StaticAsyncComponent
{
}
impl SyncTaskMPSCReceiver for TokioMpscMessageBusReceiver<SyncTask> {}

pub trait SyncTaskMPMCSender:
    MessageBusSender<SyncTask> + StaticAsyncComponent + BroadcastingMessageBusSender<SyncTask>
{
}

pub trait SyncTaskMPMCReceiver:
    MessageBusReceiver<SyncTask> + StaticAsyncComponent + BroadcastingMessageBusReceiver
{
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPMCReceiver>;
}

impl StaticAsyncComponent for TokioBroadcastingMessageBusSender<SyncTask> {}

impl SyncTaskMPMCSender for TokioBroadcastingMessageBusSender<SyncTask> {}
impl SyncTaskMPMCReceiver for TokioBroadcastingMessageBusReceiver<SyncTask> {
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPMCReceiver> {
        Box::new(self.clone())
    }
}

#[derive(Derivative, Getters, Setters, Default, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct GetTaskRequest {
    sync_plan_id: Uuid,
}

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

trait TaskManagerErrorMpscSender: MessageBusSender<TaskManagerError> + StaticClonableMpscMQ {}
pub trait TaskManagerErrorMPSCSender:
    MessageBusSender<TaskManagerError> + MpscMessageBus + StaticAsyncComponent
{
    fn clone_boxed(&self) -> Box<dyn TaskManagerErrorMPSCSender>;
}

impl TaskManagerErrorMPSCSender for TokioMpscMessageBusSender<TaskManagerError> {
    fn clone_boxed(&self) -> Box<dyn TaskManagerErrorMPSCSender> {
        Box::new(self.clone())
    }
}

trait TaskManagerErrorMpscReceiver:
    MessageBusReceiver<TaskManagerError> + StaticMpscMQReceiver
{
}

impl TaskManagerErrorMpscSender for TokioMpscMessageBusSender<TaskManagerError> {}
impl TaskManagerErrorMpscReceiver for TokioMpscMessageBusReceiver<TaskManagerError> {}

pub trait TaskManagerErrorMPSCReceiver:
    MessageBusReceiver<TaskManagerError> + MpscMessageBus + StaticAsyncComponent
{
}

impl TaskManagerErrorMPSCReceiver for TokioMpscMessageBusReceiver<TaskManagerError> {}

pub type FailedTask = (Uuid, SyncTask);
trait FailedTaskSpmcReceiver:
    MessageBusReceiver<FailedTask> + StaticClonableAsyncComponent + SpmcMessageBusReceiver
{
}

trait FailedTaskSpmcSender:
    MessageBusSender<FailedTask> + StaticAsyncComponent + SpmcMessageBusSender<FailedTask>
{
}

impl FailedTaskSpmcReceiver for TokioSpmcMessageBusReceiver<FailedTask> {}
impl FailedTaskSpmcSender for TokioSpmcMessageBusSender<FailedTask> {}

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

/// TaskManager
#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct TaskSendingProgress {
    sync_plan_id: Uuid,
    task_sent: usize,
    total_tasks: usize,
    complete_rate: f32,
}


#[async_trait]
pub trait SyncTaskManager<T: RateLimiter, TR: TaskRequestMPMCReceiver> {
    // start syncing all plans by sending tasks out to workers
    async fn listen_for_get_task_request(&mut self) -> Result<(), TaskManagerError>;

    // stop syncing all plans
    async fn stop_sending_all_tasks(
        &mut self,
    ) -> Result<HashMap<Uuid, Vec<SyncTask>>, TaskManagerError>;

    // When need add new tasks ad hoc, use this method
    async fn add_tasks_to_plan(
        &mut self,
        plan_id: Uuid,
        tasks: Vec<SyncTask>,
    ) -> Result<(), TaskManagerError>;

    // add new plans to sync
    async fn load_sync_plan(
        &mut self,
        sync_plan: &SyncPlan,
        rate_limiter: Option<T>,
        task_request_receiver: TR
    ) -> Result<(), TaskManagerError>;
    async fn load_sync_plans(
        &mut self,
        sync_plans: &[SyncPlan],
        rate_limiters: Vec<Option<T>>,
        task_request_receivers: Vec<TR>
    ) -> Result<(), TaskManagerError>;

    // stop syncing given the id, but it is resumable
    async fn stop_sending_task(&mut self, sync_plan_id: Uuid) -> Result<(), TaskManagerError>;

    // stop and remove the sync plan
    async fn stop_and_remove_sync_plan(
        &mut self,
        sync_plan_id: Uuid,
    ) -> Result<Vec<SyncTask>, TaskManagerError>;

    // pause sending tasks
    // typically used when rate limiting does not help
    // then users can pause it for a while to let the remote release the limit
    async fn pause_sending_task(&mut self, sync_plan_id: Uuid) -> Result<(), TaskManagerError>;

    // resume sending tasks
    async fn resume_sending_tasks(&mut self, sync_plan_id: Uuid) -> Result<(), TaskManagerError>;

    // report how many unsent tasks for each sync plan
    async fn report_task_sending_progress(
        &self,
        sync_plan_id: Uuid,
    ) -> Result<TaskSendingProgress, TaskManagerError>;
    async fn report_all_task_sending_progress(
        &self,
    ) -> Result<Vec<TaskSendingProgress>, TaskManagerError>;

    // stop all sync plans and wait until all queues are closed, then releases all resources
    async fn graceful_shutdown(&mut self) -> Result<(), TaskManagerError>;

    // stop directly without waiting
    fn force_shutdown(&mut self);
}