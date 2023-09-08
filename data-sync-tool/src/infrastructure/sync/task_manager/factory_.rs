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
    type Product = TaskManager<TQ, MT, ES, MF>;

    fn new() -> Self {
        TaskManagerBuilder {
            queues: None,
            task_sender: None,
            error_message_channel: None,
            failed_task_channel: None,
            current_state: None,
        }
    }

    fn build(self) -> Self::Product {
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
