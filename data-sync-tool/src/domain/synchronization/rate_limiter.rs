/// Task Executor Trait
/// Defines the common interface for task execution 
use std::error::Error;

use uuid::Uuid;
use super::{value_objects::sync_config::RateQuota, sync_task::SyncTask};

pub trait RateLimiter: Sync + Send {
    fn apply_limit(&mut self, quota: &RateQuota, sync_plan_id: Uuid) -> bool;
    fn can_proceed(&mut self, task: &SyncTask) -> Result<bool, Box<dyn Error>>;
}