//! Task Manager
//! Defines synchronization task queue and a manager module to support task polling, scheduling and throttling.
//!

use std::{
    collections::{HashMap, VecDeque},
    error::{self, Error},
    fmt::{self, Debug},
    ops::RangeBounds,
    sync::Arc,
};

use async_trait::async_trait;
use derivative::Derivative;
use futures::{
    executor::block_on,
    future::{join_all, try_join_all},
};
use getset::{Getters, MutGetters, Setters};
use log::{error, info, warn};
use tokio::{
    join,
    sync::{Mutex, RwLock},
    task::JoinHandle,
    time::{sleep, Duration},
};
use uuid::Uuid;

use crate::{
    application::synchronization::dtos::task_manager::CreateTaskManagerRequest,
    domain::synchronization::{
        custom_errors::TimerError,
        rate_limiter::{RateLimitStatus, RateLimiter},
        sync_plan::SyncPlan,
        sync_task::SyncTask,
    },
    infrastructure::mq::{
        message_bus::{
            BroadcastingMessageBusReceiver, BroadcastingMessageBusSender, MessageBusReceiver,
            MessageBusSender, MpscMessageBus, SpmcMessageBusReceiver, SpmcMessageBusSender,
            StaticAsyncComponent, StaticClonableAsyncComponent, StaticClonableMpscMQ,
            StaticMpscMQReceiver,
        },
        tokio_channel_mq::{
            TokioBroadcastingMessageBusReceiver, TokioBroadcastingMessageBusSender,
            TokioMpscMessageBusReceiver, TokioMpscMessageBusSender, TokioSpmcMessageBusReceiver,
            TokioSpmcMessageBusSender,
        },
    },
};

use super::sync_rate_limiter::{new_web_request_limiter, WebRequestRateLimiter};

pub type QueueId = Uuid;
type CooldownTimerTask = JoinHandle<()>;
type TimeSecondLeft = i64;

// Factory Methods
pub fn new_empty_queue<T: RateLimiter, TR: TaskRequestMPMCReceiver>(
    rate_limiter: T,
    task_request_receiver: TR,
    max_retry: Option<u32>,
    sync_plan_id: Uuid,
) -> SyncTaskQueue<T, TR> {
    return SyncTaskQueue::<T, TR>::new(
        vec![],
        Some(rate_limiter),
        max_retry,
        sync_plan_id,
        task_request_receiver,
    );
}

pub fn new_empty_limitless_queue<T: RateLimiter, TR: TaskRequestMPMCReceiver>(
    max_retry: Option<u32>,
    sync_plan_id: Uuid,
    task_request_receiver: TR,
) -> SyncTaskQueue<T, TR> {
    return SyncTaskQueue::new(vec![], None, max_retry, sync_plan_id, task_request_receiver);
}

pub fn create_sync_task_manager<
    T: RateLimiter + 'static,
    MT: SyncTaskMPMCSender,
    TR: TaskRequestMPMCReceiver,
    ME: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
>(
    create_request: &CreateTaskManagerRequest,
    task_sender: MT,
    task_request_receivers: Vec<TR>,
    error_sender: ME,
    failed_task_receiver: MF,
) -> TaskManager<WebRequestRateLimiter, MT, TR, ME, MF>
where
    Vec<SyncTaskQueue<T, TR>>: FromIterator<
        SyncTaskQueue<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>,
    >,
{
    // todo: use abstract factory and factory methods to avoid hard coding
    // for now, we only have a few impls so we can simply hardcode the factory.
    let queues: Vec<SyncTaskQueue<WebRequestRateLimiter, TR>> = create_request
        .create_task_queue_requests()
        .iter()
        .zip(task_request_receivers)
        .map(|(req, task_request_receiver)| {
            match req.rate_limiter_param() {
                Some(create_limiter_req) => match req.rate_limiter_impl() {
                    WebRequestRateLimiter => {
                        let rate_limiter = new_web_request_limiter(
                            *create_limiter_req.max_request(),
                            *create_limiter_req.max_daily_request(),
                            *create_limiter_req.cooldown(),
                        );
                        let queue = new_empty_queue(
                            rate_limiter,
                            task_request_receiver,
                            *req.max_retry(),
                            *req.sync_plan_id(),
                        );
                        return queue;
                    }
                },
                None => {
                    let queue = new_empty_limitless_queue(
                        *req.max_retry(),
                        *req.sync_plan_id(),
                        task_request_receiver,
                    );
                    return queue;
                }
        }})
        .collect();

    let task_manager = TaskManager::new(
        queues,
        task_sender,
        error_sender,
        failed_task_receiver,
    );
    return task_manager;
}

// May need error handling
#[derive(Debug)]
pub enum QueueError {
    NothingToSend,
    SendingNotStarted(String),
    RateLimited(Option<CooldownTimerTask>, TimeSecondLeft),
    DailyLimitExceeded,
    RateLimiterError(TimerError),
    QueuePaused(String),
    QueueStopped(String),
    QueueFinished(String),
    EmptyRequestReceived(String),
    UnmatchedSyncPlanId
}

// Component Definitions
#[derive(Debug)]
pub enum SyncTaskQueueValue {
    Task(Option<SyncTask>),
    RateLimited(Option<CooldownTimerTask>, TimeSecondLeft), // timer task, seco
    DailyLimitExceeded,
}

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum QueueStatus {
    #[derivative(Default)]
    Initialized,
    SendingTasks,
    Paused,
    RateLimited(TimeSecondLeft),
    Stopped,
    Finished,
}

#[derive(Derivative, Getters, Setters, Debug, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct SyncTaskQueue<T: RateLimiter, TR: TaskRequestMPMCReceiver> {
    sync_plan_id: Uuid,
    tasks: VecDeque<SyncTask>,
    // used to listen for a poll request
    task_request_receiver: TR,
    rate_limiter: Option<T>,
    max_retry: Option<u32>,
    retries_left: Option<u32>,
    status: QueueStatus,
    initial_size: usize,
}

