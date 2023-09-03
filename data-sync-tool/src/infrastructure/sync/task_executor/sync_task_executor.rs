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
use crate::{domain::synchronization::{task_executor::{TaskExecutor, TaskExecutorError, SyncProgress}, sync_plan::SyncPlan, rate_limiter::RateLimiter, sync_task::SyncTask}, infrastructure::{sync::{workers::{worker_traits::{LongRunningWorker, ShortRunningWorker}, worker::{WebAPISyncWorker, WebsocketSyncWorker}}, task_manager::{tm_traits::SyncTaskManager, sync_task_queue::SyncTaskQueue, factory::{SyncTaskQueueBuilder, new_web_request_limiter}, sync_rate_limiter::WebRequestRateLimiter, task_manager::TaskManager}, shared_traits::TaskRequestMPMCReceiver, factory::Builder, GetTaskRequest}, mq::{factory::create_tokio_broadcasting_channel, tokio_channel_mq::{TokioBroadcastingMessageBusReceiver, TokioBroadcastingMessageBusSender, TokioMpscMessageBusSender}}}};
use std::{sync::Arc, collections::VecDeque};
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


#[derive(Derivative, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub")]
pub struct SyncTaskExecutor {
    idle_long_running_workers: HashMap<Uuid, WebAPISyncWorker<
        TokioMpscMessageBusSender<GetTaskRequest>,
        TokioBroadcastingMessageBusReceiver<SyncTask>,
        TokioBroadcastingMessageBusSender<SyncTask>,
        TokioBroadcastingMessageBusSender<SyncTask>,
    >>,
    busy_long_running_workers: HashMap<Uuid, WebsocketSyncWorker<
        TokioMpscMessageBusSender<GetTaskRequest>,
        
    >>,
    idle_short_task_handling_workers: HashMap<Uuid, SW>,
    busy_short_task_handling_workers: HashMap<Uuid, SW>,
    task_manager: Arc<Mutex<
        TaskManager<
            SyncTaskQueue<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>,
            TokioBroadcastingMessageBusSender<Sync>>>>,
    // worker_channels: WorkerChannels,
    // task_manager_channels: TaskManagerChannels,
}

impl<LW, SW, TM> SyncTaskExecutor<LW, SW, TM>
where
    LW: LongRunningWorker,
    SW: ShortRunningWorker,
    TM: SyncTaskManager
{
    pub fn new() -> Self {
        todo!()
    }

    pub fn builder() -> Self {
        todo!()
    }

}

#[async_trait]
impl<LW, SW, TM> TaskExecutor for SyncTaskExecutor<LW, SW, TM> 
where
    LW: LongRunningWorker,
    SW: ShortRunningWorker,
    TM: SyncTaskManager
{
    // add new sync plans to synchronize
    async fn assign(&mut self, sync_plans: Vec<Arc<Mutex<SyncPlan>>>) -> Result<(), TaskExecutorError> {
        let task_manager_lock = self.task_manager.lock().await;
        let (task_sender, task_receiver) = create_tokio_broadcasting_channel::<GetTaskRequest>(200);
        let new_task_queues = sync_plans.iter().map(|p| {
            let plan_lock = p.blocking_lock();
            
            let task_queue_builder: SyncTaskQueueBuilder<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>> = SyncTaskQueueBuilder::new();
            
            match plan_lock.sync_config().sync_rate_quota() {
                Some(quota) => {
                    let rate_limiter = new_web_request_limiter(
                        *quota.max_line_per_request(), Some(*quota.daily_limit()), Some(*quota.cooldown_seconds()));
                    let _task_queue = plan_lock.tasks().iter().map(|t| {
                        Arc::new(Mutex::new(t.clone()))
                    }).collect::<VecDeque<_>>();
                    let new_task_queue = task_queue_builder
                        .initial_size(plan_lock.tasks().len())
                        .rate_limiter(rate_limiter)
                        .retries_left(*quota.max_retry())
                        .task_request_receiver(task_receiver.clone())
                        .max_retry(*quota.max_retry())
                        .sync_plan_id(*plan_lock.id())
                        .tasks(_task_queue)
                        .build();
                    return new_task_queue;
                },
                None => {
                    let _task_queue = plan_lock.tasks().iter().map(|t| {
                        Arc::new(Mutex::new(t.clone()))
                    }).collect::<VecDeque<_>>();
                    let new_task_queue = task_queue_builder
                        .initial_size(plan_lock.tasks().len())
                        .retries_left(10)
                        .task_request_receiver(task_receiver.clone())
                        .max_retry(10)
                        .sync_plan_id(*plan_lock.id())
                        .tasks(_task_queue)
                        .build();
                    return new_task_queue;
                }
            }
            
        }).collect::<Vec<_>>();
        task_manager_lock.load_sync_plans(sync_plans, new_task_queues);
        Ok(())
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
