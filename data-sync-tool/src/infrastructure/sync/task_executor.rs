use std::sync::Arc;

/// Sync Task Executor Implementation
/// Sync Task Executor is the core module that implements data synchronization coordination.
/// `SyncTaskExecutor` is the central coordination entity that manages the execution of synchronization tasks.
/// It consists of a pool of `Worker` objects, each responsible for executing tasks.
/// The execution of tasks is monitored by `SyncTaskExecutor` using message passing via tokio channels.
/// The `SyncTaskExecutor` starts workers, each of which tries to execute tasks by receiving them from the `DatasetQueue`, respecting the rate limit. When the execution is done, the `Worker` sends the result back to the `SyncTaskExecutor`, which then handles the result.
/// It checks for errors, and if any are found, it decides if the task needs to be retried or not based on the remaining retry count of the task.
/// The entire design is intended to be asynchronous, built around the `async/await` feature of Rust and the async runtime provided by Tokio, making the best use of system resources and providing high throughput.

use derivative::Derivative;
use getset::{Getters, Setters};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::domain::synchronization::rate_limiter::RateLimiter;

use super::{worker::{SyncWorker, LongRunningWorker, ShortTaskHandlingWorker}, task_manager::SyncTaskManager};

#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct SyncTaskExecutor<LW, SW, TM> {
    long_running_workers: Vec<Arc<Mutex<LW>>>,
    short_task_handling_workers: Vec<Arc<Mutex<SW>>>,
    task_manager: TM
}

/// Things to consider before writing implementation:
/// You have different types of workers, and they have different behaviors when handling sync tasks
/// 1. WebAPISyncWorker: Simply sends a web request and write result back the sync task
/// 2. WebsocketSyncWorker: Continuously get data from remote and send the result
/// The first question is how to initialize these workers?
/// We know that worker initialization depends on the request method contained in the task specification.
/// Yet we don't know the task specification until we get the task from task manager
/// We could use a dedicated worker pool, but for the sake of simplicity, we can implement a factory method
/// that creates a mixture of all types of workers.
impl<LW, SW, TM> SyncTaskExecutor<LW, SW, TM>
where
    LW: SyncWorker + LongRunningWorker,
    SW: SyncWorker + ShortTaskHandlingWorker,
    TM: SyncTaskManager
{
    
}