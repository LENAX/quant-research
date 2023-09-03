use std::collections::VecDeque;

use crate::application::synchronization::dtos::task_manager::CreateRateLimiterRequest;
use crate::domain::synchronization::value_objects::sync_config::{RateLimiterImpls, RateQuota};
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

use super::sync_rate_limiter::WebRequestRateLimiter;
use super::task_queue::TaskQueue;
use super::{
    sync_task_queue::{QueueStatus, SyncTaskQueue},
    task_manager::TaskManager,
    tm_traits::TaskManagerErrorMPSCSender,
};

/**
 * Factory methods and traits for the task manager module
 */

// Factory Methods
// pub fn new_empty_queue<T: RateLimiter, TR: TaskRequestMPMCReceiver>(
//     rate_limiter: T,
//     task_request_receiver: TR,
//     max_retry: Option<u32>,
//     sync_plan_id: Uuid,
// ) -> TQ {
//     return SyncTaskQueue::<T, TR>::new(
//         vec![],
//         Some(rate_limiter),
//         max_retry,
//         sync_plan_id,
//         task_request_receiver,
//     );
// }

pub fn new_empty_limitless_queue<T: RateLimiter, TR: TaskRequestMPMCReceiver>(
    max_retry: Option<u32>,
    sync_plan_id: Uuid,
    task_request_receiver: TR,
) -> SyncTaskQueue<T, TR> {
    return SyncTaskQueue::new(vec![], None, max_retry, sync_plan_id, task_request_receiver);
}

pub fn new_web_request_limiter(
    max_request: u32,
    max_daily_request: Option<u32>,
    cooldown: Option<u32>,
) -> WebRequestRateLimiter {
    return WebRequestRateLimiter::new(max_request, max_daily_request, cooldown).unwrap();
}

pub fn create_rate_limiter(
    create_limiter_req: &CreateRateLimiterRequest,
    limiter_type: RateLimiterImpls,
) -> Box<dyn RateLimiter> {
    match limiter_type {
        RateLimiterImpls::WebRequestRateLimiter => Box::new(new_web_request_limiter(
            *create_limiter_req.max_request(),
            *create_limiter_req.max_daily_request(),
            *create_limiter_req.cooldown(),
        )),
        // handle other LimiterTypes here
    }
}

pub enum RateLimiterInstance {
    WebLimiter(WebRequestRateLimiter)
}

pub fn create_rate_limiter_by_rate_quota(
    rate_quota: &RateQuota,
) -> RateLimiterInstance {
    match rate_quota.use_impl() {
        RateLimiterImpls::WebRequestRateLimiter => RateLimiterInstance::WebLimiter(new_web_request_limiter(
            *rate_quota.max_line_per_request(),
            Some(*rate_quota.daily_limit()),
            Some(*rate_quota.cooldown_seconds()),
        )),
        // handle other LimiterTypes here
    }
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
            status: Some(QueueStatus::default()),
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

pub struct TaskManagerBuilder<TQ, MT, ES, MF>
where
    TQ: TaskQueue,
    MT: SyncTaskMPMCSender,
    ES: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    queues: Option<Arc<RwLock<HashMap<QueueId, Arc<Mutex<TQ>>>>>>,
    task_sender: Option<MT>,
    error_message_channel: Option<ES>,
    failed_task_channel: Option<MF>,
    current_state: Option<TaskManagerState>,
}

impl<TQ, MT, ES, MF> TaskManagerBuilder<TQ, MT, ES, MF>
where
    TQ: TaskQueue,
    MT: SyncTaskMPMCSender,
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

    pub fn queues(mut self, queues: HashMap<QueueId, Arc<Mutex<TQ>>>) -> Self {
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

    pub fn build(self) -> TaskManager<TQ, MT, ES, MF> {
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

impl<TQ, MT, ES, MF>Builder for TaskManagerBuilder<TQ, MT, ES, MF>
where
    TQ: TaskQueue,
    MT: SyncTaskMPMCSender,
    ES: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    type Item = TaskManager<TQ, MT, ES, MF>;

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
