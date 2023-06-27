//! Task Manager
//! Defines synchronization task queue and a manager module to support task polling, scheduling and throttling.
//!

use std::{
    collections::{HashMap, VecDeque},
    ops::RangeBounds,
    sync::{Arc, mpsc::SendError}, cell::RefCell,
};

use derivative::Derivative;
use futures::future::join_all;
use getset::{Getters, Setters};
use log::error;
use tokio::{sync::{Mutex, mpsc::{Sender, Receiver}}, task::JoinHandle, join};
use uuid::Uuid;

use crate::{domain::synchronization::{
    custom_errors::TimerError,
    rate_limiter::{RateLimitStatus, RateLimiter},
    // sync_task::SyncTask,
}, infrastructure::mq::message_bus::MessageBus};

type DatasetId = Uuid;
type CooldownTimerTask = JoinHandle<()>;
type TimeSecondLeft = i64;

pub enum SyncTaskQueueValue {
    Task(Option<Arc<Mutex<SyncTask>>>),
    RateLimited(Option<CooldownTimerTask>, TimeSecondLeft), // timer task, seco
    DailyLimitExceeded,
}

pub struct SyncTask {}

#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct SyncTaskQueue<T: RateLimiter> {
    tasks: Mutex<VecDeque<Arc<Mutex<SyncTask>>>>,
    rate_limiter: Option<T>,
}

impl<T: RateLimiter> SyncTaskQueue<T> {
    pub fn new(
        tasks: Vec<Arc<Mutex<SyncTask>>>,
        rate_limiter: Option<T>,
    ) -> SyncTaskQueue<T> {
        let task_queue = Mutex::new(VecDeque::from(tasks));
        if let Some(rate_limiter) = rate_limiter {
            SyncTaskQueue {
                tasks: task_queue,
                rate_limiter: Some(rate_limiter),
            }
        } else {
            SyncTaskQueue {
                tasks: task_queue,
                rate_limiter: None,
            }
        }
    }

    pub async fn pop_front(&mut self) -> Result<SyncTaskQueueValue, TimerError> {
        //! try to pop the front of the task queue
        //! if the queue is empty, or the queue has a rate limiter, and the rate limiter rejects the request, return None
        let mut q_lock = self.tasks.lock().await;
        match &mut self.rate_limiter {
            Some(rate_limiter) => {
                let rate_limiter_response = rate_limiter.can_proceed().await;
                match rate_limiter_response {
                    RateLimitStatus::Ok(available_request_left) => {
                        println!(
                            "Rate limiter permits this request. There are {} requests left.",
                            available_request_left
                        );
                        let value = q_lock.pop_front();
                        if let Some(value) = value {
                            return Ok(SyncTaskQueueValue::Task(Some(value.clone())));
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
                            return Ok(SyncTaskQueueValue::RateLimited(Some(countdown_task), seconds_left));
                        } else {
                            return Ok(SyncTaskQueueValue::RateLimited(None, seconds_left));
                        }
                    }
                }
            }
            None => {
                let value = q_lock.pop_front();
                if let Some(value) = value {
                    return Ok(SyncTaskQueueValue::Task(Some(value.clone())));
                } else {
                    return Ok(SyncTaskQueueValue::Task(None));
                }
            }
        }
    }

    pub async fn drain<R: RangeBounds<usize>>(
        &mut self,
        range: R,
    ) -> Vec<Arc<Mutex<SyncTask>>> {
        //! Pops all elements in the queue given the range
        //! Typically used when the remote reports a daily limited reached error
        let mut q_lock = self.tasks.lock().await;
        let values = q_lock.drain(range);
        return values.collect::<Vec<_>>();
    }

    pub async fn push_back(&mut self, task: Arc<Mutex<SyncTask>>) {
        let mut q_lock = self.tasks.lock().await;
        q_lock.push_back(task);
        return ();
    }

    pub async fn push_front(&mut self, task: Arc<Mutex<SyncTask>>) {
        let mut q_lock = self.tasks.lock().await;
        q_lock.push_front(task);
        return ();
    }

    pub async fn is_empty(&self) -> bool {
        let q_lock = self.tasks.lock().await;
        return q_lock.is_empty();
    }

}

#[derive(Debug)]
pub enum TaskManagerError {
    RateLimited(Option<CooldownTimerTask>, TimeSecondLeft), // timer task, seco
    DailyLimitExceeded,
}

/// TaskManager
#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct TaskManager<
    T: RateLimiter, 
    MT: MessageBus<Arc<Mutex<SyncTask>>>,
    ME: MessageBus<TaskManagerError>,
    MF: MessageBus<(DatasetId, Arc<Mutex<SyncTask>>)>
> {
    queues: HashMap<DatasetId, Arc<Mutex<SyncTaskQueue<T>>>>,
    task_channel: Arc<Mutex<MT>>,
    error_message_channel:Arc<Mutex<ME>>, // TODO: use a T: MessageQueue member to abstract away the communication details
    failed_task_channel: Arc<Mutex<MF>>,
}

impl<T: RateLimiter, 
     MT: MessageBus<Arc<Mutex<SyncTask>>>,
     ME: MessageBus<TaskManagerError>,
     MF: MessageBus<(DatasetId, Arc<Mutex<SyncTask>>)>> TaskManager<T, MT, ME, MF> {
    pub fn new(
        task_queues: Vec<(DatasetId, Arc<Mutex<SyncTaskQueue<T>>>)>,
        task_channel: Arc<Mutex<MT>>,
        error_message_channel: Arc<Mutex<ME>>,
        failed_task_channel: Arc<Mutex<MF>>,
    ) -> TaskManager<T, MT, ME, MF> {
        let queues = task_queues.into_iter().collect::<HashMap<_, _>>();
        Self { queues, task_channel, error_message_channel, failed_task_channel }
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // Check whether all queues are empty
            // break the loop if all queues are empty

            let queues: Vec<Arc<Mutex<SyncTaskQueue<T>>>> = self.queues.values().cloned().collect();
            for queue in queues {
                let mut q_lock = queue.lock().await;
                let task_value = q_lock.pop_front().await;
                // Do something with task_value
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        Ok(())
    }
}