impl<T: RateLimiter, TR: TaskRequestMPMCReceiver> SyncTaskQueue<T, TR> {
    pub fn new(
        tasks: Vec<SyncTask>,
        rate_limiter: Option<T>,
        max_retry: Option<u32>,
        sync_plan_id: Uuid,
        task_request_receiver: TR,
    ) -> SyncTaskQueue<T, TR> {
        let task_queue = VecDeque::from(tasks);
        let total_tasks = task_queue.len();
        SyncTaskQueue {
            tasks: task_queue,
            rate_limiter,
            max_retry,
            retries_left: max_retry,
            sync_plan_id,
            status: QueueStatus::default(),
            initial_size: total_tasks,
            task_request_receiver,
        }
    }

    pub fn start_sending_tasks(&mut self) {
        self.status = QueueStatus::SendingTasks;
        info!("Queue {} has start sending tasks.", self.sync_plan_id);
    }

    pub fn pause(&mut self) {
        self.status = QueueStatus::Paused;
        info!("Queue {} has been paused.", self.sync_plan_id);
    }

    pub fn resume(&mut self) {
        self.status = QueueStatus::SendingTasks;
        info!("Queue {}'s status has set to resumed.", self.sync_plan_id);
    }

    pub fn finished(&mut self) {
        self.status = QueueStatus::Finished;
        info!(
            "Queue {}'s status has finished sending tasks.",
            self.sync_plan_id
        );
    }

    pub fn stop(&mut self) -> Vec<SyncTask> {
        self.drain_all()
    }

    /// Listens for task fetching request. Try to fetch a task of the queue if received such request
    /// Otherwise it will block and yield to other async task
    async fn wait_and_fetch_task(&mut self) -> Result<SyncTask, QueueError> {
        let task_fetch_request_recv_result = self.task_request_receiver.receive().await;
        match task_fetch_request_recv_result {
            None => {
                error!("Expected to receive a task request but received none in queue {}!", *self.sync_plan_id());
                Err(QueueError::EmptyRequestReceived(String::from("Expected to receive a task request but received none!")))
            },
            Some(task_fetch_request) => {
                if *task_fetch_request.sync_plan_id() != self.sync_plan_id {
                    return Err(QueueError::UnmatchedSyncPlanId);
                } else {
                    let fetch_result = self.pop_front().await;
                    return fetch_result;
                }
            },
        }

    }

    async fn try_fetch_task(
        tasks: &mut VecDeque<SyncTask>,
        rate_limiter: &mut T,
    ) -> Result<SyncTask, QueueError> {
        let rate_limiter_response = rate_limiter.can_proceed().await;
        match rate_limiter_response {
            RateLimitStatus::Ok(available_request_left) => {
                info!(
                    "Rate limiter permits this request. There are {} requests left.",
                    available_request_left
                );
                match tasks.pop_front() {
                    Some(value) => Ok(value),
                    None => Err(QueueError::NothingToSend),
                }
            }
            RateLimitStatus::RequestPerDayExceeded => Err(QueueError::DailyLimitExceeded),
            RateLimitStatus::RequestPerMinuteExceeded(should_start_cooldown, seconds_left) => {
                if !should_start_cooldown {
                    return Err(QueueError::RateLimited(None, seconds_left));
                }
                let result = rate_limiter.start_countdown(true).await;
                match result {
                    Ok(countdown_task) => {
                        return Err(QueueError::RateLimited(Some(countdown_task), seconds_left));
                    }
                    Err(e) => {
                        return Err(QueueError::RateLimiterError(e));
                    }
                }
            }
        }
    }

    pub async fn pop_front(&mut self) -> Result<SyncTask, QueueError> {
        //! try to pop the front of the task queue
        //! if the queue is empty, or the queue has a rate limiter, and the rate limiter rejects the request, return None
        match self.status {
            QueueStatus::Initialized => {
                return Err(QueueError::SendingNotStarted(
                    "Please call start before sending tasks".to_string(),
                ));
            }
            QueueStatus::Paused => {
                return Err(QueueError::QueuePaused(
                    "Queue is paused. Please call resume to begin sending tasks.".to_string(),
                ));
            }
            QueueStatus::Stopped => {
                return Err(QueueError::QueueStopped(format!(
                    "Queue {} has stopped.",
                    self.sync_plan_id()
                )))
            }
            QueueStatus::Finished => {
                return Err(QueueError::QueueFinished(format!(
                    "Queue {} has finished sending tasks.",
                    self.sync_plan_id()
                )));
            }
            QueueStatus::RateLimited(_) => {
                let rate_limiter = self.rate_limiter.as_mut().expect(&format!("Something unlikely happened to queue {} because it is rate limited without a rate limiter!", self.sync_plan_id));
                return Self::try_fetch_task(&mut self.tasks, rate_limiter).await;
            }
            QueueStatus::SendingTasks => match &mut self.rate_limiter {
                None => {
                    let value = self.tasks.pop_front();
                    match value {
                        Some(task) => {
                            return Ok(task);
                        }
                        None => {
                            return Err(QueueError::NothingToSend);
                        }
                    }
                }
                Some(rate_limiter) => {
                    return Self::try_fetch_task(&mut self.tasks, rate_limiter).await;
                }
            },
        }
    }

    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Vec<SyncTask> {
        //! Pops all elements in the queue given the range
        //! Typically used when the remote reports a daily limited reached error
        // let mut q_lock = .lock().await;
        let values = self.tasks.drain(range);
        self.status = QueueStatus::Stopped;
        return values.collect::<Vec<_>>();
    }

    pub fn drain_all(&mut self) -> Vec<SyncTask> {
        let values = self.tasks.drain(0..self.len());
        self.status = QueueStatus::Stopped;
        return values.collect::<Vec<_>>();
    }

