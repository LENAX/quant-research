//! Task Manager
//! Defines synchronization task queue and a manager module to support task polling, scheduling and throttling.
//!

use std::{
    collections::{HashMap, VecDeque},
    error::{Error, self},
    ops::RangeBounds,
    sync::Arc, fmt,
};

use async_trait::async_trait;
use derivative::Derivative;
use futures::future::join_all;
use getset::{Getters, Setters};

use rbatis::dark_std::sync::vec;
use tokio::{join, sync::{Mutex, RwLock}, task::JoinHandle};
use uuid::Uuid;

use crate::{
    domain::synchronization::{
        custom_errors::TimerError,
        rate_limiter::{RateLimitStatus, RateLimiter},
        sync_task::SyncTask,
    },
    infrastructure::mq::message_bus::{
        MessageBus, MessageBusReceiver, MessageBusSender, MpscMessageBus,
        SpmcMessageBus
    },
};

type QueueId = Uuid;
type CooldownTimerTask = JoinHandle<()>;
type TimeSecondLeft = i64;

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
    retries_left: Option<u32>
}

impl<T: RateLimiter> SyncTaskQueue<T> {
    pub fn new(tasks: Vec<SyncTask>, rate_limiter: Option<T>, max_retry: Option<u32>) -> SyncTaskQueue<T> {
        let task_queue = VecDeque::from(tasks);
        SyncTaskQueue {
            id: Uuid::new_v4(),
            tasks: task_queue,
            rate_limiter,
            max_retry,
            retries_left: max_retry
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
}




#[derive(Debug, Clone)]
pub enum TaskManagerError {
    RateLimited(Option<Arc<CooldownTimerTask>>, TimeSecondLeft), // timer task, seco
    DailyLimitExceeded,
    RateLimiterInitializationError(TimerError),
    OtherError(String)
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
}

type MaxRetry = u32;

/// TaskManager
#[derive(Derivative, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub")]
pub struct TaskManager<T, MT, ME, MF>
where
    T: RateLimiter,
    MT: MessageBusSender<SyncTask> + MpscMessageBus,
    ME: MessageBusSender<TaskManagerError> + MpscMessageBus,
    MF: MessageBusReceiver<(QueueId, SyncTask)>,
{
    // TODO: Change queues to Arc<HashMap<QueueId, Arc<Mutex<SyncTaskQueue<T>>>>>
    // so that individual queues can be cloned and shared across threads
    queues: Arc<RwLock<HashMap<QueueId, Arc<Mutex<SyncTaskQueue<T>>>>>>,
    task_channel: MT,
    error_message_channel: ME,
    failed_task_channel: MF,
}

impl<T, MT, ME, MF> TaskManager<T, MT, ME, MF>
where
    T: RateLimiter,
    MT: MessageBusSender<SyncTask> + MpscMessageBus,
    ME: MessageBusSender<TaskManagerError> + MpscMessageBus,
    MF: MessageBusReceiver<(QueueId, SyncTask)>,
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
    T: RateLimiter,
    MT: MessageBusSender<SyncTask> + MpscMessageBus + std::marker::Sync + Clone + std::marker::Send,
    ME: MessageBusSender<TaskManagerError> + MpscMessageBus + std::marker::Sync + Clone + std::marker::Send,
    MF: MessageBusReceiver<(QueueId, SyncTask)> + std::marker::Sync + std::marker::Send,
{
    async fn stop(&mut self) {
        self.task_channel.close();
        self.error_message_channel.close();
        self.failed_task_channel.close();
    }

    /// start task manager and push tasks to its consumers
    /// Task manager will poll its queues and try to get a task from each of them, and then send the task to task channel
    async fn start(&mut self) -> Result<(), TaskManagerError> {
        let failed_task_channel_lock = Arc::new(Mutex::new(&mut self.failed_task_channel));
        let new_error_sender = Arc::new(self.error_message_channel.clone());

        let queues = self.queues.read().await;
        let fetch_tasks = queues.iter().map(|(q_id, task_queue)| async {
            let queue = Arc::clone(task_queue);
            let new_task_sender_channel = self.task_channel.clone();
            let new_error_sender_channel = Arc::clone(&new_error_sender);
            let q_lock = queue.lock().await;
            let q_id = q_lock.id().clone();
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
                                            q_lock.push_back(t);
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
                        self.error_message_channel.send(TaskManagerError::RateLimiterInitializationError(e));
                    }
                }
            }
            println!("Queue {} has no task left. Quitting...", q_id);
        });

        let handle_failures = || async {
            while let Some((dataset_id, failed_task)) =
                failed_task_channel_lock.lock().await.receive().await {
                let queues = self.queues.read().await;
                if let Some(q) = queues.get(&dataset_id) {
                    let mut q_lock = q.lock().await;
                    if let Some(mut n_retry) = q_lock.retries_left() {
                        if n_retry > 0 {
                            q_lock.push_back(failed_task);
                            q_lock.set_retries_left(Some(n_retry - 1));
                        }
                    }
                }
            }
        };

        let task_handles = fetch_tasks.map(|t| {
            tokio::spawn(t)
        });
        
        
        println!("Waiting for all tasks to complete.");

        // join!(
        //     join_all(fetch_tasks),
        //     handle_failures()
        // );

        join!(
            join_all(task_handles),
            handle_failures()
        );
        
