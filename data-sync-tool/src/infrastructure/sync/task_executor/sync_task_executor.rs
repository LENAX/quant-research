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
use crate::{
    domain::synchronization::{
        rate_limiter::RateLimiter,
        sync_plan::SyncPlan,
        sync_task::SyncTask,
        task_executor::{SyncProgress, TaskExecutor, TaskExecutorError},
        value_objects::sync_config::SyncMode,
    },
    infrastructure::{
        mq::{
            factory::create_tokio_broadcasting_channel,
            tokio_channel_mq::{
                TokioBroadcastingMessageBusReceiver, TokioBroadcastingMessageBusSender,
                TokioMpscMessageBusSender, TokioSpmcMessageBusReceiver,
            },
        },
        sync::{
            factory::Builder,
            shared_traits::StreamingData,
            task_manager::{
                errors::TaskManagerError,
                factory::{rate_limiter::RateLimiterBuilder, task_queue::TaskQueueBuilder},
                sync_rate_limiter::WebRequestRateLimiter,
                sync_task_queue::SyncTaskQueue,
                task_manager::TaskManager,
                task_queue::TaskQueue,
                tm_traits::SyncTaskManager,
            },
            workers::{
                errors::SyncWorkerError,
                factory::{
                    http_worker_factory::{create_http_worker, WebAPISyncWorkerBuilder},
                    websocket_worker_factory::{create_websocket_worker, WebsocketSyncWorkerBuilder},
                },
                http_worker::WebAPISyncWorker,
                websocket_worker::{self, WebsocketSyncWorker},
                worker_traits::{LongRunningWorker, ShortRunningWorker},
            },
            GetTaskRequest,
        },
    },
};
use std::collections::HashMap;
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

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
    http_api_sync_workers: HashMap<
        Uuid,
        WebAPISyncWorker<
            TokioBroadcastingMessageBusSender<GetTaskRequest>,
            TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
            TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
            TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
        >,
    >,
    websocket_streaming_workers: HashMap<
        Uuid,
        WebsocketSyncWorker<
            TokioBroadcastingMessageBusSender<GetTaskRequest>,
            TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
            TokioBroadcastingMessageBusSender<StreamingData>,
            TokioBroadcastingMessageBusSender<SyncWorkerError>,
        >,
    >,
    task_manager: Arc<
        Mutex<
            TaskManager<
                SyncTaskQueue<
                    WebRequestRateLimiter,
                    TokioBroadcastingMessageBusReceiver<GetTaskRequest>,
                >,
                TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
                TokioMpscMessageBusSender<TaskManagerError>,
                TokioSpmcMessageBusReceiver<Arc<Mutex<SyncTask>>>,
            >,
        >,
    >,
    // collect completed task; expose an api to get completed tasks
    completed_task_receiver: TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
    streaming_data_receiver: TokioBroadcastingMessageBusReceiver<StreamingData>,
    failed_task_receiver: TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
    worker_error_receiver: TokioBroadcastingMessageBusReceiver<SyncWorkerError>
}

impl SyncTaskExecutor {
    pub fn new() -> Self {
        todo!()
    }

    pub fn builder() -> Self {
        todo!()
    }

    // for simplicity, use tokio channel
    // may update to generic method to support multiple channel implementation
    fn allocate_task_request_channels(
        &self,
        n_sync_plans: usize,
        channel_size: Option<usize>,
    ) -> (
        Vec<TokioBroadcastingMessageBusSender<GetTaskRequest>>,
        Vec<TokioBroadcastingMessageBusReceiver<GetTaskRequest>>,
    ) {
        let (task_sender, task_receiver) =
            create_tokio_broadcasting_channel::<GetTaskRequest>(channel_size.unwrap_or(200));

        let mut task_senders = Vec::with_capacity(n_sync_plans);
        let mut task_receivers = Vec::with_capacity(n_sync_plans);

        for _ in 0..n_sync_plans {
            task_senders.push(task_sender.clone());
            task_receivers.push(task_receiver.clone());
        }

        (task_senders, task_receivers)
    }
}

#[async_trait]
impl TaskExecutor for SyncTaskExecutor {
    type TaskManagerType = TaskManager<
        SyncTaskQueue<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>,
        TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
        TokioMpscMessageBusSender<TaskManagerError>,
        TokioSpmcMessageBusReceiver<Arc<Mutex<SyncTask>>>,
    >;
    type TaskQueueType =
        SyncTaskQueue<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>;