    pub fn push_back(&mut self, task: SyncTask) {
        self.tasks.push_back(task);

        // Update initial size if current length execeeds initial size
        if self.len() > self.initial_size {
            self.initial_size = self.len();
        }
        return ();
    }

    pub fn push_front(&mut self, task: SyncTask) {
        self.tasks.push_front(task);
        // Update initial size if current length execeeds initial size
        if self.len() > self.initial_size {
            self.initial_size = self.len();
        }
        return ();
    }

    pub fn front(&self) -> Option<&SyncTask> {
        self.tasks.front()
    }

    pub fn is_empty(&self) -> bool {
        return self.tasks.is_empty();
    }

    pub fn is_finished(&self) -> bool {
        return self.status == QueueStatus::Finished;
    }

    pub fn is_running(&self) -> bool {
        return self.status == QueueStatus::SendingTasks;
    }

    pub fn is_paused(&self) -> bool {
        return self.status == QueueStatus::Paused;
    }

    pub fn is_stopped(&self) -> bool {
        return self.status == QueueStatus::Stopped;
    }

    pub fn len(&self) -> usize {
        return self.tasks.len();
    }

    pub fn can_retry(&self) -> bool {
        match self.max_retry {
            Some(max_retry) => {
                if let Some(n_retry) = self.retries_left {
                    return n_retry > 0 && n_retry <= max_retry;
                } else {
                    return false;
                }
            }
            None => {
                return true;
            }
        }
    }

    pub fn retry(&mut self, task: SyncTask) {
        if self.can_retry() {
            self.push_back(task)
        }
    }
}

#[derive(Debug, Clone)]
pub enum TaskManagerError {
    RateLimited(Option<Arc<CooldownTimerTask>>, TimeSecondLeft), // timer task, seco
    DailyLimitExceeded,
    RateLimiterInitializationError(TimerError),
    BatchPlanInsertionFailed(String),
    QueueNotFound,
    MissingRateLimitParam,
    RateLimiterNotSet(String),
    OtherError(String),
}

impl fmt::Display for TaskManagerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl error::Error for TaskManagerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

/// TaskManager
#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct TaskSendingProgress {
    sync_plan_id: Uuid,
    task_sent: usize,
    total_tasks: usize,
    complete_rate: f32,
}

#[async_trait]
pub trait SyncTaskManager<T: RateLimiter, TR: TaskRequestMPMCReceiver> {
    // start syncing all plans by sending tasks out to workers
    async fn listen_for_get_task_request(&mut self) -> Result<(), TaskManagerError>;

    // stop syncing all plans
    async fn stop_sending_all_tasks(
        &mut self,
    ) -> Result<HashMap<Uuid, Vec<SyncTask>>, TaskManagerError>;

    // When need add new tasks ad hoc, use this method
    async fn add_tasks_to_plan(
        &mut self,
        plan_id: Uuid,
        tasks: Vec<SyncTask>,
    ) -> Result<(), TaskManagerError>;

    // add new plans to sync
    async fn load_sync_plan(
        &mut self,
        sync_plan: &SyncPlan,
        rate_limiter: Option<T>,
        task_request_receiver: TR
    ) -> Result<(), TaskManagerError>;
    async fn load_sync_plans(
        &mut self,
        sync_plans: &[SyncPlan],
        rate_limiters: Vec<Option<T>>,
        task_request_receivers: Vec<TR>
    ) -> Result<(), TaskManagerError>;

    // stop syncing given the id, but it is resumable
    async fn stop_sending_task(&mut self, sync_plan_id: Uuid) -> Result<(), TaskManagerError>;

    // stop and remove the sync plan
    async fn stop_and_remove_sync_plan(
        &mut self,
        sync_plan_id: Uuid,
    ) -> Result<Vec<SyncTask>, TaskManagerError>;

    // pause sending tasks
    // typically used when rate limiting does not help
    // then users can pause it for a while to let the remote release the limit
    async fn pause_sending_task(&mut self, sync_plan_id: Uuid) -> Result<(), TaskManagerError>;

    // resume sending tasks
    async fn resume_sending_tasks(&mut self, sync_plan_id: Uuid) -> Result<(), TaskManagerError>;

    // report how many unsent tasks for each sync plan
    async fn report_task_sending_progress(
        &self,
        sync_plan_id: Uuid,
    ) -> Result<TaskSendingProgress, TaskManagerError>;
    async fn report_all_task_sending_progress(
        &self,
    ) -> Result<Vec<TaskSendingProgress>, TaskManagerError>;

    // stop all sync plans and wait until all queues are closed, then releases all resources
    async fn graceful_shutdown(&mut self) -> Result<(), TaskManagerError>;

    // stop directly without waiting
    fn force_shutdown(&mut self);
}

trait SyncTaskMpscSender: MessageBusSender<SyncTask> + StaticClonableMpscMQ {}
trait SyncTaskMpscReceiver: MessageBusReceiver<SyncTask> + StaticMpscMQReceiver {}

impl SyncTaskMpscSender for TokioMpscMessageBusSender<SyncTask> {}
impl SyncTaskMpscReceiver for TokioMpscMessageBusReceiver<SyncTask> {}

pub trait SyncTaskMPSCSender:
    MessageBusSender<SyncTask> + MpscMessageBus + StaticAsyncComponent
{
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPSCSender>;
}

impl SyncTaskMPSCSender for TokioMpscMessageBusSender<SyncTask> {
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPSCSender> {
        Box::new(self.clone())
    }
}

pub trait SyncTaskMPSCReceiver:
    MessageBusReceiver<SyncTask> + MpscMessageBus + StaticAsyncComponent
{
}
impl SyncTaskMPSCReceiver for TokioMpscMessageBusReceiver<SyncTask> {}

pub trait SyncTaskMPMCSender:
    MessageBusSender<SyncTask> + StaticAsyncComponent + BroadcastingMessageBusSender<SyncTask>
{
}

