/**
 * SyncEngine 同步任务执行管理模块
 * 用于支持数据同步任务的执行和运行状态管理。
 *
 */
use async_trait::async_trait;
use derivative::Derivative;
use futures::future::join_all;
use getset::{Getters, MutGetters, Setters};
use log::{error, info};
use crate::{
    domain::synchronization::{
        sync_plan::SyncPlan,
        sync_task::SyncTask,
        task_executor::{SyncProgress, TaskExecutor, TaskExecutorError, PlanProgress},
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
            shared_traits::StreamingData,
            task_manager::{
                errors::TaskManagerError,
                factory::{rate_limiter::WebRequestRateLimiterBuilder, task_queue::SyncTaskQueueBuilder},
                sync_rate_limiter::WebRequestRateLimiter,
                sync_task_queue::SyncTaskQueue,
                task_manager::TaskManager,
                tm_traits::{SyncTaskManager, TaskManagerCommand},
            },
            workers::{
                errors::SyncWorkerError,
                factory::{
                    http_worker_factory::{create_http_worker, WebAPISyncWorkerBuilder},
                    websocket_worker_factory::{create_websocket_worker, WebsocketSyncWorkerBuilder},
                },
                http_worker::WebAPISyncWorker,
                websocket_worker::WebsocketSyncWorker,
                worker_traits::{SyncWorker, WorkerCommand},
            },
            GetTaskRequest,
        },
    },
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

/**
 * Detailed Design
 *
 * SyncEngine uses a command-driven approach to orchestrate its submodules. When initialized, it
 * creates and allocates the submodules and resources. Then it can be called by sending commands or calling proxy methods.
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

// Consider a message-driven design to avoid overusing locks and to simplify design.

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SyncEngineState {
    #[derivative(Default)]
    Created, // default state after being created
    Ready, // becomes ready when being initialized or no task is assigned
    Running, // performing synchronization
    Stopped // turned to stopped state after calling stop, need to be reinitialized to get into the ready state
}

#[derive(Derivative, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub")]
pub struct SyncEngine {
    state: SyncEngineState,
    // scalable pool of http api synchronization workers
    http_api_sync_workers: HashMap<
        Uuid,
        WebAPISyncWorker<
            TokioBroadcastingMessageBusSender<GetTaskRequest>,
            TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
            TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
            TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
        >,
    >,
    // scalable pool of websocket synchronization workers
    websocket_streaming_workers: HashMap<
        Uuid,
        WebsocketSyncWorker<
            TokioBroadcastingMessageBusSender<GetTaskRequest>,
            TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
            TokioBroadcastingMessageBusSender<StreamingData>,
            TokioBroadcastingMessageBusSender<SyncWorkerError>,
        >,
    >,
    // task manager, responsible for managing task sending states and throttling
    task_manager: TaskManager<
        SyncTaskQueue<
            WebRequestRateLimiter,
            TokioBroadcastingMessageBusReceiver<GetTaskRequest>,
        >,
        TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
        TokioMpscMessageBusSender<TaskManagerError>,
        TokioSpmcMessageBusReceiver<Arc<Mutex<SyncTask>>>,
    >,
    // command senders
    task_manager_command_sender: TokioMpscMessageBusSender<TaskManagerCommand>,
    worker_command_sender: TokioSpmcMessageBusReceiver<WorkerCommand>,

    // collect completed task; expose an api to get completed tasks
    completed_task_receiver: TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
    streaming_data_receiver: TokioBroadcastingMessageBusReceiver<StreamingData>,
    failed_task_receiver: TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>,
    worker_error_receiver: TokioBroadcastingMessageBusReceiver<SyncWorkerError>
}

impl SyncEngine {
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
        todo!()
    }
}

#[async_trait]
impl TaskExecutor for SyncEngine {
    type TaskManagerType = TaskManager<
        SyncTaskQueue<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>,
        TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
        TokioMpscMessageBusSender<TaskManagerError>,
        TokioSpmcMessageBusReceiver<Arc<Mutex<SyncTask>>>,
    >;
    type TaskQueueType =
        SyncTaskQueue<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>;

    // Move the common bounds to associated types to declutter the method signature
    type RateLimiter = WebRequestRateLimiter;
    type RateLimiterBuilder = WebRequestRateLimiterBuilder;

    type QueueBuilder = SyncTaskQueueBuilder<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>;
    
    type CompletedTaskReceiverType = TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>;
    type StreamingDataReceiverType = TokioBroadcastingMessageBusReceiver<StreamingData>;
    type FailedTaskReceiverType = TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>;
    type WorkerErrorReceiverType = TokioBroadcastingMessageBusReceiver<SyncWorkerError>;
    type ProgressReceiverType = TokioSpmcMessageBusReceiver<PlanProgress>;
    
    /// Initialize TaskExecutor and wait for commands
    /// This method must be called before using any TaskExecutor implementation as it allocates necessary resources to run
    /// 
    /// Detailed Design:
    /// 
    async fn init(&'static mut self,) -> Result<(), TaskExecutorError> {
        todo!()
    }

    // Deallocate resources and shutdown TaskExecutor
    async fn shutdown(&mut self) -> Result<(), TaskExecutorError> {
        todo!()
    }

    // add new sync plans to synchronize
    async fn assign(
        &mut self,
        sync_plans: Vec<Arc<RwLock<SyncPlan>>>,
    ) -> Result<(), TaskExecutorError> {
        todo!()
    }

    // wait and continuously get completed task
    fn subscribe_completed_task(&mut self) -> Self::CompletedTaskReceiverType  {
        todo!()
    }

    // wait and continuously get streaming data
    fn subscribe_streaming_data(&mut self) -> Self::StreamingDataReceiverType  {
        todo!()
    }

    fn subscribe_failed_task(&mut self) -> Self::FailedTaskReceiverType  {
        todo!()
    }

    fn subscribe_worker_error(&mut self) -> Self::WorkerErrorReceiverType  {
        todo!()
    }

    fn subscribe_progress(&mut self) -> Self::ProgressReceiverType  {
        todo!()
    }

    // run a single plan. Either start a new plan or continue a paused plan
    async fn run(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError>  {
        todo!()
    }

    // run all assigned plans
    async fn run_all(&'static mut self) -> Result<(), TaskExecutorError>  {
        todo!()
    }

    // temporarily pause a plan
    async fn pause(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError> {
        todo!()
    }

    // pause all plans
    async fn pause_all(&mut self) -> Result<(), TaskExecutorError>  {
        todo!()
    }

    // cancel sync for plan, also removes it from the executor
    async fn cancel(&mut self, sync_plan_ids: Vec<Uuid>) -> Result<(), TaskExecutorError>  {
        todo!()
    }

    // cancel and drop all plans
    async fn cancel_all(&mut self) -> Result<(), TaskExecutorError>  {
        todo!()
    }
}
