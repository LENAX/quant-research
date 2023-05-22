/// Task Executor Trait
/// Defines the common interface for task execution 
use std::error::Error;
use async_trait::async_trait;

use super::{sync_task::SyncTask, value_objects::execution_result::ExecutionResult};

#[async_trait]
pub trait TaskExecutor {
    fn assign(&mut self, tasks: &[SyncTask]) -> Result<Box<dyn TaskExecutor>, Box<dyn Error>>;
    async fn execute_all<'a>(&mut self) -> Result<ExecutionResult<'a>, Box<dyn Error>>;
    async fn execute<'a>(self, task: &mut SyncTask) -> Result<ExecutionResult<'a>, Box<dyn Error>>;
    async fn cancel<'a>(self, task: &mut SyncTask) -> Result<&'a mut SyncTask<'a>, Box<dyn Error>>;
}