pub trait SyncTaskMPMCReceiver:
    MessageBusReceiver<SyncTask> + StaticAsyncComponent + BroadcastingMessageBusReceiver
{
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPMCReceiver>;
}

impl StaticAsyncComponent for TokioBroadcastingMessageBusSender<SyncTask> {}

impl SyncTaskMPMCSender for TokioBroadcastingMessageBusSender<SyncTask> {}
impl SyncTaskMPMCReceiver for TokioBroadcastingMessageBusReceiver<SyncTask> {
    fn clone_boxed(&self) -> Box<dyn SyncTaskMPMCReceiver> {
        Box::new(self.clone())
    }
}

#[derive(Derivative, Getters, Setters, Default, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct GetTaskRequest {
    sync_plan_id: Uuid,
}

pub trait TaskRequestMPMCSender:
    MessageBusSender<GetTaskRequest>
    + StaticAsyncComponent
    + BroadcastingMessageBusSender<GetTaskRequest>
{
    fn clone_boxed(&self) -> Box<dyn TaskRequestMPMCSender>;
}

pub trait TaskRequestMPMCReceiver:
    MessageBusReceiver<GetTaskRequest> + StaticAsyncComponent + BroadcastingMessageBusReceiver
{
    fn clone_boxed(&self) -> Box<dyn TaskRequestMPMCReceiver>;
}

impl StaticAsyncComponent for TokioBroadcastingMessageBusSender<GetTaskRequest> {}

impl TaskRequestMPMCSender for TokioBroadcastingMessageBusSender<GetTaskRequest> {
    fn clone_boxed(&self) -> Box<dyn TaskRequestMPMCSender> {
        Box::new(self.clone())
    }
}
impl TaskRequestMPMCReceiver for TokioBroadcastingMessageBusReceiver<GetTaskRequest> {
    fn clone_boxed(&self) -> Box<dyn TaskRequestMPMCReceiver> {
        Box::new(self.clone())
    }
}

trait TaskManagerErrorMpscSender: MessageBusSender<TaskManagerError> + StaticClonableMpscMQ {}
pub trait TaskManagerErrorMPSCSender:
    MessageBusSender<TaskManagerError> + MpscMessageBus + StaticAsyncComponent
{
    fn clone_boxed(&self) -> Box<dyn TaskManagerErrorMPSCSender>;
}

impl TaskManagerErrorMPSCSender for TokioMpscMessageBusSender<TaskManagerError> {
    fn clone_boxed(&self) -> Box<dyn TaskManagerErrorMPSCSender> {
        Box::new(self.clone())
    }
}

trait TaskManagerErrorMpscReceiver:
    MessageBusReceiver<TaskManagerError> + StaticMpscMQReceiver
{
}

impl TaskManagerErrorMpscSender for TokioMpscMessageBusSender<TaskManagerError> {}
impl TaskManagerErrorMpscReceiver for TokioMpscMessageBusReceiver<TaskManagerError> {}

pub trait TaskManagerErrorMPSCReceiver:
    MessageBusReceiver<TaskManagerError> + MpscMessageBus + StaticAsyncComponent
{
}

impl TaskManagerErrorMPSCReceiver for TokioMpscMessageBusReceiver<TaskManagerError> {}

pub type FailedTask = (Uuid, SyncTask);
trait FailedTaskSpmcReceiver:
    MessageBusReceiver<FailedTask> + StaticClonableAsyncComponent + SpmcMessageBusReceiver
{
}

trait FailedTaskSpmcSender:
    MessageBusSender<FailedTask> + StaticAsyncComponent + SpmcMessageBusSender<FailedTask>
{
}

impl FailedTaskSpmcReceiver for TokioSpmcMessageBusReceiver<FailedTask> {}
impl FailedTaskSpmcSender for TokioSpmcMessageBusSender<FailedTask> {}

pub trait FailedTaskSPMCReceiver:
    MessageBusReceiver<FailedTask> + StaticAsyncComponent + SpmcMessageBusReceiver
{
    fn clone_boxed(&self) -> Box<dyn FailedTaskSPMCReceiver>;
}

impl FailedTaskSPMCReceiver for TokioSpmcMessageBusReceiver<FailedTask> {
    fn clone_boxed(&self) -> Box<dyn FailedTaskSPMCReceiver> {
        Box::new(self.clone())
    }
}

pub trait FailedTaskSPMCSender:
    MessageBusSender<FailedTask> + StaticAsyncComponent + SpmcMessageBusSender<FailedTask>
{
}

impl FailedTaskSPMCSender for TokioSpmcMessageBusSender<FailedTask> {}

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
#[derive(Debug, PartialEq, Eq, Clone)]
enum TaskManagerState {
    #[derivative(Default)]
    Initialized,
    Running,
    Stopped,
}

/// TaskManager
/// TaskManger is responsible for sending tasks for each sync plan upon receiving request.
#[derive(Derivative, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub")]
pub struct TaskManager<T, MT, TR, ME, MF>
where
    T: RateLimiter,
    MT: SyncTaskMPMCSender,
    TR: TaskRequestMPMCReceiver,
    ME: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    queues: Arc<RwLock<HashMap<QueueId, Arc<Mutex<SyncTaskQueue<T, TR>>>>>>,
    task_sender: MT,
    error_message_channel: ME,
    failed_task_channel: MF,
    current_state: TaskManagerState,
}

