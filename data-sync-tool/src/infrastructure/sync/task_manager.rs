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
use futures::future::join_all;
use getset::{Getters, Setters};

use log::error;
use rbatis::dark_std::sync::vec;
use tokio::{
    join,
    sync::{Mutex, RwLock},
    task::JoinHandle,
};
use uuid::Uuid;

use crate::{
    application::synchronization::dtos::task_manager::CreateTaskManagerRequest,
    domain::synchronization::{
        custom_errors::TimerError,
        rate_limiter::{RateLimitStatus, RateLimiter},
        sync_task::SyncTask,
    },
    infrastructure::mq::{
        factory::get_tokio_mq_factory,
        message_bus::{
            MessageBus, MessageBusReceiver, MessageBusSender, MpscMessageBus,
            SpmcMessageBusReceiver, SpmcMessageBusSender, StaticAsyncComponent,
            StaticClonableAsyncComponent, StaticClonableMpscMQ, StaticMpscMQReceiver,
        },
        tokio_channel_mq::{
            TokioMpscMessageBusReceiver, TokioMpscMessageBusSender, TokioSpmcMessageBusReceiver,
            TokioSpmcMessageBusSender,
        },
    },
};

use super::{
    sync_rate_limiter::{new_web_request_limiter, WebRequestRateLimiter},
    worker::SyncWorkerDataMPSCReceiver,
};

pub type QueueId = Uuid;
type CooldownTimerTask = JoinHandle<()>;
type TimeSecondLeft = i64;

// Factory Methods
pub fn new_empty_queue<T: RateLimiter>(
    rate_limiter: T,
    max_retry: Option<u32>,
) -> SyncTaskQueue<T> {
    return SyncTaskQueue::<T>::new(vec![], Some(rate_limiter), max_retry);
}

pub fn new_empty_queue_limitless<T: RateLimiter>(max_retry: Option<u32>) -> SyncTaskQueue<T> {
    return SyncTaskQueue::new(vec![], None, max_retry);
}

pub fn create_sync_task_manager<
    T: RateLimiter + 'static,
    MT: SyncTaskMPSCSender,
    ME: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
>(
    create_request: &CreateTaskManagerRequest,
    task_sender: MT,
    error_sender: ME,
    failed_task_receiver: MF,
) -> TaskManager<T, MT, ME, MF>
where
    Vec<SyncTaskQueue<T>>: FromIterator<SyncTaskQueue<WebRequestRateLimiter>>,
{
    // todo: use abstract factory and factory methods to avoid hard coding
    // for now, we only have a few impls so we can simply hardcode the factory.
    let queues: Vec<SyncTaskQueue<T>> = create_request
        .create_task_queue_requests()
        .iter()
        .map(|req| match req.rate_limiter_param() {
            Some(create_limiter_req) => match req.rate_limiter_impl() {
                WebRequestRateLimiter => {
                    let rate_limiter = new_web_request_limiter(
                        *create_limiter_req.max_request(),
                        *create_limiter_req.max_daily_request(),
                        *create_limiter_req.cooldown(),
                    );
                    let queue = new_empty_queue(rate_limiter, *req.max_retry());
                    return queue;
                }
            },
            None => {
                let queue = new_empty_queue_limitless(*req.max_retry());
                return queue;
            }
        })
        .collect();

    let task_manager = TaskManager::new(queues, task_sender, error_sender, failed_task_receiver);
    return task_manager;
}

// Component Definitions
#[derive(Debug)]
pub enum SyncTaskQueueValue {
    Task(Option<SyncTask>),
    RateLimited(Option<CooldownTimerTask>, TimeSecondLeft), // timer task, seco
    DailyLimitExceeded,
}

#[derive(Derivative, Getters, Setters, Debug)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct SyncTaskQueue<T: RateLimiter> {
    id: Uuid,
    tasks: VecDeque<SyncTask>,
    rate_limiter: Option<T>,
    max_retry: Option<u32>,
    retries_left: Option<u32>,
}

impl<T: RateLimiter> SyncTaskQueue<T> {
    pub fn new(
        tasks: Vec<SyncTask>,
        rate_limiter: Option<T>,
        max_retry: Option<u32>,
    ) -> SyncTaskQueue<T> {
        let task_queue = VecDeque::from(tasks);
        SyncTaskQueue {
            id: Uuid::new_v4(),
            tasks: task_queue,
            rate_limiter,
            max_retry,
            retries_left: max_retry,
        }
    }

