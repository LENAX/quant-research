/// Task Executor Trait
/// Defines the common interface for task execution 
use std::error::Error;
use async_trait::async_trait;

use super::sync_task::SyncTask;

#[async_trait]
pub trait TaskExecutor {
    async fn assign(&mut self, tasks: &[SyncTask]);
    async fn run(&mut self) -> Result<(), Box<dyn Error>>;
    async fn cancel(&mut self) -> Result<(), Box<dyn Error>>;
}