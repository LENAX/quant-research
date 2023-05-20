/// Task Executor Trait
/// Defines the common interface for task execution 

use serde_json::Value;
use std::error::Error;
use async_trait::async_trait;

#[async_trait]
pub trait TaskExecutor {
    async fn execute(&mut self) -> Result<Value, Box<dyn Error>>;
    async fn cancel_task(&mut self) -> Result<Value, Box<dyn Error>>;
}