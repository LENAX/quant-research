/// Sync Task Executor Implementation
/// Sync Task Executor is the core module that implements data synchronization coordination.
/// `SyncTaskExecutor` is the central coordination entity that manages the execution of synchronization tasks.
/// It consists of two pools of `Worker` objects, each responsible for executing tasks.
/// The execution of tasks is monitored by `SyncTaskExecutor` using message passing via tokio channels.
/// The `SyncTaskExecutor` starts workers, each of which tries to execute tasks by receiving them from the `DatasetQueue`, respecting the rate limit. When the execution is done, the `Worker` sends the result back to the `SyncTaskExecutor`, which then handles the result.
/// It checks for errors, and if any are found, it decides if the task needs to be retried or not based on the remaining retry count of the task.
/// The entire design is intended to be asynchronous, built around the `async/await` feature of Rust and the async runtime provided by Tokio, making the best use of system resources and providing high throughput.

use crate::{
    application::synchronization::dtos::task_manager::CreateTaskManagerRequest,
    domain::synchronization::{rate_limiter::RateLimiter, sync_task::SyncTask},
    infrastructure::mq::{
        factory::{get_tokio_mq_factory, TokioMQFactory},
        message_bus::{MessageBusSender, StaticClonableMpscMQ},
        tokio_channel_mq::{
            TokioMpscMessageBusReceiver, TokioMpscMessageBusSender, TokioSpmcMessageBusReceiver,
            TokioSpmcMessageBusSender,
        },
    },
};
use derivative::Derivative;
use getset::{Getters, Setters};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{
    sync_rate_limiter::WebRequestRateLimiter,
    task_manager::{
        create_sync_task_manager, QueueId, SyncTaskManager, TaskManager, TaskManagerError,
    },
    worker::{LongRunningWorker, ShortTaskHandlingWorker, SyncWorker},
};

#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct SyncTaskExecutor<LW, SW, TM> {
    long_running_workers: Vec<LW>,
    short_task_handling_workers: Vec<SW>,
    task_manager: TM,
}

fn init_task_executor<LW, SW, TM>(
    long_running_workers: Vec<LW>,
    short_running_workers: Vec<SW>,
    task_sender: TokioMpscMessageBusSender<SyncTask>,
    error_sender: TokioMpscMessageBusSender<TaskManagerError>,
    failed_task_receiver: TokioSpmcMessageBusReceiver<(Uuid, SyncTask)>,
    create_tm_request: &CreateTaskManagerRequest,
) -> SyncTaskExecutor<
    LW,
    SW,
    TaskManager<
        WebRequestRateLimiter,
        TokioMpscMessageBusSender<SyncTask>,
        TokioMpscMessageBusSender<TaskManagerError>,
        TokioSpmcMessageBusReceiver<(Uuid, SyncTask)>,
    >,
> {
    let task_manager = create_sync_task_manager(
        create_tm_request,
        task_sender,
        error_sender,
        failed_task_receiver,
    );
    let task_executor: SyncTaskExecutor<
        LW,
        SW,
        TaskManager<
            super::sync_rate_limiter::WebRequestRateLimiter,
            TokioMpscMessageBusSender<SyncTask>,
            TokioMpscMessageBusSender<TaskManagerError>,
            TokioSpmcMessageBusReceiver<(Uuid, SyncTask)>,
        >,
    > = SyncTaskExecutor {
        long_running_workers,
        short_task_handling_workers: short_running_workers,
        task_manager,
    };
    return task_executor;
}

impl<LW, SW, TM> SyncTaskExecutor<LW, SW, TM>
where
    LW: SyncWorker + LongRunningWorker,
    SW: SyncWorker + ShortTaskHandlingWorker,
    TM: SyncTaskManager,
{
    pub fn new(
        n_long_running_workers: usize,
        n_short_running_workers: usize,
        create_tm_request: CreateTaskManagerRequest,
    ) -> Self {
        todo!()
    }
}
