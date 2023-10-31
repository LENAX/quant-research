//! Specialized traits used by Task Manager

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use derivative::Derivative;
use getset::{Getters, Setters};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::domain::synchronization::rate_limiter::RateLimiter;
use crate::domain::synchronization::sync_plan::SyncPlan;
use crate::infrastructure::mq::message_bus::{
    MessageBusReceiver, MpscMessageBus, SpmcMessageBusReceiver, SpmcMessageBusSender,
    StaticAsyncComponent, StaticClonableAsyncComponent, StaticClonableMpscMQ, StaticMpscMQReceiver,
};
use crate::infrastructure::mq::tokio_channel_mq::{
    TokioMpscMessageBusReceiver, TokioMpscMessageBusSender, TokioSpmcMessageBusReceiver,
    TokioSpmcMessageBusSender,
};
use crate::infrastructure::sync::factory::Builder;
use crate::infrastructure::sync::shared_traits::{FailedTask, TaskRequestMPMCReceiver};
use crate::{
    domain::synchronization::sync_task::SyncTask, infrastructure::mq::message_bus::MessageBusSender,
};

use super::errors::TaskManagerError;
use super::factory::rate_limiter::RateLimiterBuilder;
use super::factory::task_queue::TaskQueueBuilder;
use super::task_queue::TaskQueue;

trait SyncTaskMpscSender: MessageBusSender<Arc<Mutex<SyncTask>>> + StaticClonableMpscMQ {}
trait SyncTaskMpscReceiver: MessageBusReceiver<Arc<Mutex<SyncTask>>> + StaticMpscMQReceiver {}

impl SyncTaskMpscSender for TokioMpscMessageBusSender<Arc<Mutex<SyncTask>>> {}
impl SyncTaskMpscReceiver for TokioMpscMessageBusReceiver<Arc<Mutex<SyncTask>>> {}

pub trait SyncTaskMPSCSender:
    MessageBusSender<Arc<Mutex<SyncTask>>> + MpscMessageBus + StaticAsyncComponent
{
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPSCSender>;
}

impl SyncTaskMPSCSender for TokioMpscMessageBusSender<Arc<Mutex<SyncTask>>> {
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPSCSender> {
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

/// TaskManager
#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct TaskSendingProgress {
    sync_plan_id: Uuid,
    task_sent: usize,
    total_tasks: usize,
    complete_rate: f32,
}

impl TaskSendingProgress {
    pub fn new(
        sync_plan_id: Uuid,
        task_sent: usize,
        total_tasks: usize,
        complete_rate: f32,
    ) -> Self {
        Self {
            sync_plan_id,
            task_sent,
            total_tasks,
            complete_rate,
        }
    }
}

pub enum TaskManagerCommand {
    AddPlan(Vec<RwLock<SyncPlan>>),
    CancelPlan(Uuid),
    PausePlan(Uuid),
    ResumePlan(Uuid)
}

#[async_trait]
pub trait SyncTaskManager: Sync + Send {
    type TaskQueueType;

    // start syncing all plans by sending tasks out to workers
    async fn listen_for_get_task_request(&mut self) -> Result<(), TaskManagerError>;

    // stop syncing all plans
    async fn stop_sending_all_tasks(
        &mut self,
    ) -> Result<HashMap<Uuid, Vec<Arc<Mutex<SyncTask>>>>, TaskManagerError>;

    // async fn new_empty_queue(
    //     &self,
    //     sync_plan: Arc<RwLock<SyncPlan>>,
    //     task_request_receiver: TokioBroadcastingMessageBusReceiver<GetTaskRequest>,
    // ) -> Self::TaskQueueType;
    // async fn new_empty_queues(
    //     &self,
    //     sync_plan: Arc<RwLock<SyncPlan>>,
    //     task_request_receivers: Vec<impl TaskRequestMPMCReceiver>,
    // ) -> Self::TaskQueueType;

    // When need add new tasks ad hoc, use this method
    async fn add_tasks_to_plan(
        &mut self,
        plan_id: Uuid,
        tasks: Vec<Arc<Mutex<SyncTask>>>,
    ) -> Result<(), TaskManagerError>;

    // add new plans to sync
    async fn load_sync_plan<RL: RateLimiter, TR: TaskRequestMPMCReceiver>(
        &mut self,
        sync_plan: Arc<RwLock<SyncPlan>>,
        task_request_receiver: TR,
    ) -> Result<(), TaskManagerError>
    where
        <RL as RateLimiter>::BuilderType: Builder<Product = RL> + RateLimiterBuilder + Send,
        <Self::TaskQueueType as TaskQueue>::BuilderType: Builder<Product = Self::TaskQueueType>
            + TaskQueueBuilder<RateLimiterType = RL, TaskRequestReceiverType = TR>
            + Send,
        <Self as SyncTaskManager>::TaskQueueType: TaskQueue;

    async fn load_sync_plans<RL: RateLimiter, TR: TaskRequestMPMCReceiver>(
        &mut self,
        sync_plans: Vec<Arc<RwLock<SyncPlan>>>,
        task_request_receivers: Vec<TR>,
    ) -> Result<(), TaskManagerError>
    where
        <RL as RateLimiter>::BuilderType: Builder<Product = RL> + RateLimiterBuilder + Send,
        <Self::TaskQueueType as TaskQueue>::BuilderType: Builder<Product = Self::TaskQueueType>
            + TaskQueueBuilder<RateLimiterType = RL, TaskRequestReceiverType = TR>
            + Send,
        <Self as SyncTaskManager>::TaskQueueType: TaskQueue;

    // stop syncing given the id, but it is resumable
    async fn stop_sending_task(&mut self, sync_plan_id: Uuid) -> Result<(), TaskManagerError>;

    // stop and remove the sync plan
    async fn stop_and_remove_sync_plan(
        &mut self,
        sync_plan_id: Uuid,
    ) -> Result<Vec<Arc<Mutex<SyncTask>>>, TaskManagerError>;

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
