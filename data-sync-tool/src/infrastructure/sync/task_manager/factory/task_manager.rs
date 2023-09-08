use getset::{Getters, MutGetters, Setters};

use crate::infrastructure::sync::{
    factory::Builder,
    shared_traits::{FailedTaskSPMCReceiver, SyncTaskMPMCSender},
    task_manager::{
        task_queue::TaskQueue,
        tm_traits::{SyncTaskManager, TaskManagerErrorMPSCSender},
    },
};
use std::sync::Arc;
use tokio::sync::RwLock;
/**
 * Task Manager Builders and Factories
 */

pub fn create_task_manager<
    TMB: TMBuilder<TQ, MT, ES, MF>,
    TQ: TaskQueue,
    MT: SyncTaskMPMCSender,
    ES: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
>(
    task_queues: Vec<TQ>,
    task_sender: MT,
    error_sender: ES,
    failed_task_sender: MF,
) -> TMB::Product
where
    TMB::Product: SyncTaskManager,
{
    let builder = TMB::new();
    builder
        .with_task_queues(task_queues)
        .with_task_sender(task_sender)
        .with_error_sender(error_sender)
        .with_failed_task_sender(failed_task_sender)
        .build()
}

pub trait TMBuilder<TQ, MT, ES, MF>: Builder
where
    TQ: TaskQueue,
    MT: SyncTaskMPMCSender,
    ES: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    fn with_task_queues(self, queues: Vec<TQ>) -> Self;
    fn with_task_sender(self, task_sender: MT) -> Self;
    fn with_error_sender(self, error_sender: ES) -> Self;
    fn with_failed_task_sender(self, failed_task_sender: MF) -> Self;
}

#[derive(Debug, MutGetters, Getters, Setters)]
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

impl<TQ, MT, ES, MF> Builder for TaskManagerBuilder<TQ, MT, ES, MF>
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