impl<T, MT, TR, ME, MF> TaskManager<T, MT, TR, ME, MF>
where
    T: RateLimiter,
    MT: SyncTaskMPMCSender,
    TR: TaskRequestMPMCReceiver,
    ME: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    pub fn new(
        task_queues: Vec<SyncTaskQueue<T, TR>>,
        task_sender: MT,
        error_message_channel: ME,
        failed_task_channel: MF,
    ) -> TaskManager<T, MT, TR, ME, MF> {
        let mut q_map = HashMap::new();
        task_queues.into_iter().for_each(|q| {
            q_map.insert(*q.sync_plan_id(), Arc::new(Mutex::new(q)));
        });
        Self {
            queues: Arc::new(RwLock::new(q_map)),
            task_sender,
            error_message_channel,
            failed_task_channel,
            current_state: TaskManagerState::default(),
        }
    }

    pub async fn add_queue(&mut self, task_queue: SyncTaskQueue<T, TR>) {
        let mut q_lock = self.queues.write().await;
        q_lock.insert(*task_queue.sync_plan_id(), Arc::new(Mutex::new(task_queue)));
    }

    pub async fn add_tasks_to_queue(&mut self, queue_id: QueueId, tasks: Vec<SyncTask>) {
        let queues = self.queues.read().await;
        let q_result = queues.get(&queue_id);

        if let Some(q) = q_result {
            let mut q_lock = q.lock().await;
            tasks.into_iter().for_each(|t| {
                q_lock.push_back(t);
            })
        }
    }

    pub fn is_running(&self) -> bool {
        return self.current_state == TaskManagerState::Running;
    }

    /// Check whether all queues are empty. If so, the
    pub async fn all_queues_are_empty(&self) -> bool {
        let queues = self.queues.read().await;
        let is_empty = queues.values().all(|q| {
            let q_lock = q.blocking_lock();
            return q_lock.is_empty();
        });
        return is_empty;
    }

    pub fn close_all_channels(&mut self) {
        info!("Closing all channels for task manager...");
        self.task_sender.close();
        self.error_message_channel.close();
        self.failed_task_channel.close();
        info!("All channels are closed.")
    }
}