    // add new sync plans to synchronize
    async fn assign(
        &mut self,
        sync_plans: Vec<Arc<RwLock<SyncPlan>>>,
    ) -> Result<(), TaskExecutorError>
    where
        <WebRequestRateLimiter as RateLimiter>::BuilderType:
            Builder<Product = WebRequestRateLimiter> + RateLimiterBuilder + Send,
        <Self::TaskQueueType as TaskQueue>::BuilderType: Builder<Product = Self::TaskQueueType>
            + TaskQueueBuilder<
                RateLimiterType = WebRequestRateLimiter,
                TaskRequestReceiverType = TokioBroadcastingMessageBusReceiver<GetTaskRequest>,
            > + Send,
        <Self::TaskManagerType as SyncTaskManager>::TaskQueueType: TaskQueue,
        <Self as TaskExecutor>::TaskManagerType: SyncTaskManager,
        <Self as TaskExecutor>::TaskQueueType: TaskQueue,
    {
        let (task_senders, task_receivers) =
            self.allocate_task_request_channels(sync_plans.len(), None);
        let (todo_task_sender, todo_task_receiver) =
            create_tokio_broadcasting_channel::<Arc<Mutex<SyncTask>>>(200);
        
        // Who should receive the completed task? Of the caller of sync task executor
        let (completed_task_sender, completed_task_receiver) =
                create_tokio_broadcasting_channel::<Arc<Mutex<SyncTask>>>(200);
        let (completed_streaming_data_sender, completed_streaming_data_receiver) =
            create_tokio_broadcasting_channel::<StreamingData>(200);
        let (failed_task_sender, failed_task_receiver) =
            create_tokio_broadcasting_channel::<Arc<Mutex<SyncTask>>>(200);
        let (worker_error_sender, worker_error_receiver) =
            create_tokio_broadcasting_channel::<SyncWorkerError>(200);

        // Lock the task manager and load the sync plans
        let mut task_manager_lock = self.task_manager.lock().await;
        // FIXME: Channels between workers, queues, and task manager do not match
        task_manager_lock.set_task_sender(todo_task_sender);

        // task_manager_lock.set_failed_task_channel(failed_task_receiver);
        self.completed_task_receiver = completed_task_receiver;
        self.streaming_data_receiver = completed_streaming_data_receiver;
        self.failed_task_receiver = failed_task_receiver;
        self.worker_error_receiver = worker_error_receiver;


        for (i, sync_plan) in sync_plans.iter().enumerate() {
            // where should I place the other end of channels
            let plan_lock = sync_plan.read().await;
            match plan_lock.sync_config().sync_mode() {
                SyncMode::HttpAPI => {
                    // Fixme: fill the correct type
                    let http_worker: WebAPISyncWorker<
                        TokioBroadcastingMessageBusSender<GetTaskRequest>,
                        TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
                        TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
                        TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
                    > = create_http_worker::<
                        WebAPISyncWorkerBuilder<
                            TokioBroadcastingMessageBusSender<GetTaskRequest>,
                            TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
                            TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
                            TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
                        >,
                        TokioBroadcastingMessageBusSender<GetTaskRequest>,
                        TokioBroadcastingMessageBusReceiver<Arc<tokio::sync::Mutex<SyncTask>>>,
                        TokioBroadcastingMessageBusSender<Arc<tokio::sync::Mutex<SyncTask>>>,
                        TokioBroadcastingMessageBusSender<Arc<tokio::sync::Mutex<SyncTask>>>,
                    >(
                        task_senders[i].clone(),
                        todo_task_receiver.clone(),
                        completed_task_sender.clone(),
                        failed_task_sender.clone(),
                    );
                    self.http_api_sync_workers
                        .insert(*http_worker.id(), http_worker);
                }
                SyncMode::WebsocketStreaming => {
                    let websocket_worker: WebsocketSyncWorker<
                        TokioBroadcastingMessageBusSender<GetTaskRequest>,
                        TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
                        TokioBroadcastingMessageBusSender<StreamingData>,
                        TokioBroadcastingMessageBusSender<SyncWorkerError>,
                    > = create_websocket_worker::<
                        WebsocketSyncWorkerBuilder<
                            TokioBroadcastingMessageBusSender<GetTaskRequest>,
                            TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
                            TokioBroadcastingMessageBusSender<StreamingData>,
                            TokioBroadcastingMessageBusSender<SyncWorkerError>>,
                        TokioBroadcastingMessageBusSender<GetTaskRequest>,
                        TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
                        TokioBroadcastingMessageBusSender<StreamingData>,
                        TokioBroadcastingMessageBusSender<SyncWorkerError>,
                    >(
                        task_senders[i].clone(),
                        todo_task_receiver.clone(),
                        completed_streaming_data_sender.clone(),
                        worker_error_sender.clone(),
                    );
                    self.websocket_streaming_workers
                        .insert(*websocket_worker.id(), websocket_worker);
                }
            }
        }

        match task_manager_lock.load_sync_plans::<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>(sync_plans, task_receivers).await {
            Ok(_) => Ok(()),
            Err(e) => { Err(TaskExecutorError::LoadPlanFailure) }, // Convert the error type if needed
        }

        // Allocate workers for each sync plan
    }

    // wait and continuously get completed task
    // should receive data after calling run in another thread
    async fn wait_and_get_completed_task(&mut self) -> Result<Arc<Mutex<SyncTask>>, TaskExecutorError> {
        todo!()
    }

    // wait and continuously get streaming data
    // should receive data after calling run in another thread
    async fn wait_and_get_streaming_data(&mut self) -> Result<StreamingData, TaskExecutorError> {
        todo!()
    }

    async fn wait_and_get_failed_task(&mut self) -> Result<Arc<Mutex<SyncTask>>, TaskExecutorError> {
        todo!()
    }

    async fn wait_and_get_worker_error(&mut self) -> Result<SyncWorkerError, TaskExecutorError> {
        todo!()
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
