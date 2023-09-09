/**
 * Task queue builders and factories
 */
use std::{collections::VecDeque, sync::Arc};

use getset::{Getters, MutGetters, Setters};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::{
    domain::synchronization::{
        rate_limiter::RateLimiter, sync_plan::SyncPlan, sync_task::SyncTask,
    },
    infrastructure::sync::{
        factory::Builder,
        shared_traits::TaskRequestMPMCReceiver,
        task_manager::{
            sync_task_queue::{QueueStatus, SyncTaskQueue},
            task_queue::TaskQueue,
        },
    },
};

pub async fn create_task_queue<
    QB: Builder
        + TaskQueueBuilder
        + TaskQueueBuilder<RateLimiterType = RL, TaskRequestReceiverType = TR>,
    RL: RateLimiter,
    TR: TaskRequestMPMCReceiver,
>(
    sync_plan_id: Uuid,
    sync_plan: Option<Arc<RwLock<SyncPlan>>>,
    rate_limiter: Option<RL>,
    task_req_receiver: TR,
) -> QB::Product
where
    QB::Product: TaskQueue,
{
    let mut builder = QB::new();

    // Always set task_request_receiver and status
    builder = builder
        .with_task_request_receiver(task_req_receiver)
        .with_sync_plan_id(sync_plan_id);

    if let Some(limiter) = rate_limiter {
        builder = builder.with_rate_limiter(limiter);
    }

    if let Some(plan) = sync_plan {
        let plan_lock = plan.read().await;
        let tasks = plan_lock
            .tasks()
            .iter()
            .map(|t| Arc::new(Mutex::new(t.clone())))
            .collect::<VecDeque<_>>();

        builder = builder
            .with_tasks(tasks)
            .with_initial_size(plan_lock.tasks().len());

        if let Some(q) = plan_lock.sync_config().sync_rate_quota() {
            builder = builder.with_max_retry(*q.max_retry());
        }
    }

    builder.build()
}

pub trait TaskQueueBuilder {
    type RateLimiterType;
    type TaskRequestReceiverType;

    fn with_sync_plan_id(self, sync_plan_id: Uuid) -> Self;
    fn with_tasks(self, tasks: VecDeque<Arc<Mutex<SyncTask>>>) -> Self;
    fn with_task_request_receiver(self, receiver: Self::TaskRequestReceiverType) -> Self;
    fn with_rate_limiter(self, rate_limiter: Self::RateLimiterType) -> Self;
    fn with_max_retry(self, max_retry: u32) -> Self;
    fn with_retries_left(self, retries_left: u32) -> Self;
    fn with_status(self, status: QueueStatus) -> Self;
    fn with_initial_size(self, initial_size: usize) -> Self;
}

/// Builders
/// 1. SyncTaskQueueBuilder
/// 2. TaskManagerBuilder
#[derive(Debug, Getters, MutGetters, Setters)]
pub struct SyncTaskQueueBuilder<RL: RateLimiter, TR: TaskRequestMPMCReceiver> {
    sync_plan_id: Option<Uuid>,
    tasks: Option<VecDeque<Arc<Mutex<SyncTask>>>>,
    task_request_receiver: Option<TR>,
    rate_limiter: Option<RL>,
    max_retry: Option<u32>,
    retries_left: Option<u32>,
    status: Option<QueueStatus>,
    initial_size: Option<usize>,
}

impl<RL: RateLimiter, TR: TaskRequestMPMCReceiver> Default for SyncTaskQueueBuilder<RL, TR> {
    fn default() -> Self {
        Self {
            sync_plan_id: None,
            tasks: Some(VecDeque::<Arc<Mutex<SyncTask>>>::new()),
            task_request_receiver: None,
            rate_limiter: None,
            max_retry: None,
            retries_left: None,
            status: Some(QueueStatus::Initialized),
            initial_size: Some(0),
        }
    }
}

impl<RL: RateLimiter, TR: TaskRequestMPMCReceiver> TaskQueueBuilder
    for SyncTaskQueueBuilder<RL, TR>
{
    type RateLimiterType = RL;
    type TaskRequestReceiverType = TR;

    fn with_sync_plan_id(mut self, sync_plan_id: Uuid) -> Self {
        self.sync_plan_id = Some(sync_plan_id);
        self
    }

    fn with_tasks(mut self, tasks: VecDeque<Arc<Mutex<SyncTask>>>) -> Self {
        self.tasks = Some(tasks);
        self
    }

    fn with_task_request_receiver(mut self, receiver: TR) -> Self {
        self.task_request_receiver = Some(receiver);
        self
    }

    fn with_rate_limiter(mut self, rate_limiter: RL) -> Self {
        self.rate_limiter = Some(rate_limiter);
        self
    }

    fn with_max_retry(mut self, max_retry: u32) -> Self {
        self.max_retry = Some(max_retry);
        self
    }

    fn with_retries_left(mut self, retries_left: u32) -> Self {
        self.retries_left = Some(retries_left);
        self
    }

    fn with_status(mut self, status: QueueStatus) -> Self {
        self.status = Some(status);
        self
    }

    fn with_initial_size(mut self, initial_size: usize) -> Self {
        self.initial_size = Some(initial_size);
        self
    }
}

impl<RL: RateLimiter, TR: TaskRequestMPMCReceiver> Builder for SyncTaskQueueBuilder<RL, TR> {
    type Product = SyncTaskQueue<RL, TR>;

    fn new() -> Self {
        SyncTaskQueueBuilder::default()
    }

    fn build(self) -> Self::Product {
        SyncTaskQueue::new(
            self.tasks.unwrap_or_else(VecDeque::new), 
            self.rate_limiter,
            self.max_retry, 
            self.sync_plan_id.expect("Plan id is required!"), 
            self
                .task_request_receiver
                .expect("Object implemented the TaskRequestMPMCReceiver trait is required"),
        )
    }
}
