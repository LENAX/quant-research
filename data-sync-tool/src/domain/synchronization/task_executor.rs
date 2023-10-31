use async_trait::async_trait;
/// Task Executor Trait
/// Defines the common interface for task execution
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::infrastructure::{
    mq::tokio_channel_mq::TokioBroadcastingMessageBusReceiver,
    sync::{
        factory::Builder,
        task_manager::{
            factory::{rate_limiter::RateLimiterBuilder, task_queue::TaskQueueBuilder},
            task_queue::TaskQueue,
            tm_traits::SyncTaskManager,
        },
        GetTaskRequest
    },
};

use super::{rate_limiter::RateLimiter, sync_plan::SyncPlan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskExecutorError {
    LoadPlanFailure,
    NoWorkerAssigned,
    WorkerAssignmentFailed(String),
    SyncFailure(String)
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanProgress {
    plan_id: Uuid,
    name: String,
    total_tasks: usize,
    completed_task: usize,
    completion_rate: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncProgress {
    plan_progress: HashMap<Uuid, PlanProgress>,
}

#[async_trait]
pub trait TaskExecutor: Sync + Send {
    type TaskManagerType: SyncTaskManager;
    type TaskQueueType: TaskQueue;

    // Move the common bounds to associated types to declutter the method signature
    type RateLimiter: RateLimiter;
    type RateLimiterBuilder: Builder<Product = Self::RateLimiter> + RateLimiterBuilder + Send;

    type QueueBuilder: Builder<Product = Self::TaskQueueType> 
        + TaskQueueBuilder<
            RateLimiterType = Self::RateLimiter,
            TaskRequestReceiverType = TokioBroadcastingMessageBusReceiver<GetTaskRequest>,
        > + Send;

    type CompletedTaskChannelType;
    type StreamingDataChannelType;
    type FailedTaskChannelType;
    type WorkerErrorChannelType;

    // add new sync plans to synchronize
    async fn assign(
        &mut self,
        sync_plans: Vec<Arc<RwLock<SyncPlan>>>,
    ) -> Result<(), TaskExecutorError>;

    // wait and continuously get completed task
    fn subscribe_completed_task(&mut self) -> Self::CompletedTaskChannelType;

    // wait and continuously get streaming data
    fn subscribe_streaming_data(&mut self) -> Self::StreamingDataChannelType;

    fn subscribe_failed_task(&mut self) -> Self::FailedTaskChannelType;

    fn subscribe_worker_error(&mut self) -> Self::WorkerErrorChannelType;

    // run a single plan. Either start a new plan or continue a paused plan
    async fn run(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError>;

    // run all assigned plans
    async fn run_all(&'static mut self) -> Result<(), TaskExecutorError>;

    // temporarily pause a plan
    async fn pause(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError>;

    // pause all plans
    async fn pause_all(&mut self) -> Result<(), TaskExecutorError>;

    // cancel sync for plan, also removes it from the executor
    async fn cancel(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError>;

    // cancel and drop all plans
    async fn cancel_all(&mut self) -> Result<(), TaskExecutorError>;

    // report current progress
    async fn report_progress(&self) -> Result<SyncProgress, TaskExecutorError>;
}
