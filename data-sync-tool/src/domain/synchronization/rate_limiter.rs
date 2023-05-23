/// Task Executor Trait
/// Defines the common interface for task execution 
use std::error::Error;

use super::{sync_task::SyncTask, value_objects::execution_result::ExecutionResult};

pub trait RateLimiter {
    fn limit(&mut self, tasks: &[SyncTask]) -> Result<(), Box<dyn Error>>;
}