    pub async fn pop_front(&mut self) -> Result<SyncTaskQueueValue, TimerError> {
        //! try to pop the front of the task queue
        //! if the queue is empty, or the queue has a rate limiter, and the rate limiter rejects the request, return None
        // let mut q_lock = self.tasks.lock().await;
        match &mut self.rate_limiter {
            Some(rate_limiter) => {
                let rate_limiter_response = rate_limiter.can_proceed().await;
                match rate_limiter_response {
                    RateLimitStatus::Ok(available_request_left) => {
                        println!(
                            "Rate limiter permits this request. There are {} requests left.",
                            available_request_left
                        );
                        let value = self.tasks.pop_front();
                        if let Some(value) = value {
                            return Ok(SyncTaskQueueValue::Task(Some(value)));
                        } else {
                            return Ok(SyncTaskQueueValue::Task(None));
                        }
                    }
                    RateLimitStatus::RequestPerDayExceeded => {
                        return Ok(SyncTaskQueueValue::DailyLimitExceeded);
                    }
                    RateLimitStatus::RequestPerMinuteExceeded(
                        should_start_cooldown,
                        seconds_left,
                    ) => {
                        if should_start_cooldown {
                            let countdown_task = rate_limiter.start_countdown(true).await?;
                            return Ok(SyncTaskQueueValue::RateLimited(
                                Some(countdown_task),
                                seconds_left,
                            ));
                        } else {
                            return Ok(SyncTaskQueueValue::RateLimited(None, seconds_left));
                        }
                    }
                }
            }
            None => {
                let value = self.tasks.pop_front();
                if let Some(value) = value {
                    return Ok(SyncTaskQueueValue::Task(Some(value.clone())));
                } else {
                    return Ok(SyncTaskQueueValue::Task(None));
                }
            }
        }
    }

    pub fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Vec<SyncTask> {
        //! Pops all elements in the queue given the range
        //! Typically used when the remote reports a daily limited reached error
        // let mut q_lock = .lock().await;
        let values = self.tasks.drain(range);
        return values.collect::<Vec<_>>();
    }

    pub fn push_back(&mut self, task: SyncTask) {
        self.tasks.push_back(task);
        return ();
    }

    pub fn push_front(&mut self, task: SyncTask) {
        self.tasks.push_front(task);
        return ();
    }

    pub fn front(&self) -> Option<SyncTask> {
        self.tasks.front().cloned()
    }

