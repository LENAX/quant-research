//! Task Manager
//! Defines synchronization task queue and a manager module to support task polling, scheduling and throttling.
//!

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use futures::{
    executor::block_on,
    future::{join_all, try_join_all},
};
use getset::{Getters, Setters};
use log::{error, info, warn};
use tokio::{
    join,
    sync::{Mutex, RwLock},
    time::{sleep, Duration},
};
use uuid::Uuid;

use crate::{
    domain::synchronization::{
        rate_limiter::RateLimiter, sync_plan::SyncPlan, sync_task::SyncTask, value_objects::sync_config::RateLimiterImpls,
    },
    infrastructure::sync::{
        shared_traits::{FailedTaskSPMCReceiver, SyncTaskMPMCSender, TaskRequestMPMCReceiver},
        task_manager::errors::QueueError,
    },
};

use super::{
    errors::TaskManagerError,
    factory::{new_empty_limitless_queue, new_empty_queue, create_rate_limiter_by_rate_quota, RateLimiterInstance},
    tm_traits::{SyncTaskManager, TaskManagerErrorMPSCSender, TaskSendingProgress}, sync_rate_limiter::WebRequestRateLimiter,
};
use crate::infrastructure::sync::task_manager::sync_task_queue::QueueId;
use crate::infrastructure::sync::task_manager::sync_task_queue::SyncTaskQueue;

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TaskManagerState {
    #[derivative(Default)]
    Initialized,
    Running,
    Stopped,
}

// Hint: Use trait to separate TaskManager and SyncTaskQueue
// Then use generics with trait bound
// In this way, there is no need to pass extra generic param to build the struct.

/// TaskManager
/// TaskManger is responsible for sending tasks for each sync plan upon receiving request.
#[derive(Derivative, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub")]
pub struct TaskManager<RL, MT, TR, ES, MF>
where
    RL: RateLimiter,
    MT: SyncTaskMPMCSender,
    TR: TaskRequestMPMCReceiver,
    ES: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    queues: Arc<RwLock<HashMap<QueueId, Arc<Mutex<SyncTaskQueue<RL, TR>>>>>>,
    task_sender: MT,
    error_message_channel: ES,
    failed_task_channel: MF,
    current_state: TaskManagerState,
}

