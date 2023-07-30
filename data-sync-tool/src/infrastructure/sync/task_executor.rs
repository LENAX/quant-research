/// Sync Task Executor Implementation
/// Sync Task Executor is the core module that implements data synchronization coordination.
/// `SyncTaskExecutor` is the central coordination entity that manages the execution of synchronization tasks.
/// It consists of two pools of `Worker` objects, each responsible for executing tasks.
/// The execution of tasks is monitored by `SyncTaskExecutor` using message passing via tokio channels.
/// The `SyncTaskExecutor` starts workers, each of which tries to execute tasks by receiving them from the `DatasetQueue`, respecting the rate limit. When the execution is done, the `Worker` sends the result back to the `SyncTaskExecutor`, which then handles the result.
/// It checks for errors, and if any are found, it decides if the task needs to be retried or not based on the remaining retry count of the task.
/// The entire design is intended to be asynchronous, built around the `async/await` feature of Rust and the async runtime provided by Tokio, making the best use of system resources and providing high throughput.

use std::sync::Arc;
use tokio::sync::Mutex;
use derivative::Derivative;
use getset::{Getters, Setters};
use serde_json::Value;
use crate::{
    application::synchronization::dtos::task_manager::CreateTaskManagerRequest,
    domain::synchronization::{rate_limiter::RateLimiter, sync_task::SyncTask}, infrastructure::mq::{factory::{get_tokio_mq_factory, TokioMQFactory}, tokio_channel_mq::{TokioMpscMessageBusReceiver, TokioSpmcMessageBusReceiver, TokioSpmcMessageBusSender}},
};

use super::{
    task_manager::{SyncTaskManager, TaskManagerError, QueueId, create_sync_task_manager},
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
    n_long_running_workers: usize,
    n_short_running_workers: usize,
    create_tm_request: CreateTaskManagerRequest
) -> Option<(
    SyncTaskExecutor<LW, SW, TM>,
    TokioMpscMessageBusReceiver<SyncTask>,
    TokioMpscMessageBusReceiver<TaskManagerError>,
    TokioSpmcMessageBusSender<(QueueId, SyncTask)>
)> {
    let task_channel_factory = get_tokio_mq_factory::<SyncTask>(*create_tm_request.task_channel_mq_type());
    let error_channel_factory = get_tokio_mq_factory::<TaskManagerError>(*create_tm_request.error_channel_mq_type());
    let failed_task_channel_factory = get_tokio_mq_factory::<(QueueId, SyncTask)>(*create_tm_request.failed_task_mq_type());

    if let TokioMQFactory::MpscFactory(tc_factory) = task_channel_factory {
        if let TokioMQFactory::MpscFactory(ec_factory) = error_channel_factory {
            if let TokioMQFactory::SpmcFactory(ft_factory) = failed_task_channel_factory {
                let (task_sender,
                    mut task_receiver) = tc_factory::<SyncTask>(*create_tm_request.task_channel_capacity());
       
                let (error_msg_sender,
                    mut error_msg_receiver) = ec_factory::<TaskManagerError>(*create_tm_request.error_channel_capcity());
       
                let (failed_task_sender, 
                    failed_task_receiver) = ft_factory::<(Uuid, SyncTask)>(*create_tm_request.failed_task_channel_capacity());
                let task_manager = create_sync_task_manager(&create_tm_request, task_sender, error_sender, failed_task_receiver);

                return ();
            }
        }
    } else {
        return None
    }

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
        
        if let 


        let (task_sender,
            mut task_receiver) = task_channel_factory(1000);

       let (error_msg_sender,
            mut error_msg_receiver) = create_tokio_mpsc_channel::<TaskManagerError>(500);

       let (failed_task_sender, 
           failed_task_receiver) = create_tokio_spmc_channel::<(Uuid, SyncTask)>(500);
  
    }
}
