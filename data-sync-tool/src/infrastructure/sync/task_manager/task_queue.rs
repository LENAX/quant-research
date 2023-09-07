use std::{collections::VecDeque, ops::RangeBounds, sync::Arc};

use async_trait::async_trait;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::synchronization::{rate_limiter::RateLimiter, sync_task::SyncTask};

use super::errors::QueueError;

/**
 * Synchronization Task Queue Interface
 */

#[async_trait]
pub trait TaskQueue {
    type BuilderType;

    fn start_sending_tasks(&mut self);
    fn pause(&mut self);
    fn resume(&mut self);
    fn finished(&mut self);
    fn stop(&mut self) -> Vec<Arc<Mutex<SyncTask>>>;
    fn get_plan_id(&self) -> Uuid;
    fn initial_size(&self) -> usize;
    fn retries_left(&self) -> Option<u32>;
    fn set_retries_left(&mut self, n_retry: u32);
    async fn wait_and_fetch_task(&mut self) -> Result<Arc<Mutex<SyncTask>>, QueueError>;
    async fn try_fetch_task(
        tasks: &mut VecDeque<Arc<Mutex<SyncTask>>>,
        rate_limiter: &mut impl RateLimiter,
    ) -> Result<Arc<Mutex<SyncTask>>, QueueError>;
    async fn pop_front(&mut self) -> Result<Arc<Mutex<SyncTask>>, QueueError>;
    fn drain<R: RangeBounds<usize>>(&mut self, range: R) -> Vec<Arc<Mutex<SyncTask>>>;
    fn drain_all(&mut self) -> Vec<Arc<Mutex<SyncTask>>>;
    fn push_back(&mut self, task: Arc<Mutex<SyncTask>>);
    fn push_front(&mut self, task: Arc<Mutex<SyncTask>>);
    fn front(&self) -> Option<&Arc<Mutex<SyncTask>>>;
    fn is_empty(&self) -> bool;
    fn is_finished(&self) -> bool;
    fn is_running(&self) -> bool;
    fn is_paused(&self) -> bool;
    fn is_stopped(&self) -> bool;
    fn len(&self) -> usize;
    fn can_retry(&self) -> bool;
    fn retry(&mut self, task: Arc<Mutex<SyncTask>>);
}