impl<RL, MT, TR, ES, MF> TaskManager<RL, MT, TR, ES, MF>
where
    RL: RateLimiter,
    MT: SyncTaskMPMCSender,
    TR: TaskRequestMPMCReceiver,
    ES: TaskManagerErrorMPSCSender,
    MF: FailedTaskSPMCReceiver,
{
    pub fn new(
        task_queues: Vec<SyncTaskQueue<RL, TR>>,
        task_sender: MT,
        error_message_channel: ES,
        failed_task_channel: MF,
    ) -> TaskManager<RL, MT, TR, ES, MF> {
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

    pub async fn add_queue(&mut self, task_queue: SyncTaskQueue<RL, TR>) {
        let mut q_lock = self.queues.write().await;
        q_lock.insert(*task_queue.sync_plan_id(), Arc::new(Mutex::new(task_queue)));
    }

    pub async fn add_tasks_to_queue(&mut self, queue_id: QueueId, tasks: Vec<Arc<Mutex<SyncTask>>>) {
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
impl<RL, MT, TR, ES, MF> SyncTaskManager for TaskManager<RL, MT, TR, ES, MF>
where
    RL: RateLimiter + 'static,
    MT: SyncTaskMPMCSender + Clone,
    TR: TaskRequestMPMCReceiver,
    ES: TaskManagerErrorMPSCSender + Clone,
    MF: FailedTaskSPMCReceiver + Clone,
{
    type RateLimiterType = RL;
    type TaskReceiverType = TR;

    async fn add_tasks_to_plan(
        &mut self,
        plan_id: Uuid,
        tasks: Vec<Arc<Mutex<SyncTask>>>,
    ) -> Result<(), TaskManagerError> {
        self.add_tasks_to_queue(plan_id, tasks).await;
        Ok(())
    }

    async fn stop_sending_all_tasks(
        &mut self,
    ) -> Result<HashMap<Uuid, Vec<Arc<Mutex<SyncTask>>>>, TaskManagerError> {
        let queues = self.queues.read().await;
        let mut unsent_tasks = HashMap::new();
        for queue in queues.values() {
            let mut queue_lock = queue.lock().await;
            let remaining_tasks = queue_lock.drain_all(); // Drop all tasks
            unsent_tasks.insert(*queue_lock.sync_plan_id(), remaining_tasks);
        }
        Ok(unsent_tasks)
    }

    async fn load_sync_plan<R: RateLimiter + 'static, T: TaskRequestMPMCReceiver>(
        &mut self,
        sync_plan: Arc<Mutex<SyncPlan>>,
        rate_limiter: Option<R>,
        task_request_receiver: T,
    ) -> Result<(), TaskManagerError> {
        let plan_lock = sync_plan.lock().await;
        let sync_config = plan_lock.sync_config();
        let plan_id = *plan_lock.id();
        
        match rate_limiter {
            Some(rate_limiter) => {
                let quota = sync_config.sync_rate_quota()
                    .as_ref()
                    .ok_or(TaskManagerError::MissingRateLimitParam)?;


                let mut queue = new_empty_queue(rate_limiter, task_request_receiver, Some(*quota.max_retry()), plan_id);
                plan_lock.tasks().iter().for_each(|t| {queue.push_back(Arc::new(Mutex::new(*t)))});
                
                self.add_queue(queue).await;
                Ok(())
            }
            None => match sync_config.sync_rate_quota() {
                None => {
                    let mut queue = new_empty_limitless_queue(None, plan_id, task_request_receiver);
                    plan_lock.tasks().iter().for_each(|t| queue.push_back(Arc::new(Mutex::new(*t))));

                    self.add_queue(queue).await;
                    Ok(())
                },
                Some(_) => Err(TaskManagerError::RateLimiterNotSet(
                    String::from("Missing rate limiter while expecting to pass one because you have specified rate quota!"))),
            },
        }
    }

    async fn load_sync_plan_with_limiter(
        &mut self,
        sync_plan: Arc<Mutex<SyncPlan>>,
        task_request_receiver: TR,
    ) -> Result<(), TaskManagerError> {
        let plan_lock = sync_plan.lock().await;
        let rate_quota = plan_lock.sync_config().sync_rate_quota();
        drop(plan_lock);

        match rate_quota {
            Some(quota) =>  {
                // let rate_limiter_impl = quote.use_impl;
                let rate_limiter_instance = create_rate_limiter_by_rate_quota(quota);
                match rate_limiter_instance {
                    RateLimiterInstance::WebLimiter(limiter) => {
                        return self.load_sync_plan(sync_plan, Some(limiter), task_request_receiver).await;
                    }
                }
            },
            None => {
                return self.load_sync_plan(sync_plan, None, task_request_receiver).await;
            }
        }
    }

    async fn load_sync_plans(
        &mut self,
        sync_plans: Vec<Arc<Mutex<SyncPlan>>>,
        rate_limiters: Vec<Option<RL>>,
        task_request_receivers: Vec<TR>,
    ) -> Result<(), TaskManagerError> {
        for ((plan, limiter), task_request_receiver) in sync_plans
            .iter()
            .zip(rate_limiters)
            .zip(task_request_receivers)
        {
            let result = self
                .load_sync_plan(*plan, limiter, task_request_receiver)
                .await;
            if let Err(e) = result {
                error!("{:?}", e);
                return Err(TaskManagerError::BatchPlanInsertionFailed(
                    String::from("Batch task insertion failed due to inconsistencies between rate limit param and actual passed rate limiter type.")));
            }
        }

        Ok(())
    }

    async fn load_sync_plans_with_rate_limiter(
        &mut self,
        sync_plans: Vec<Arc<Mutex<SyncPlan>>>,
        task_request_receivers: Vec<TR>,
    ) -> Result<(), TaskManagerError> {
        todo!();
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
    ) -> Result<Vec<Arc<Mutex<SyncTask>>>, TaskManagerError> {
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
            let progress =
                TaskSendingProgress::new(sync_plan_id, n_task_sent, total_tasks, complete_rate);
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
                                let r = new_task_sender_channel.send(Arc::clone(&task)).await;

                                match r {
                                    Ok(()) => {
                                        info!("Sent 1 task in queue {:#?}", q_id);
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
                        let queues: tokio::sync::RwLockReadGuard<'_, HashMap<Uuid, Arc<Mutex<SyncTaskQueue<RL, TR>>>>> = queues_ref.read().await;
                        let task_lock = failed_task.lock().await;
                        let q_key = task_lock.sync_plan_id().unwrap_or(Uuid::new_v4());
                        drop(task_lock);

                        if let Some(q) = queues.get(&q_key) {
                            let mut q_lock = q.lock().await;
                            if let Some(mut n_retry) = q_lock.retries_left() {
                                if n_retry > 0 {
                                    // Ensure the order of fetched data
                                    q_lock.push_front(failed_task);
                                    q_lock.set_retries_left(Some(n_retry - 1));
                                }
                            }
                        } else {
                            error!("Queue not found. Perhaps sync task is not tied to a dataset id and sync plan id. Arc<Mutex<SyncTask>>: {:#?}", failed_task);
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
        domain::synchronization::rate_limiter::RateLimiter,
        domain::synchronization::value_objects::task_spec::{RequestMethod, TaskSpecification},
        infrastructure::{
            mq::{
                factory::create_tokio_broadcasting_channel,
                tokio_channel_mq::{
                    TokioBroadcastingMessageBusReceiver, TokioBroadcastingMessageBusSender,
                },
            },
            sync::{
                task_manager::sync_rate_limiter::WebRequestRateLimiter,
                GetTaskRequest,
            },
            // sync::{sync_rate_limiter::{new_web_request_limiter, WebRequestRateLimiter}, GetTaskRequest},
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

    pub fn generate_random_sync_tasks(n: u32) -> Vec<Arc<Mutex<SyncTask>>> {
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
                Arc::new(Mutex::new(SyncTask::new(
                    Uuid::new_v4(),
                    &dataset_name.unwrap(),
                    Uuid::new_v4(),
                    &datasource_name.unwrap(),
                    task_spec,
                    Uuid::new_v4(),
                    Some(10),
                )))
            })
            .collect()
    }

    pub fn new_queue_with_random_amount_of_tasks<RL: RateLimiter, TR: TaskRequestMPMCReceiver>(
        rate_limiter: RL,
        task_request_receiver: TR,
        min_tasks: u32,
        max_tasks: u32,
    ) -> SyncTaskQueue<RL, TR> {
        let mut rng = rand::thread_rng();
        let max_retry = rng.gen_range(10..20);
        let sync_plan_id = Uuid::new_v4();
        let mut new_queue = new_empty_queue(
            rate_limiter,
            task_request_receiver,
            Some(max_retry),
            sync_plan_id,
        );
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
    ) -> Vec<(
        SyncTaskQueue<WebRequestRateLimiter, TokioBroadcastingMessageBusReceiver<GetTaskRequest>>,
        TokioBroadcastingMessageBusSender<GetTaskRequest>,
    )> {
        let queues = join_all((0..n).map(|_| async {
            let mut rng = rand::thread_rng();
            let max_request: u32 = rng.gen_range(30..100);
            let cooldown: u32 = rng.gen_range(1..3);
            let limiter = new_web_request_limiter(max_request, None, Some(cooldown));
            let (task_req_sender, task_req_receiver) =
                create_tokio_broadcasting_channel::<GetTaskRequest>(channel_size);
            let q = new_queue_with_random_amount_of_tasks(
                limiter,
                task_req_receiver,
                min_tasks,
                max_tasks,
            );
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
        let (task_req_sender, task_req_receiver) =
            create_tokio_broadcasting_channel::<GetTaskRequest>(100);
        let task_queue: Arc<
            Mutex<
                SyncTaskQueue<
                    WebRequestRateLimiter,
                    TokioBroadcastingMessageBusReceiver<GetTaskRequest>,
                >,
            >,
        > = Arc::new(Mutex::new(SyncTaskQueue::<
            WebRequestRateLimiter,
            TokioBroadcastingMessageBusReceiver<GetTaskRequest>,
        >::new(
            vec![],
            Some(test_rate_limiter),
            Some(max_retry),
            Uuid::new_v4(),
            task_req_receiver,
        )));

        let tasks = generate_random_sync_tasks(100);
        let first_task = tasks[0].clone();

        let mut task_queue_lock = task_queue.lock().await;

        for t in tasks {
            task_queue_lock.push_back(t);
            info!("task queue size: {}", task_queue_lock.len());
        }

        if let Some(t1) = task_queue_lock.front() {
            let t_lock = t1.lock().await;
            let first_task_lock = first_task.lock().await;
            assert_eq!(t_lock.id(), first_task_lock.id(), "Task id not equal!")
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

    //     let (task_sender, mut task_receiver) = create_tokio_mpsc_channel::<Arc<Mutex<SyncTask>>>(1000);

    //     let (error_msg_sender, mut error_msg_receiver) =
    //         create_tokio_mpsc_channel::<TaskManagerError>(500);

    //     let (failed_task_sender, failed_task_receiver) =
    //         create_tokio_spmc_channel::<(Uuid, Arc<Mutex<SyncTask>>)>(500);

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