#[async_trait]
impl<T, MT, TR, ME, MF> SyncTaskManager<T, TR> for TaskManager<T, MT, TR, ME, MF>
where
    T: RateLimiter + 'static,
    MT: SyncTaskMPMCSender + Clone,
    TR: TaskRequestMPMCReceiver,
    ME: TaskManagerErrorMPSCSender + Clone,
    MF: FailedTaskSPMCReceiver + Clone,
{
    async fn add_tasks_to_plan(
        &mut self,
        plan_id: Uuid,
        tasks: Vec<SyncTask>,
    ) -> Result<(), TaskManagerError> {
        self.add_tasks_to_queue(plan_id, tasks).await;
        Ok(())
    }

    async fn stop_sending_all_tasks(
        &mut self,
    ) -> Result<HashMap<Uuid, Vec<SyncTask>>, TaskManagerError> {
        let queues = self.queues.read().await;
        let mut unsent_tasks = HashMap::new();
        for queue in queues.values() {
            let mut queue_lock = queue.lock().await;
            let remaining_tasks = queue_lock.drain_all(); // Drop all tasks
            unsent_tasks.insert(*queue_lock.sync_plan_id(), remaining_tasks);
        }
        Ok(unsent_tasks)
    }

    async fn load_sync_plan(
        &mut self,
        sync_plan: &SyncPlan,
        rate_limiter: Option<T>,
        task_request_receiver: TR
    ) -> Result<(), TaskManagerError> {
        let sync_config = sync_plan.sync_config();

        match rate_limiter {
            Some(rate_limiter) => {
                let quota = sync_config.sync_rate_quota()
                    .as_ref()
                    .ok_or(TaskManagerError::MissingRateLimitParam)?;

                let mut queue = new_empty_queue(rate_limiter, task_request_receiver, Some(*quota.max_retry()), *sync_plan.id());
                sync_plan.tasks().iter().for_each(|t| queue.push_back(t.clone()));
                
                self.add_queue(queue).await;
                Ok(())
            }
            None => match sync_config.sync_rate_quota() {
                None => {
                    let mut queue = new_empty_limitless_queue(None, *sync_plan.id(), task_request_receiver);
                    sync_plan.tasks().iter().for_each(|t| queue.push_back(t.clone()));

                    self.add_queue(queue).await;
                    Ok(())
                },
                Some(_) => Err(TaskManagerError::RateLimiterNotSet(
                    String::from("Missing rate limiter while expecting to pass one because you have specified rate quota!"))),
            },
        }
    }

    async fn load_sync_plans(
        &mut self,
        sync_plans: &[SyncPlan],
        rate_limiters: Vec<Option<T>>,
        task_request_receivers: Vec<TR>
    ) -> Result<(), TaskManagerError> {
        for ((plan, limiter), task_request_receiver) in sync_plans.iter().zip(rate_limiters).zip(task_request_receivers) {
            let result = self.load_sync_plan(plan, limiter, task_request_receiver).await;
            if let Err(e) = result {
                error!("{:?}", e);
                return Err(TaskManagerError::BatchPlanInsertionFailed(
                    String::from("Batch task insertion failed due to inconsistencies between rate limit param and actual passed rate limiter type.")));
            }
        }

        Ok(())
    }

    async fn stop_sending_task(&mut self, sync_plan_id: Uuid) -> Result<(), TaskManagerError> {
        let queues = self.queues.read().await;
        if let Some(queue) = queues.get(&sync_plan_id) {
            let mut q_lock = queue.lock().await;
            q_lock.stop();
            Ok(())
        } else {
            Err(TaskManagerError::QueueNotFound)
        }
    }

    async fn stop_and_remove_sync_plan(
        &mut self,
        sync_plan_id: Uuid,
    ) -> Result<Vec<SyncTask>, TaskManagerError> {
        let queues = self.queues.read().await;
        if let Some(queue) = queues.get(&sync_plan_id) {
            let mut q_lock = queue.lock().await;
            let unsent_task = q_lock.drain_all();
            Ok(unsent_task)
        } else {
            Err(TaskManagerError::QueueNotFound)
        }
    }

    async fn pause_sending_task(&mut self, sync_plan_id: Uuid) -> Result<(), TaskManagerError> {
        let queues = self.queues.read().await;
        if let Some(queue) = queues.get(&sync_plan_id) {
            let mut queue_lock = queue.lock().await;
            queue_lock.pause();
            Ok(())
        } else {
            Err(TaskManagerError::QueueNotFound)
        }
    }

    async fn resume_sending_tasks(&mut self, sync_plan_id: Uuid) -> Result<(), TaskManagerError> {
        let queues = self.queues.read().await;
        if let Some(queue) = queues.get(&sync_plan_id) {
            let mut queue_lock = queue.lock().await;
            queue_lock.resume();
            Ok(())
        } else {
            Err(TaskManagerError::QueueNotFound)
        }
    }

    async fn report_task_sending_progress(
        &self,
        sync_plan_id: Uuid,
    ) -> Result<TaskSendingProgress, TaskManagerError> {
        let queues = self.queues.read().await;
        if let Some(queue) = queues.get(&sync_plan_id) {
            let queue_lock = queue.lock().await;
            let n_task_sent = *queue_lock.initial_size() - queue_lock.len();
            let complete_rate = (n_task_sent as f32) / (*queue_lock.initial_size() as f32);
            let total_tasks = *queue_lock.initial_size();
            let progress = TaskSendingProgress {
                sync_plan_id,
                task_sent: n_task_sent,
                total_tasks,
                complete_rate,
            };
            Ok(progress)
        } else {
            Err(TaskManagerError::QueueNotFound)
        }
    }

    async fn report_all_task_sending_progress(
        &self,
    ) -> Result<Vec<TaskSendingProgress>, TaskManagerError> {
        let queues = self.queues.read().await;
        let progress_report_tasks = queues
            .keys()
            .map(|sync_plan_id| async { self.report_task_sending_progress(*sync_plan_id).await });
        try_join_all(progress_report_tasks).await
    }

    async fn graceful_shutdown(&mut self) -> Result<(), TaskManagerError> {
        info!("Trying to gracefully shutdown task manager...");
        self.current_state = TaskManagerState::Stopped;
        self.stop_sending_all_tasks().await?;
        self.close_all_channels();
        info!("Done!");
        Ok(())
    }

    fn force_shutdown(&mut self) {
        // Note: We are not making this method asynchronous because it's intended to be immediate.
        // It does not guarantee that all tasks are stopped, just that all queues are cleared.
        warn!("You are trying to forcibly shutdown task manager!");
        self.current_state = TaskManagerState::Stopped;
        self.close_all_channels();
        let mut queues = block_on(self.queues.write());
        queues.clear();
        info!("Task Manager shutdown completed!");
    }

    /// start task manager and listens for get task request
    /// Task manager will poll its queues and try to get a task from each of them, and then send the task to task channel
    async fn listen_for_get_task_request(&mut self) -> Result<(), TaskManagerError> {
        let failed_task_channel = Arc::new(Mutex::new(self.failed_task_channel.clone()));
        let queues = Arc::clone(&self.queues);
        let task_sender = self.task_sender.clone();
        let error_message_channel = self.error_message_channel.clone();
        self.current_state = TaskManagerState::Running;

        let fetch_tasks: Vec<_> = queues.read().await
            .iter()
            .map(|(q_id, task_queue)| {
                let q_id = q_id.clone();
                let queue = Arc::clone(task_queue); // Cloning it here
                let new_task_sender_channel = task_sender.clone();
                let new_error_sender_channel = error_message_channel.clone();

                tokio::spawn(async move {
                    let q_id = q_id.clone();
                    let mut q_lock = queue.lock().await;
                    q_lock.start_sending_tasks();
                    drop(q_lock);

                    loop {
                        
                        let mut q_lock = queue.lock().await;
                        info!("Acquired lock for queue {:?} to fetch a task", q_id);

                        if q_lock.is_paused() || q_lock.is_stopped() {
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        } else if q_lock.is_finished() {
                            break;
                        }

                        let fetch_result = q_lock.wait_and_fetch_task().await;
                        drop(q_lock);
                        info!("Released lock for queue {:?} after fetching a task", q_id);

                        match fetch_result {
                            Ok(task) => {
                                let task_id = task.id();
                                let r = new_task_sender_channel.send(task.clone()).await;

                                match r {
                                    Ok(()) => {
                                        info!("Sent task {:?} in queue {:#?}", task_id, q_id);
                                    },
                                    Err(e) => {
                                        info!("Failed to send task because {:?}", e);
                                        let mut q_lock = queue.lock().await;
                                        info!("Acquired lock for queue {:?} to push back a task", q_id);
                                        q_lock.retry(task);
                                    }
                                }  
                            },
                            Err(e) => {
                                match e {
                                    QueueError::UnmatchedSyncPlanId => {
                                        info!("Sync plan id does not match. Skipped...");
                                    },
                                    QueueError::EmptyRequestReceived(reason) => {
                                        error!("{}", &reason);
                                    },
                                    QueueError::DailyLimitExceeded => {
                                        error!("SyncPlan {} has reached daily request limit!", q_id);
                                        let _ = new_error_sender_channel
                                            .send(TaskManagerError::DailyLimitExceeded)
                                            .await;
                                    },
                                    QueueError::NothingToSend => {
                                        info!("Queue {} has nothing to send. Task sending finished.", q_id);
                                        let mut q_lock = queue.lock().await;
                                        q_lock.finished();
                                    },
                                    QueueError::QueueFinished(_) => {
                                        info!("Queue {} has finished sending tasks.", q_id);
                                    },
                                    QueueError::QueuePaused(_) => {
                                        info!("Queue {} is paused.", q_id);
                                    },
                                    QueueError::QueueStopped(reason) => {
                                        info!("{}", reason);
                                    },
                                    QueueError::RateLimited(cooldown_task, seconds_left) => {
                                        info!("Queue {} is rate limited. time left: {}", q_id, seconds_left);
                                        if let Some(ct) = cooldown_task {
                                            let _ = new_error_sender_channel
                                            .send(TaskManagerError::RateLimited(Some(Arc::new(ct)), seconds_left))
                                            .await;
                                        }
                                    },
                                    QueueError::RateLimiterError(timer_error) => {
                                        error!("Error happened within the rate limiter of queue {}. Error: {:?}", q_id, timer_error);
                                    },
                                    QueueError::SendingNotStarted(reason) => {
                                        info!("Sending not started.. Try to restart");
                                        let mut q_lock = queue.lock().await;
                                        q_lock.start_sending_tasks();
                                        drop(q_lock);
                                    }
                                }
                            }   
                        }
                        sleep(Duration::from_millis(500)).await;
                    }
                    // Recommend dropping channels explicitly!
                    drop(new_task_sender_channel);
                    info!("Dropped task sender in queue {:#?}!", q_id);
                    drop(new_error_sender_channel);
                    info!("Dropped error sender in queue {:#?}!", q_id);
                    info!("Queue {:#?} has no task left. Quitting...", q_id);
                })
            })
            .collect();
        let finished_plan_cleaning_task = async {
            while self.is_running() {
                let queue_read_lock = self.queues.read().await;
                let check_finished_queues = queue_read_lock.values().into_iter().map(|q| async {
                    let q_lock = q.lock().await;
                    if q_lock.is_finished() {
                        Some(*q_lock.sync_plan_id())
                    } else {
                        None
                    }
                });
                let finished_queues = join_all(check_finished_queues).await;
                drop(queue_read_lock);

                info!("Trying to remove finished queue...");
                let mut queue_write_lock = self.queues.write().await;
                for q_id in finished_queues {
                    if let Some(qid) = q_id {
                        queue_write_lock.remove(&qid);
                    }
                }
                drop(queue_write_lock);

                sleep(Duration::from_millis(500)).await;
            }
        };
        let queues_ref = Arc::clone(&queues);
        let handle_failures = tokio::spawn(async move {
            loop {
                let mut failed_task_channel_lock = failed_task_channel.lock().await;
                let receive_result = failed_task_channel_lock.try_recv();
                match receive_result {
                    Ok((_, failed_task)) => {
                        let queues = queues_ref.read().await;
                        let sync_plan_id = failed_task.sync_plan_id().unwrap_or(Uuid::new_v4());
                        let q_key = sync_plan_id;
                        if let Some(q) = queues.get(&q_key) {
                            let mut q_lock = q.lock().await;
                            if let Some(mut n_retry) = q_lock.retries_left() {
                                if n_retry > 0 {
                                    q_lock.push_back(failed_task);
                                    q_lock.set_retries_left(Some(n_retry - 1));
                                }
                            }
                        } else {
                            error!("Queue not found. Perhaps sync task is not tied to a dataset id and sync plan id. SyncTask: {:#?}", failed_task);
                        }
                    }
                    Err(e) => {
                        error!("{}", e);
                        // just to make this task quit, not the final solution
                        break;
                    }
                }
            }
        });

        info!("Waiting for all tasks to complete.");

        let _ = join!(
            join_all(fetch_tasks),
            handle_failures,
            finished_plan_cleaning_task
        );

        info!("Done!");
        Ok(())
    }
}

