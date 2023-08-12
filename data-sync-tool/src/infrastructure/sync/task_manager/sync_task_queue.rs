/**
 * Synchronization Task Queue
 *
 * A queue for managing sychronization task.
 */
use uuid::Uuid;

use crate::{
    domain::synchronization::{
        rate_limiter::{RateLimitStatus, RateLimiter},
        sync_task::SyncTask,
    },
    infrastructure::sync::factory::Builder,
};

use super::{
    errors::{CooldownTimerTask, QueueError, TimeSecondLeft},
    factory::SyncTaskQueueBuilder,
    tm_traits::TaskRequestMPMCReceiver,
};

use std::{collections::VecDeque, ops::RangeBounds};

use derivative::Derivative;
use getset::{Getters, MutGetters, Setters};
use log::{error, info};

pub type QueueId = Uuid;

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

    pub fn builder() -> SyncTaskQueueBuilder<T, TR> {
        return SyncTaskQueueBuilder::new();
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
    pub async fn wait_and_fetch_task(&mut self) -> Result<SyncTask, QueueError> {
        let task_fetch_request_recv_result = self.task_request_receiver.receive().await;
        match task_fetch_request_recv_result {
            None => {
                error!(
                    "Expected to receive a task request but received none in queue {}!",
                    *self.sync_plan_id()
                );
                Err(QueueError::EmptyRequestReceived(String::from(
                    "Expected to receive a task request but received none!",
                )))
            }
            Some(task_fetch_request) => {
                if *task_fetch_request.sync_plan_id() != self.sync_plan_id {
                    return Err(QueueError::UnmatchedSyncPlanId);
                } else {
                    let fetch_result = self.pop_front().await;
                    return fetch_result;
                }
            }
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
