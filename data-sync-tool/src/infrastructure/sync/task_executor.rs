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
        factory::{
            create_tokio_broadcasting_channel, create_tokio_mpsc_channel,
            create_tokio_spmc_channel, get_tokio_mq_factory, TokioMQFactory,
        },
        message_bus::{MessageBusSender, StaticClonableMpscMQ},
        tokio_channel_mq::{
            TokioBroadcastingMessageBusReceiver, TokioBroadcastingMessageBusSender,
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
    worker::{
        create_web_api_sync_workers, create_websocket_sync_workers, LongRunningWorker,
        ShortTaskHandlingWorker, SyncWorker, SyncWorkerData, SyncWorkerErrorMessage,
        SyncWorkerMessage, WebAPISyncWorker, WebsocketSyncWorker,
    },
};

type TokioExecutorChannels = (
    TokioMpscMessageBusReceiver<SyncWorkerData>,
    TokioBroadcastingMessageBusSender<SyncWorkerMessage>,
    TokioMpscMessageBusReceiver<SyncWorkerErrorMessage>,
    TokioMpscMessageBusReceiver<SyncTask>,
    TokioMpscMessageBusReceiver<TaskManagerError>,
    TokioSpmcMessageBusSender<(Uuid, SyncTask)>,
);

pub fn create_tokio_task_executor(
    n_workers: usize,
    channel_size: usize,
    create_tm_request: &CreateTaskManagerRequest,
) -> (
    SyncTaskExecutor<
        WebsocketSyncWorker<
            TokioMpscMessageBusSender<SyncWorkerData>,
            TokioBroadcastingMessageBusReceiver<SyncWorkerMessage>,
            TokioMpscMessageBusSender<SyncWorkerErrorMessage>,
        >,
        WebAPISyncWorker,
        TaskManager<
            WebRequestRateLimiter,
            TokioMpscMessageBusSender<SyncTask>,
            TokioMpscMessageBusSender<TaskManagerError>,
            TokioSpmcMessageBusReceiver<(Uuid, SyncTask)>,
        >,
    >,
    TokioExecutorChannels,
) {
    // create message bus channels
    let (task_sender, task_receiver) = create_tokio_mpsc_channel::<SyncTask>(channel_size);
    let (error_sender, error_receiver) =
        create_tokio_mpsc_channel::<TaskManagerError>(channel_size);
    let (failed_task_sender, failed_task_receiver) =
        create_tokio_spmc_channel::<(Uuid, SyncTask)>(channel_size);
    let (sync_worker_message_sender, sync_worker_message_receiver) =
        create_tokio_broadcasting_channel::<SyncWorkerMessage>(channel_size);
    let (sync_worker_data_sender, sync_worker_data_receiver) =
        create_tokio_mpsc_channel::<SyncWorkerData>(channel_size);
    let (sync_worker_error_sender, sync_worker_error_receiver) =
        create_tokio_mpsc_channel::<SyncWorkerErrorMessage>(channel_size);

    // create workers
    let long_running_workers = create_websocket_sync_workers(
        n_workers,
        sync_worker_data_sender.clone(),
        sync_worker_message_receiver.clone(),
        sync_worker_error_sender.clone(),
    );
    let short_running_workers = create_web_api_sync_workers(n_workers);

    // create task manager
    let task_manager = create_sync_task_manager(
        create_tm_request,
        task_sender,
        error_sender,
        failed_task_receiver,
    );

    // create task executor
    let task_executor = SyncTaskExecutor {
        long_running_workers,
        short_task_handling_workers: short_running_workers,
        task_manager,
    };

    (
        task_executor,
        (
            sync_worker_data_receiver,
            sync_worker_message_sender,
            sync_worker_error_receiver,
            task_receiver,
            error_receiver,
            failed_task_sender,
        ),
    )
}

#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct SyncTaskExecutor<LW, SW, TM> {
    long_running_workers: Vec<LW>,
    short_task_handling_workers: Vec<SW>,
    task_manager: TM,
}
