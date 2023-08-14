use async_trait::async_trait;
/**
 * SyncTaskExecutor 同步任务执行管理模块
 * 用于支持数据同步任务的执行和运行状态管理。
 *
 */
// use crate::{
//     application::synchronization::dtos::task_manager::CreateTaskManagerRequest,
//     domain::synchronization::sync_task::SyncTask,
//     infrastructure::mq::{
//         factory::{
//             create_tokio_broadcasting_channel, create_tokio_mpsc_channel, create_tokio_spmc_channel,
//         },
//         // message_bus::{MessageBusSender, StaticClonableMpscMQ},
//         tokio_channel_mq::{
//             TokioBroadcastingMessageBusReceiver, TokioBroadcastingMessageBusSender,
//             TokioMpscMessageBusReceiver, TokioMpscMessageBusSender, TokioSpmcMessageBusReceiver,
//             TokioSpmcMessageBusSender,
//         },
//     },
// };
use derivative::Derivative;
use getset::{Getters, MutGetters, Setters};
// use serde_json::Value;
use crate::{domain::synchronization::{task_executor::{TaskExecutor, TaskExecutorError, SyncProgress}, sync_plan::SyncPlan, rate_limiter::RateLimiter}, infrastructure::sync::{workers::worker_traits::{LongRunningWorker, ShortRunningWorker}, task_manager::tm_traits::SyncTaskManager, shared_traits::TaskRequestMPMCReceiver}};
use std::sync::Arc;
use tokio::sync::{RwLock, Mutex};
use uuid::Uuid;
use std::collections::HashMap;

/**
 * Detailed Design
 *
 * By default, each sync plan is handled by one worker, but you can add more workers to each plan as long as it does not exceeds the
 * concurrency level limit.
 *
 * Primary Work Procedures
 * 1. Initialization
 *    - create data communication channels for workers and the task manager
 *    - create workers and the task manager, wire them up with channels created in the previous step
 * 2. Sync plan assignment
 *    - populates sync plans to task manager and wait for synchronization to start
 *    - assign workers to sync plans
 * 3. Continuous Synchronization
 *    - Set task manager and workers to running state
 * 4. Sync state management
 *    - run: start or continue a plan, ask workers to fetch tasks from task manager
 *    - pause: ask assigned workers to pause on given plan
 *    - cancel: ask workers to stop syncing the plan, and ask task manager to drop that plan
 *    - progress report: summarize current progress and return it
 */

// #[async_trait]
// pub trait TaskExecutor {
//     // add new sync plans to synchronize
//     async fn assign(&mut self, sync_plans: Vec<SyncPlan>) -> Result<(), TaskExecutorError>;

//     // run a single plan. Either start a new plan or continue a paused plan
//     async fn run(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError>;

//     // run all assigned plans
//     async fn run_all(&mut self) -> Result<(), TaskExecutorError>;

//     // temporarily pause a plan
//     async fn pause(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError>;

//     // pause all plans
//     async fn pause_all(&mut self) -> Result<(), TaskExecutorError>;

//     // cancel sync for plan, also removes it from the executor
//     async fn cancel(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError>;

//     // cancel and drop all plans
//     async fn cancel_all(&mut self) -> Result<(), TaskExecutorError>;

//     // report current progress
//     async fn report_progress(&self) -> Result<SyncProgress, TaskExecutorError>;
// }

// Task Executor need to retain a copy of worker channels and task manager channels to
// coordinate their work
// #[derive(Derivative, Getters, Setters, MutGetters)]
// #[getset(get = "pub", set = "pub")]
// struct WorkerChannels {
//     worker_data_receiver: Arc<RwLock<Box<dyn SyncTaskStreamingDataMPSCReceiver>>>,
//     worker_message_sender: Arc<RwLock<Box<dyn SyncWorkerMessageMPMCSender>>>,
//     worker_error_receiver: Arc<RwLock<Box<dyn SyncWorkerErrorMessageMPSCReceiver>>>,
// }

// #[derive(Derivative, Getters, Setters, MutGetters)]
// #[getset(get = "pub", set = "pub")]
// struct TaskManagerChannels {
//     sync_task_receiver: Arc<RwLock<Box<dyn SyncTaskMPMCReceiver>>>,
//     task_request_sender: Arc<RwLock<Box<dyn TaskRequestMPMCSender>>>,
//     failed_task_sender: Arc<RwLock<Box<dyn FailedTaskSPMCSender>>>,
//     manager_error_receiver: Arc<RwLock<Box<dyn TaskManagerErrorMPSCReceiver>>>,
// }

#[derive(Derivative, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub")]
pub struct SyncTaskExecutor<LW, SW, T, TR> {
    idle_long_running_workers: HashMap<Uuid, LW>,
    busy_long_running_workers: HashMap<Uuid, LW>,
    idle_short_task_handling_workers: HashMap<Uuid, SW>,
    busy_short_task_handling_workers: HashMap<Uuid, SW>,
    task_manager: Arc<Mutex<dyn SyncTaskManager<T, TR>>>,
    // worker_channels: WorkerChannels,
    // task_manager_channels: TaskManagerChannels,
}

// TODO:
// 1. implement TaskExecutor trait
// 2. may need channels to coordinate workers and task manager ✔
// 3. How to populate tasks into task manager's queues and ensure all tasks of one dataset go to the same queue?
// 4. Additional features like progress reporting.

///

impl<LW, SW, T, TR> SyncTaskExecutor<LW, SW, T, TR>
where
    LW: LongRunningWorker,
    SW: ShortRunningWorker,
    T: RateLimiter,
    TR: TaskRequestMPMCReceiver
{
    pub fn new() -> Self {
        todo!()
    }

    pub fn builder() -> Self {
        todo!()
    }

}

#[async_trait]
impl<LW, SW, T, TR> TaskExecutor for SyncTaskExecutor<LW, SW, T, TR> 
where
    LW: LongRunningWorker,
    SW: ShortRunningWorker,
    T: RateLimiter,
    TR: TaskRequestMPMCReceiver
{
    // add new sync plans to synchronize
    async fn assign(&mut self, sync_plans: Vec<Arc<Mutex<SyncPlan>>>) -> Result<(), TaskExecutorError> {
        let task_manager_lock = self.task_manager.lock().await;
        task_manager_lock.load_sync_plans(sync_plans, rate_limiters, task_request_receivers)
    }

    // run a single plan. Either start a new plan or continue a paused plan
    async fn run(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError> {
        todo!()
    }

    // run all assigned plans
    async fn run_all(&mut self) -> Result<(), TaskExecutorError> {
        todo!()
    }

    // temporarily pause a plan
    async fn pause(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError> {
        todo!()
    }

    // pause all plans
    async fn pause_all(&mut self) -> Result<(), TaskExecutorError> {
        todo!()
    }

    // cancel sync for plan, also removes it from the executor
    async fn cancel(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError> {
        todo!()
    }

    // cancel and drop all plans
    async fn cancel_all(&mut self) -> Result<(), TaskExecutorError> {
        todo!()
    }

    // report current progress
    async fn report_progress(&self) -> Result<SyncProgress, TaskExecutorError> {
        todo!()
    }
}