        println!("Done!");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        domain::synchronization::value_objects::task_spec::{TaskSpecification, RequestMethod},
        domain::synchronization::rate_limiter::{RateLimitStatus, RateLimiter},
        infrastructure::{
            sync::sync_rate_limiter::WebRequestRateLimiter,
            mq::tokio_channel_mq::{TokioMpscMessageBus, create_tokio_mpsc_channel, create_tokio_spmc_channel}},
    };
    use log::{info, error};


    use super::*;
    use std::{sync::Arc, hash::Hash};

    use chrono::Local;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::name::en::Name;
    use fake::Fake;
    use rand::Rng;
    use serde_json::Value;
    use url::Url;
    use env_logger;
    use std::env;

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
        (0..n).map(|_| {
            let fake_url = format!("http://{}", SafeEmail().fake::<String>());
            let request_endpoint = Url::parse(&fake_url).unwrap();
            let fake_headers: HashMap<String, String> = (0..5).map(|_| (Name().fake::<String>(), random_string(20))).collect();
            let fake_payload = Some(Arc::new(Value::String(random_string(50))));
            let fake_method = if rand::random() { RequestMethod::Get } else { RequestMethod::Post };
            let task_spec = TaskSpecification::new(&fake_url, if fake_method == RequestMethod::Get { "GET" } else { "POST" }, fake_headers, fake_payload).unwrap();

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
                Uuid::new_v4()
            )
        }).collect()
    }

    pub fn new_web_request_limiter(max_request: i64, max_daily_request: Option<i64>, cooldown: Option<i64>) -> WebRequestRateLimiter{
        return WebRequestRateLimiter::new(max_request, max_daily_request, cooldown).unwrap();
    }

    pub fn new_empty_queue<T: RateLimiter>(rate_limiter: T, max_retry: Option<u32>) -> SyncTaskQueue<T> {
        return SyncTaskQueue::<T>::new(vec![], Some(rate_limiter), max_retry);
    }

    pub fn new_empty_queue_limitless<T: RateLimiter>(max_retry: Option<u32>) -> SyncTaskQueue<T> {
        return SyncTaskQueue::new(vec![], None, max_retry);
    }

    pub async fn new_queue_with_random_amount_of_tasks<T: RateLimiter>(rate_limiter: T, min_tasks: u32, max_tasks: u32) -> SyncTaskQueue<T> {
        let mut rng = rand::thread_rng();
        let mut new_queue = new_empty_queue(rate_limiter);
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

    pub async fn generate_queues_with_web_request_limiter(n: u32, min_tasks: u32, max_tasks: u32) -> Vec<SyncTaskQueue<WebRequestRateLimiter>> {
        let queues = join_all((0..n).map(|_| async {
            let mut rng = rand::thread_rng();
            let max_request: i64 = rng.gen_range(30..100);
            let cooldown: i64 = rng.gen_range(1..3);
            let limiter = new_web_request_limiter(max_request, None, Some(cooldown));
            let q = new_queue_with_random_amount_of_tasks(limiter, min_tasks, max_tasks).await;
            return q;
        })).await;
        return queues;
    }


    #[test]
    fn it_should_generate_random_tasks(){
        let tasks = generate_random_sync_tasks(10);
        assert!(tasks.len() == 10);
        // info!("{:?}", tasks);
    }

    #[tokio::test]
    async fn test_add_tasks_to_a_single_queue() {
        init();
        // Arrange
        let test_rate_limiter = WebRequestRateLimiter::new(30, None, Some(3)).unwrap();
        let task_queue: Arc<Mutex<SyncTaskQueue<WebRequestRateLimiter>>> = Arc::new(Mutex::new(
            SyncTaskQueue::<WebRequestRateLimiter>::new(vec![], Some(test_rate_limiter))
        ));

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

        let min_task = 100;
        let max_task = 120;
        let n_queues = 50;

        // FIXME: RateLimiter may cause deadlock
        let mut manager_queue_map: HashMap<Uuid, (SyncTaskQueue<WebRequestRateLimiter>, u32)> = HashMap::new();
        generate_queues_with_web_request_limiter(n_queues, min_task, max_task).await
            .into_iter()
            .for_each(|q| {
                manager_queue_map.insert(uuid::Uuid::new_v4(), (q, 10 as u32));
                return ;
            });

        let (task_sender,
             task_receiver) = create_tokio_mpsc_channel::<SyncTask>(1000);

        let (error_msg_sender,
             error_msg_receiver) = create_tokio_mpsc_channel::<TaskManagerError>(500);

        let (failed_task_sender, 
            failed_task_receiver) = create_tokio_spmc_channel::<(Uuid, SyncTask)>(500);
   

        let mut task_manager = TaskManager::new(
            manager_queue_map, 
            task_sender,
            error_msg_sender,
            failed_task_receiver);
        
        let send_task = tokio::spawn(async move {
            let result = task_manager.start().await;
            info!("result: {:?}", result);
            match result {
                Ok(()) => { info!("Finished successfully!"); },
                Err(e) => { info!("Failed: {}", e); }
            }
        });

        let receive_task = tokio::spawn(async move{
            info!("Before acquiring the mq lock...");

            info!("MQ lock acquired...");
            let mut task_cnt = 0;
            let recv_task = tokio::spawn(async move {
                while let Some(_) = task_receiver.receive().await {
                    task_cnt += 1;
                    info!("Received task, count: {:?}", task_cnt);
                }
            });
            let err_report_task = tokio::spawn(async move {
                while let Some(err) = error_msg_receiver.receive().await {
                    info!("Received err, err: {:?}", err);
                }
            });

            join!(recv_task, err_report_task);
            
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
