use async_trait::async_trait;
/// Task Executor Trait
/// Defines the common interface for task execution
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::infrastructure::{
    mq::tokio_channel_mq::TokioBroadcastingMessageBusReceiver,
    sync::{
        factory::Builder,
        task_manager::{
            factory::{rate_limiter::RateLimiterBuilder, task_queue::TaskQueueBuilder},
            sync_rate_limiter::WebRequestRateLimiter,
            task_manager::TaskManager,
            task_queue::TaskQueue,
            tm_traits::SyncTaskManager,
        },
        GetTaskRequest, shared_traits::StreamingData, workers::errors::SyncWorkerError,
    },
};

use super::{rate_limiter::RateLimiter, sync_plan::SyncPlan, sync_task::SyncTask};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskExecutorError {
    LoadPlanFailure,
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
    type TaskManagerType;
    type TaskQueueType;

    // add new sync plans to synchronize
    async fn assign(
        &mut self,
        sync_plans: Vec<Arc<RwLock<SyncPlan>>>,
    ) -> Result<(), TaskExecutorError>
    where
        <WebRequestRateLimiter as RateLimiter>::BuilderType:
            Builder<Product = WebRequestRateLimiter> + RateLimiterBuilder + Send,
        <Self::TaskQueueType as TaskQueue>::BuilderType: Builder<Product = Self::TaskQueueType>
            + TaskQueueBuilder<
                RateLimiterType = WebRequestRateLimiter,
                TaskRequestReceiverType = TokioBroadcastingMessageBusReceiver<GetTaskRequest>,
            > + Send,
        <Self::TaskManagerType as SyncTaskManager>::TaskQueueType: TaskQueue,
        <Self as TaskExecutor>::TaskManagerType: SyncTaskManager,
        <Self as TaskExecutor>::TaskQueueType: TaskQueue;

    // wait and continuously get completed task
    async fn subscribe_completed_task(&mut self) -> Result<Arc<Mutex<SyncTask>>, TaskExecutorError>;

    // wait and continuously get streaming data
    async fn subscribe_streaming_data(&mut self) -> Result<StreamingData, TaskExecutorError>;

    async fn subscribe_failed_task(&mut self) -> Result<Arc<Mutex<SyncTask>>, TaskExecutorError>;

    async fn subscribe_worker_error(&mut self) -> Result<SyncWorkerError, TaskExecutorError>;

    // run a single plan. Either start a new plan or continue a paused plan
    async fn run(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError>;

    // run all assigned plans
    async fn run_all(&mut self) -> Result<(), TaskExecutorError>;

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
