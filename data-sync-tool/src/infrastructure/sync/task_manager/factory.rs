use std::collections::VecDeque;

use crate::infrastructure::sync::shared_traits::{
    FailedTaskSPMCReceiver, SyncTaskMPMCSender, TaskRequestMPMCReceiver,
};
use crate::infrastructure::sync::task_manager::sync_task_queue::QueueId;
use crate::infrastructure::sync::task_manager::task_manager::TaskManagerState;
use crate::infrastructure::sync::GetTaskRequest;
use crate::{
    application::synchronization::dtos::task_manager::CreateTaskManagerRequest,
    domain::synchronization::{rate_limiter::RateLimiter, sync_task::SyncTask},
    infrastructure::{
        mq::tokio_channel_mq::TokioBroadcastingMessageBusReceiver, sync::factory::Builder,
    },
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::sync_rate_limiter::{new_web_request_limiter, WebRequestRateLimiter};
use super::{
    sync_task_queue::{QueueStatus, SyncTaskQueue},
    task_manager::TaskManager,
    tm_traits::TaskManagerErrorMPSCSender,
};

/**
 * Factory methods and traits for the task manager module
 */

// Factory Methods
pub fn new_empty_queue<T: RateLimiter, TR: TaskRequestMPMCReceiver>(
    rate_limiter: T,
    task_request_receiver: TR,
    max_retry: Option<u32>,
    sync_plan_id: Uuid,
) -> SyncTaskQueue<T, TR> {
    return SyncTaskQueue::<T, TR>::new(
        vec![],
        Some(rate_limiter),
        max_retry,
        sync_plan_id,
        task_request_receiver,
    );
}

pub fn new_empty_limitless_queue<T: RateLimiter, TR: TaskRequestMPMCReceiver>(
    max_retry: Option<u32>,
    sync_plan_id: Uuid,
    task_request_receiver: TR,
) -> SyncTaskQueue<T, TR> {
    return SyncTaskQueue::new(vec![], None, max_retry, sync_plan_id, task_request_receiver);
}

pub fn create_sync_task_manager<
    T: RateLimiter + 'static,
    MT: SyncTaskMPMCSender,
    TR: TaskRequestMPMCReceiver,
    ES: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
>(
    create_request: &CreateTaskManagerRequest,
    task_sender: MT,
    task_request_receivers: Vec<TR>,
    error_sender: ES,
    failed_task_receiver: MF,
) -> TaskManager<WebRequestRateLimiter, MT, TR, ES, MF>
where
    Vec<SyncTaskQueue<T, TR>>: FromIterator<
        SyncTaskQueue<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>,
    >,
{
    // todo: use abstract factory and factory methods to avoid hard coding
    // for now, we only have a few impls so we can simply hardcode the factory.
    let queues: Vec<SyncTaskQueue<WebRequestRateLimiter, TR>> = create_request
        .create_task_queue_requests()
        .iter()
        .zip(task_request_receivers)
        .map(
            |(req, task_request_receiver)| match req.rate_limiter_param() {
                Some(create_limiter_req) => match req.rate_limiter_impl() {
                    WebRequestRateLimiter => {
                        let rate_limiter = new_web_request_limiter(
                            *create_limiter_req.max_request(),
                            *create_limiter_req.max_daily_request(),
                            *create_limiter_req.cooldown(),
                        );
                        let queue = new_empty_queue(
                            rate_limiter,
                            task_request_receiver,
                            *req.max_retry(),
                            *req.sync_plan_id(),
                        );
                        return queue;
                    }
                },
                None => {
                    let queue = new_empty_limitless_queue(
                        *req.max_retry(),
                        *req.sync_plan_id(),
                        task_request_receiver,
                    );
                    return queue;
                }
            },
        )
        .collect();

    let task_manager = TaskManager::new(queues, task_sender, error_sender, failed_task_receiver);
    return task_manager;
}

/// Builders
/// 1. SyncTaskQueueBuilder
/// 2. TaskManagerBuilder
pub struct SyncTaskQueueBuilder<T: RateLimiter, TR: TaskRequestMPMCReceiver> {
    sync_plan_id: Option<Uuid>,
    tasks: Option<VecDeque<Arc<Mutex<SyncTask>>>>,
    task_request_receiver: Option<TR>,
    rate_limiter: Option<T>,
    max_retry: Option<u32>,
    retries_left: Option<u32>,
    status: Option<QueueStatus>,
    initial_size: Option<usize>,
}

impl<T: RateLimiter, TR: TaskRequestMPMCReceiver> SyncTaskQueueBuilder<T, TR> {
    pub fn sync_plan_id(mut self, id: Uuid) -> Self {
        self.sync_plan_id = Some(id);
        self
    }

    pub fn tasks(mut self, tasks: VecDeque<Arc<Mutex<SyncTask>>>) -> Self {
        self.tasks = Some(tasks);
        self
    }

    pub fn task_request_receiver(mut self, receiver: TR) -> Self {
        self.task_request_receiver = Some(receiver);
        self
    }

    pub fn rate_limiter(mut self, limiter: T) -> Self {
        self.rate_limiter = Some(limiter);
        self
    }

    pub fn max_retry(mut self, retry: u32) -> Self {
        self.max_retry = Some(retry);
        self
    }

    pub fn retries_left(mut self, retries: u32) -> Self {
        self.retries_left = Some(retries);
        self
    }

    pub fn status(mut self, status: QueueStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn initial_size(mut self, size: usize) -> Self {
        self.initial_size = Some(size);
        self
    }
}

impl<T: RateLimiter, TR: TaskRequestMPMCReceiver> Builder for SyncTaskQueueBuilder<T, TR> {
    type Item = SyncTaskQueue<T, TR>;

    fn new() -> Self {
        SyncTaskQueueBuilder {
            sync_plan_id: None,
            tasks: None,
            task_request_receiver: None,
            rate_limiter: None,
            max_retry: None,
            retries_left: None,
            status: None,
            initial_size: None,
        }
    }

    fn build(self) -> Self::Item {
        SyncTaskQueue {
            sync_plan_id: self.sync_plan_id.unwrap_or_else(Uuid::new_v4),
            tasks: self.tasks.unwrap_or_else(VecDeque::new),
            task_request_receiver: self
                .task_request_receiver
                .expect("Object implemented the TaskRequestMPMCReceiver trait is required"),
            rate_limiter: self.rate_limiter,
            max_retry: self.max_retry,
            retries_left: self.retries_left,
            status: self.status.unwrap_or(QueueStatus::default()),
            initial_size: self.initial_size.unwrap_or(0),
        }
    }
}

pub struct TaskManagerBuilder<T, MT, TR, ES, MF>
where
    T: RateLimiter,
    MT: SyncTaskMPMCSender,
    TR: TaskRequestMPMCReceiver,
    ES: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    queues: Option<Arc<RwLock<HashMap<QueueId, Arc<Mutex<SyncTaskQueue<T, TR>>>>>>>,
    task_sender: Option<MT>,
    error_message_channel: Option<ES>,
    failed_task_channel: Option<MF>,
    current_state: Option<TaskManagerState>,
}

impl<T, MT, TR, ES, MF> TaskManagerBuilder<T, MT, TR, ES, MF>
where
    T: RateLimiter,
    MT: SyncTaskMPMCSender,
    TR: TaskRequestMPMCReceiver,
    ES: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    pub fn new() -> Self {
        TaskManagerBuilder {
            queues: None,
            task_sender: None,
            error_message_channel: None,
            failed_task_channel: None,
            current_state: None,
        }
    }

    pub fn queues(mut self, queues: HashMap<QueueId, Arc<Mutex<SyncTaskQueue<T, TR>>>>) -> Self {
        self.queues = Some(Arc::new(RwLock::new(queues)));
        self
    }

    pub fn task_sender(mut self, sender: MT) -> Self {
        self.task_sender = Some(sender);
        self
    }

    pub fn error_message_channel(mut self, channel: ES) -> Self {
        self.error_message_channel = Some(channel);
        self
    }

    pub fn failed_task_channel(mut self, channel: MF) -> Self {
        self.failed_task_channel = Some(channel);
        self
    }

    pub fn current_state(mut self, state: TaskManagerState) -> Self {
        self.current_state = Some(state);
        self
    }

    pub fn build(self) -> TaskManager<T, MT, TR, ES, MF> {
        TaskManager {
            queues: self.queues.expect("queues must be set"),
            task_sender: self.task_sender.expect("task_sender must be set"),
            error_message_channel: self
                .error_message_channel
                .expect("error_message_channel must be set"),
            failed_task_channel: self
                .failed_task_channel
                .expect("failed_task_channel must be set"),
            current_state: self.current_state.expect("current_state must be set"),
        }
    }
}

impl<T, MT, TR, ES, MF> Builder for TaskManagerBuilder<T, MT, TR, ES, MF>
where
    T: RateLimiter,
    MT: SyncTaskMPMCSender,
    TR: TaskRequestMPMCReceiver,
    ES: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    type Item = TaskManager<T, MT, TR, ES, MF>;

    fn new() -> Self {
        TaskManagerBuilder {
            queues: None,
            task_sender: None,
            error_message_channel: None,
            failed_task_channel: None,
            current_state: None,
        }
    }

    fn build(self) -> Self::Item {
        TaskManager {
            queues: self.queues.expect("queues must be set"),
            task_sender: self.task_sender.expect("task_sender must be set"),
            error_message_channel: self
                .error_message_channel
                .expect("error_message_channel must be set"),
            failed_task_channel: self
                .failed_task_channel
                .expect("failed_task_channel must be set"),
            current_state: self.current_state.expect("current_state must be set"),
        }
    }
}