// TODO: test may fail because for now queue key is changed to (qid, sync_plan_id, dataset_id)

#[cfg(test)]
mod tests {
    use crate::{
        domain::synchronization::rate_limiter::{RateLimitStatus, RateLimiter},
        domain::synchronization::{
            sync_plan,
            value_objects::task_spec::{RequestMethod, TaskSpecification},
        },
        infrastructure::{
            mq::factory::{create_tokio_mpsc_channel, create_tokio_spmc_channel, create_tokio_broadcasting_channel},
            sync::sync_rate_limiter::{new_web_request_limiter, WebRequestRateLimiter},
        },
    };
    use log::{error, info};

    use super::*;
    use std::sync::Arc;

    use chrono::Local;
    use env_logger;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::name::en::Name;
    use fake::Fake;
    use rand::Rng;
    use serde_json::Value;
    use std::env;
    use url::Url;

    fn init() {
        env::set_var("RUST_LOG", "info");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn it_works() {
        init();

        info!("This record will be captured by `cargo test`");
        error!("??");

        assert_eq!(2, 1 + 1);
    }

    fn random_string(len: usize) -> String {
        let mut rng = rand::thread_rng();
        std::iter::repeat(())
            .map(|()| rng.sample(rand::distributions::Alphanumeric))
            .map(char::from)
            .take(len)
            .collect()
    }

    pub fn generate_random_sync_tasks(n: u32) -> Vec<SyncTask> {
        (0..n)
            .map(|_| {
                let fake_url = format!("http://{}", SafeEmail().fake::<String>());
                let request_endpoint = Url::parse(&fake_url).unwrap();
                let fake_headers: HashMap<String, String> = (0..5)
                    .map(|_| (Name().fake::<String>(), random_string(20)))
                    .collect();
                let fake_payload = Some(Arc::new(Value::String(random_string(50))));
                let fake_method = if rand::random() {
                    RequestMethod::Get
                } else {
                    RequestMethod::Post
                };
                let task_spec = TaskSpecification::new(
                    &fake_url,
                    if fake_method == RequestMethod::Get {
                        "GET"
                    } else {
                        "POST"
                    },
                    fake_headers,
                    fake_payload,
                )
                .unwrap();

                let start_time = Local::now();
                let create_time = Local::now();
                let dataset_name = Some(random_string(10));
                let datasource_name = Some(random_string(10));
                SyncTask::new(
                    Uuid::new_v4(),
                    &dataset_name.unwrap(),
                    Uuid::new_v4(),
                    &datasource_name.unwrap(),
                    task_spec,
                    Uuid::new_v4(),
                )
            })
            .collect()
    }

    pub fn new_queue_with_random_amount_of_tasks<T: RateLimiter, TR: TaskRequestMPMCReceiver>(
        rate_limiter: T,
        task_request_receiver: TR,
        min_tasks: u32,
        max_tasks: u32,
    ) -> SyncTaskQueue<T, TR> {
        let mut rng = rand::thread_rng();
        let max_retry = rng.gen_range(10..20);
        let sync_plan_id = Uuid::new_v4();
        let mut new_queue = new_empty_queue(rate_limiter, task_request_receiver, Some(max_retry), sync_plan_id);
        // let mut new_queue = new_empty_limitless_queue(None);
        let n_task = rng.gen_range(min_tasks..max_tasks);
        let random_tasks = generate_random_sync_tasks(n_task);
        // let mut q_lock = new_queue.lock().await;
        for t in random_tasks {
            new_queue.push_back(t);
        }
        // drop(q_lock);
        return new_queue;
    }

    pub async fn generate_queues_with_web_request_limiter(
        n: u32,
        channel_size: usize,
        min_tasks: u32,
        max_tasks: u32,
    ) -> Vec<(SyncTaskQueue<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>, TokioBroadcastingMessageBusSender<GetTaskRequest>)> {
        let queues = join_all((0..n).map(|_| async {
            let mut rng = rand::thread_rng();
            let max_request: u32 = rng.gen_range(30..100);
            let cooldown: u32 = rng.gen_range(1..3);
            let limiter = new_web_request_limiter(max_request, None, Some(cooldown));
            let (task_req_sender, task_req_receiver) = create_tokio_broadcasting_channel::<GetTaskRequest>(channel_size);
            let q = new_queue_with_random_amount_of_tasks(limiter, task_req_receiver, min_tasks, max_tasks);
            return (q, task_req_sender);
        }))
        .await;
        return queues;
    }

    #[test]
    fn it_should_generate_random_tasks() {
        let tasks = generate_random_sync_tasks(10);
        assert!(tasks.len() == 10);
        // info!("{:?}", tasks);
    }

    #[tokio::test]
    async fn test_add_tasks_to_a_single_queue() {
        init();
        let mut rng = rand::thread_rng();
        let max_retry = rng.gen_range(10..20);
        let test_rate_limiter = WebRequestRateLimiter::new(30, None, Some(3)).unwrap();
        let (task_req_sender, task_req_receiver) = create_tokio_broadcasting_channel::<GetTaskRequest>(100);
        let task_queue: Arc<Mutex<SyncTaskQueue<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>>> =
            Arc::new(Mutex::new(SyncTaskQueue::<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>::new(
                vec![],
                Some(test_rate_limiter),
                Some(max_retry),
                Uuid::new_v4(),
                task_req_receiver
            )));

        let tasks = generate_random_sync_tasks(100);
        let first_task = tasks[0].clone();

        let mut task_queue_lock = task_queue.lock().await;

        for t in tasks {
            task_queue_lock.push_back(t);
            info!("task queue size: {}", task_queue_lock.len());
        }

        if let Some(t1) = task_queue_lock.front() {
            // let t_lock = t1.lock().await;
            assert_eq!(t1.id(), first_task.id(), "Task id not equal!")
        }
    }

    // #[tokio::test]
    // async fn test_producing_and_consuming_tasks() {
    //     init();

    //     // pressure testing
    //     let min_task = 5;
    //     let max_task = 10;
    //     let n_queues = 5;

    //     let queues = generate_queues_with_web_request_limiter(n_queues, min_task, max_task).await;

    //     let (task_sender, mut task_receiver) = create_tokio_mpsc_channel::<SyncTask>(1000);

    //     let (error_msg_sender, mut error_msg_receiver) =
    //         create_tokio_mpsc_channel::<TaskManagerError>(500);

    //     let (failed_task_sender, failed_task_receiver) =
    //         create_tokio_spmc_channel::<(Uuid, SyncTask)>(500);

    //     let mut task_manager =
    //         TaskManager::new(queues, task_sender, error_msg_sender, failed_task_receiver);

    //     let send_task = tokio::spawn(async move {
    //         let result = task_manager.listen_for_get_task_request().await;
    //         info!("result: {:?}", result);
    //         match result {
    //             Ok(()) => {
    //                 info!("Finished successfully!");
    //             }
    //             Err(e) => {
    //                 info!("Failed: {}", e);
    //             }
    //         }
    //     });

    //     let receive_task = tokio::spawn(async move {
    //         info!("Before acquiring the mq lock...");

    //         info!("MQ lock acquired...");
    //         let mut task_cnt = 0;
    //         let recv_task = tokio::spawn(async move {
    //             loop {
    //                 let result = task_receiver.receive().await;
    //                 if let Some(_) = result {
    //                     task_cnt += 1;
    //                     info!("Received task, count: {:?}", task_cnt);
    //                 } else {
    //                     // Important! Use this branch to exit.
    //                     info!("No task left... Exit...");
    //                     break;
    //                 }
    //             }
    //         });
    //         let err_report_task = tokio::spawn(async move {
    //             loop {
    //                 let result = error_msg_receiver.receive().await;
    //                 if let Some(e) = result {
    //                     error!("Received error, {:?}", e);
    //                 } else {
    //                     // Important! Use this branch to exit.
    //                     info!("No error received.");
    //                     break;
    //                 }
    //             }
    //         });

    //         let _ = join!(recv_task, err_report_task);

    //         // let _ = task_channel_lock.close();
    //         info!("Done receiving tasks...");
    //         info!("Receiver tries to release the mq lock...");
    //         // drop(task_channel_lock);
    //         info!("MQ lock released...");
    //     });

    //     // FIXME: loop will not quit after finishing tasks
    //     let _ = tokio::join!(send_task, receive_task);
    //     // let _ = tokio::join!(send_task,);
    // }
}