    pub fn is_empty(&self) -> bool {
        return self.tasks.is_empty();
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

#[async_trait]
pub trait SyncTaskManager {
    async fn start(&mut self) -> Result<(), TaskManagerError>;
    async fn stop(&mut self);
    async fn add_tasks_to_queue(&mut self, queue_id: QueueId, tasks: Vec<SyncTask>);
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

/// TaskManager
#[derive(Derivative, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub")]
pub struct TaskManager<T, MT, ME, MF>
where
    T: RateLimiter,
    MT: SyncTaskMPSCSender,
    ME: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    queues: Arc<RwLock<HashMap<QueueId, Arc<Mutex<SyncTaskQueue<T>>>>>>,
    task_channel: MT,
    error_message_channel: ME,
    failed_task_channel: MF,
}

impl<T, MT, ME, MF> TaskManager<T, MT, ME, MF>
where
    T: RateLimiter,
    MT: SyncTaskMPSCSender,
    ME: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    pub fn new(
        task_queues: Vec<SyncTaskQueue<T>>,
        task_channel: MT,
        error_message_channel: ME,
        failed_task_channel: MF,
    ) -> TaskManager<T, MT, ME, MF> {
        let mut q_map = HashMap::new();
        task_queues.into_iter().for_each(|q| {
            q_map.insert(q.id().clone(), Arc::new(Mutex::new(q)));
        });
        Self {
            queues: Arc::new(RwLock::new(q_map)),
            task_channel,
            error_message_channel,
            failed_task_channel,
        }
    }

    pub async fn add_queue(&mut self, task_queue: SyncTaskQueue<T>) {
        let mut q_lock = self.queues.write().await;
        q_lock.insert(task_queue.id().clone(), Arc::new(Mutex::new(task_queue)));
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

    /// Check whether all queues are empty. If so, the
    pub async fn all_queues_are_empty(&self) -> bool {
        let queues = self.queues.read().await;
        let is_empty = queues.values().all(|q| {
            let q_lock = q.blocking_lock();
            return q_lock.is_empty();
        });
        return false;
    }
}

#[async_trait]
impl<T, MT, ME, MF> SyncTaskManager for TaskManager<T, MT, ME, MF>
where
    T: RateLimiter + 'static,
    MT: SyncTaskMPSCSender + Clone,
    ME: TaskManagerErrorMPSCSender + Clone,
    MF: FailedTaskSPMCReceiver + Clone,
{
    async fn stop(&mut self) {
        self.task_channel.close();
        self.error_message_channel.close();
        self.failed_task_channel.close();
    }

    async fn add_tasks_to_queue(&mut self, queue_id: QueueId, tasks: Vec<SyncTask>) {
        self.add_tasks_to_queue(queue_id, tasks).await;
    }

    /// start task manager and push tasks to its consumers
    /// Task manager will poll its queues and try to get a task from each of them, and then send the task to task channel
    async fn start(&mut self) -> Result<(), TaskManagerError> {
        let failed_task_channel = Arc::new(Mutex::new(self.failed_task_channel.clone()));
        let queues = Arc::clone(&self.queues);
        let task_channel = self.task_channel.clone();
        let error_message_channel = self.error_message_channel.clone();

        let fetch_tasks: Vec<_> = queues.read().await
            .iter()
            .map(|(q_id, task_queue)| {
                let q_id = q_id.clone();
                let queue = Arc::clone(task_queue); // Cloning it here
                let new_task_sender_channel = task_channel.clone();
                let new_error_sender_channel = error_message_channel.clone();
                // let failed_task_channel = failed_task_channel.clone();

                tokio::spawn(async move {
                    let q_id = q_id.clone();
                    let q_lock = queue.lock().await;
                    println!("Acquired lock for queue {:?}", q_id);
                    let mut no_task_found = false;
                    drop(q_lock);
                    println!("Released lock for queue {:?}", q_id);

                    loop {
                        if no_task_found {
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                        }
                        let mut q_lock = queue.lock().await;
                        println!("Acquired lock for queue {:?} to fetch a task", q_id);

                        let q_is_empty = q_lock.is_empty();
                        if q_is_empty {
                            break;
                        }

                        let task_value = q_lock.pop_front().await;
                        drop(q_lock);
                        println!("Released lock for queue {:?} after fetching a task", q_id);

                        match task_value {
                            Ok(value) => {
                                match value {
                                    SyncTaskQueueValue::Task(t) => {
                                        if let Some(t) = t {
                                            // Send the task to its consumer
                                            let task_id = t.id();
                                            let r = new_task_sender_channel.send(t.clone()).await;

                                            match r {
                                                Ok(()) => {
                                                    println!("Sent task {:?} in queue {}", task_id, q_id);
                                                    no_task_found = false;
                                                },
                                                Err(e) => {
                                                    println!("Failed to send task because {:?}", e);
                                                    let mut q_lock = queue.lock().await;
                                                    println!("Acquired lock for queue {:?} to push back a task", q_id);
                                                    q_lock.retry(t);
                                                }
                                            }
                                        } else {
                                            println!("Received no task from queue {}!", q_id);
                                        }
                                    },
                                    SyncTaskQueueValue::RateLimited(cooldown_task, time_left) => {
                                        println!("Error! Rate limited! time left: {}", time_left);
                                        if let Some(ct) = cooldown_task {
                                            let _ = new_error_sender_channel
                                            .send(TaskManagerError::RateLimited(Some(Arc::new(ct)), time_left))
                                            .await;
                                        }
                                    },
                                    SyncTaskQueueValue::DailyLimitExceeded => {
                                        // tell task manager's consumers that daily limit is triggered
                                        // TODO: figure out what to do with it? Does task manager just tell others this error or do something further?
                                        println!("Error! DailyLimitExceeded!");
                                        let _ = new_error_sender_channel
                                            .send(TaskManagerError::DailyLimitExceeded)
                                            .await;
                                    }
                                }
                            },
                            Err(e) => {
                                let result = new_error_sender_channel.send(TaskManagerError::RateLimiterInitializationError(e)).await;
                                if let Err(e) = result {
                                    error!("{:?}",e);
                                }
                            }
                        }
                    }
                    // Recommend dropping channels explicitly!
                    drop(new_task_sender_channel);
                    println!("Dropped task sender in queue {}!", q_id);
                    drop(new_error_sender_channel);
                    println!("Dropped error sender in queue {}!", q_id);
                    println!("Queue {} has no task left. Quitting...", q_id);
                })
            })
            .collect();

        let queues_ref = Arc::clone(&queues);
        let handle_failures = tokio::spawn(async move {
            loop {
                let mut failed_task_channel_lock = failed_task_channel.lock().await;
                let receive_result = failed_task_channel_lock.try_recv();
                match receive_result {
                    Ok((qid, failed_task)) => {
                        let queues = queues_ref.read().await;
                        if let Some(q) = queues.get(&qid) {
                            let mut q_lock = q.lock().await;
                            if let Some(mut n_retry) = q_lock.retries_left() {
                                if n_retry > 0 {
                                    q_lock.push_back(failed_task);
                                    q_lock.set_retries_left(Some(n_retry - 1));
                                }
                            }
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

        println!("Waiting for all tasks to complete.");

        let _ = join!(join_all(fetch_tasks), handle_failures);

        println!("Done!");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::synchronization::rate_limiter::{RateLimitStatus, RateLimiter},
        domain::synchronization::value_objects::task_spec::{RequestMethod, TaskSpecification},
        infrastructure::{
            mq::factory::{create_tokio_mpsc_channel, create_tokio_spmc_channel},
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

    pub async fn new_queue_with_random_amount_of_tasks<T: RateLimiter>(
        rate_limiter: T,
        min_tasks: u32,
        max_tasks: u32,
    ) -> SyncTaskQueue<T> {
        let mut rng = rand::thread_rng();
        let max_retry = rng.gen_range(10..20);
        let mut new_queue = new_empty_queue(rate_limiter, Some(max_retry));
        // let mut new_queue = new_empty_queue_limitless(None);
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
        min_tasks: u32,
        max_tasks: u32,
    ) -> Vec<SyncTaskQueue<WebRequestRateLimiter>> {
        let queues = join_all((0..n).map(|_| async {
            let mut rng = rand::thread_rng();
            let max_request: i64 = rng.gen_range(30..100);
            let cooldown: i64 = rng.gen_range(1..3);
            let limiter = new_web_request_limiter(max_request, None, Some(cooldown));
            let q = new_queue_with_random_amount_of_tasks(limiter, min_tasks, max_tasks).await;
            return q;
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
        let task_queue: Arc<Mutex<SyncTaskQueue<WebRequestRateLimiter>>> =
            Arc::new(Mutex::new(SyncTaskQueue::<WebRequestRateLimiter>::new(
                vec![],
                Some(test_rate_limiter),
                Some(max_retry),
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

    #[tokio::test]
    async fn test_producing_and_consuming_tasks() {
        init();

        // pressure testing
        let min_task = 5;
        let max_task = 10;
        let n_queues = 5;

        let queues = generate_queues_with_web_request_limiter(n_queues, min_task, max_task).await;

        let (task_sender, mut task_receiver) = create_tokio_mpsc_channel::<SyncTask>(1000);

        let (error_msg_sender, mut error_msg_receiver) =
            create_tokio_mpsc_channel::<TaskManagerError>(500);

        let (failed_task_sender, failed_task_receiver) =
            create_tokio_spmc_channel::<(Uuid, SyncTask)>(500);

        let mut task_manager =
            TaskManager::new(queues, task_sender, error_msg_sender, failed_task_receiver);

        let send_task = tokio::spawn(async move {
            let result = task_manager.start().await;
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

        let receive_task = tokio::spawn(async move {
            info!("Before acquiring the mq lock...");

            info!("MQ lock acquired...");
            let mut task_cnt = 0;
            let recv_task = tokio::spawn(async move {
                loop {
                    let result = task_receiver.receive().await;
                    if let Some(_) = result {
                        task_cnt += 1;
                        info!("Received task, count: {:?}", task_cnt);
                    } else {
                        // Important! Use this branch to exit.
                        info!("No task left... Exit...");
                        break;
                    }
                }
            });
            let err_report_task = tokio::spawn(async move {
                loop {
                    let result = error_msg_receiver.receive().await;
                    if let Some(e) = result {
                        error!("Received error, {:?}", e);
                    } else {
                        // Important! Use this branch to exit.
                        info!("No error received.");
                        break;
                    }
                }
            });

            let _ = join!(recv_task, err_report_task);

            // let _ = task_channel_lock.close();
            info!("Done receiving tasks...");
            info!("Receiver tries to release the mq lock...");
            // drop(task_channel_lock);
            info!("MQ lock released...");
        });

        // FIXME: loop will not quit after finishing tasks
        let _ = tokio::join!(send_task, receive_task);
        // let _ = tokio::join!(send_task,);
    }
}
