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
use futures::future::join_all;
use getset::{Getters, MutGetters, Setters};
use log::{error, info};
// use serde_json::Value;
use crate::{
    domain::synchronization::{
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
            shared_traits::StreamingData,
            task_manager::{
                errors::TaskManagerError,
                factory::{rate_limiter::WebRequestRateLimiterBuilder, task_queue::SyncTaskQueueBuilder},
                sync_rate_limiter::WebRequestRateLimiter,
                sync_task_queue::SyncTaskQueue,
                task_manager::TaskManager,
                tm_traits::SyncTaskManager,
            },
            workers::{
                errors::SyncWorkerError,
                factory::{
                    http_worker_factory::{create_http_worker, WebAPISyncWorkerBuilder},
                    websocket_worker_factory::{create_websocket_worker, WebsocketSyncWorkerBuilder},
                },
                http_worker::WebAPISyncWorker,
                websocket_worker::WebsocketSyncWorker,
                worker_traits::SyncWorker,
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

// Consider a message-driven design to avoid overusing locks and to simplify design.

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
    task_manager: TaskManager<
        SyncTaskQueue<
            WebRequestRateLimiter,
            TokioBroadcastingMessageBusReceiver<GetTaskRequest>,
        >,
        TokioBroadcastingMessageBusSender<Arc<Mutex<SyncTask>>>,
        TokioMpscMessageBusSender<TaskManagerError>,
        TokioSpmcMessageBusReceiver<Arc<Mutex<SyncTask>>>,
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

    // Move the common bounds to associated types to declutter the method signature
    type RateLimiter = WebRequestRateLimiter;
    type RateLimiterBuilder = WebRequestRateLimiterBuilder;

    type QueueBuilder = SyncTaskQueueBuilder<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>;
    
    type CompletedTaskChannelType = TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>;
    type StreamingDataChannelType = TokioBroadcastingMessageBusReceiver<StreamingData>;
    type FailedTaskChannelType = TokioBroadcastingMessageBusReceiver<Arc<Mutex<SyncTask>>>;
    type WorkerErrorChannelType = TokioBroadcastingMessageBusReceiver<SyncWorkerError>;
    
    // add new sync plans to synchronize
    async fn assign(
        &mut self,
        sync_plans: Vec<Arc<RwLock<SyncPlan>>>,
    ) -> Result<(), TaskExecutorError>
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
        // let mut task_manager_lock = self.task_manager.lock().await;
        self.task_manager.set_task_sender(todo_task_sender);

        // task_manager_lock.set_failed_task_channel(failed_task_receiver);
        self.completed_task_receiver = completed_task_receiver;
        self.streaming_data_receiver = completed_streaming_data_receiver;
        self.failed_task_receiver = failed_task_receiver;
        self.worker_error_receiver = worker_error_receiver;


        // create and assign a worker for each sync plan
        for (i, sync_plan) in sync_plans.iter().enumerate() {
            let plan_lock = sync_plan.read().await;
            match plan_lock.sync_config().sync_mode() {
                SyncMode::HttpAPI => {
                    let mut http_worker: WebAPISyncWorker<
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
                    match http_worker.assign_sync_plan(plan_lock.id()) {
                        Ok(_) => {
                            self.http_api_sync_workers
                                .insert(*http_worker.id(), http_worker);
                        },
                        Err(e) => {
                            error!("{:?}", e);
                            return Err(TaskExecutorError::WorkerAssignmentFailed(format!("Unable to assign plan to worker: {:?}", e)));
                        }
                    }
                    
                }
                SyncMode::WebsocketStreaming => {
                    let mut websocket_worker: WebsocketSyncWorker<
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
                    match websocket_worker.assign_sync_plan(plan_lock.id()) {
                        Ok(_) => {
                            self.websocket_streaming_workers
                                .insert(*websocket_worker.id(), websocket_worker);
                        },
                        Err(e) => {
                            error!("{:?}", e);
                            return Err(TaskExecutorError::WorkerAssignmentFailed(format!("Unable to assign plan to worker: {:?}", e)));
                        }
                    }
                    
                }
            }
        }

        // load sync plans into task manager
        match self.task_manager.load_sync_plans::<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>(sync_plans, task_receivers).await {
            Ok(_) => Ok(()),
            Err(e) => { 
                error!("{:?}", e);
                Err(TaskExecutorError::LoadPlanFailure) 
            }, // Convert the error type if needed
        }
    }

    // get completed task receiver to subscribe to completed task
    fn subscribe_completed_task(&mut self) -> Self::CompletedTaskChannelType {
        return self.completed_task_receiver.clone();
    }

    // get streaming data receiver
    fn subscribe_streaming_data(&mut self) -> Self::StreamingDataChannelType {
        return self.streaming_data_receiver.clone();
    }

    // get failed task receiver
    fn subscribe_failed_task(&mut self) -> Self::FailedTaskChannelType  {
        return self.failed_task_receiver.clone();
    }

    // get worker error receiver
    fn subscribe_worker_error(&mut self) -> Self::WorkerErrorChannelType {
        return self.worker_error_receiver.clone();
    }

    // run a single plan. Either start a new plan or continue a paused plan
    // Note that it only marks a plan’s state as running, but will not actually run unless run_all is being called
    async fn run(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError> {
        // let mut task_manager = self.task_manager.lock().await;
        match self.task_manager.resume_sending_tasks(sync_plan_id).await {
            Ok(_) => {
                if self.http_api_sync_workers.contains_key(&sync_plan_id) {
                    let assigned_worker = self.http_api_sync_workers.get_mut(&sync_plan_id).unwrap();
                    match assigned_worker.start_sync().await {
                        Ok(_) => { return Ok(()); },
                        Err(e) => {
                            error!("{:?}",e);
                            return Err(TaskExecutorError::WorkerAssignmentFailed(format!("{:?}",e)));
                        }
                    }
                } else if self.websocket_streaming_workers.contains_key(&sync_plan_id) {
                    let assigned_worker = self.websocket_streaming_workers.get_mut(&sync_plan_id).unwrap();
                    match assigned_worker.start_sync().await {
                        Ok(_) => { return Ok(()); },
                        Err(e) => {
                            error!("{:?}",e);
                            return Err(TaskExecutorError::WorkerAssignmentFailed(format!("{:?}",e)));
                        }
                    }
                } else {
                    return Err(TaskExecutorError::NoWorkerAssigned);
                }
            },
            Err(e) => {
                error!("{:?}", e);
                return Err(TaskExecutorError::SyncFailure(format!("Start syncing plan failed because {:?}", e)));
            }
        }
    }

    // run all assigned plans
    async fn run_all(&'static mut self) -> Result<(), TaskExecutorError> {
        let self_arc = Arc::new(self);
        let mut http_worker_tasks: Vec<_> = self.http_api_sync_workers.values_mut().map(|w| {
            let sync_task = tokio::spawn(async {
                match w.start_sync().await {
                    Ok(()) => {
                        info!("Worker {} started syncing.", w.id());
                    },
                    Err(e) => {
                        error!("Worker {} encountered error: {:?}", w.id(), e);
                    }
                }
            });
            return sync_task;
        }).collect();

        let mut websocket_worker_tasks: Vec<_> = self.websocket_streaming_workers.values_mut().map(|w| {
            let sync_task = tokio::spawn(async {
                match w.start_sync().await {
                    Ok(()) => {
                        info!("Worker {} started syncing.", w.id());
                    },
                    Err(e) => {
                        error!("Worker {} encountered error: {:?}", w.id(), e);
                    }
                }
            });
            return sync_task;
        }).collect();

        // let task_manager_arc_mutex = self.task_manager.clone();
        // FIXME: once task manager is started there is no way to release the lock and change it
        
        let send_task = tokio::spawn(async move {
            // let mut task_manager = task_manager_arc_mutex.lock().await;
            let result = self_arc.task_manager.listen_for_get_task_request().await;
            info!("result: {:?}", result);
            match result {
                Ok(()) => {
                    info!("Finished successfully!");
                }
                Err(e) => {
                    info!("Failed: {}", e);
                }
            }
        });

        let mut all_tasks = Vec::new();
        all_tasks.append(&mut http_worker_tasks);
        all_tasks.append(&mut websocket_worker_tasks);
        all_tasks.push(send_task);
        
        join_all(all_tasks).await;
        Ok(())
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
