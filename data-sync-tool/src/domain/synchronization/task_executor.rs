use async_trait::async_trait;
use tokio::sync::Mutex;
/// Task Executor Trait
/// Defines the common interface for task execution
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

use super::sync_plan::SyncPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskExecutorError {}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanProgress {
    plan_id: Uuid,
    name: String,
    total_tasks: usize,
    completed_task: usize,
    completion_rate: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyncProgress {
    plan_progress: HashMap<Uuid, PlanProgress>,
}

#[async_trait]
pub trait TaskExecutor: Sync + Send {
    // add new sync plans to synchronize
    async fn assign(&mut self, sync_plans: Vec<Arc<Mutex<SyncPlan>>>) -> Result<(), TaskExecutorError>;

    // run a single plan. Either start a new plan or continue a paused plan
    async fn run(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError>;

    // run all assigned plans
    async fn run_all(&mut self) -> Result<(), TaskExecutorError>;

    // temporarily pause a plan
    async fn pause(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError>;

    // pause all plans
    async fn pause_all(&mut self) -> Result<(), TaskExecutorError>;

    // cancel sync for plan, also removes it from the executor
    async fn cancel(&mut self, sync_plan_id: Uuid) -> Result<(), TaskExecutorError>;

    // cancel and drop all plans
    async fn cancel_all(&mut self) -> Result<(), TaskExecutorError>;

    // report current progress
    async fn report_progress(&self) -> Result<SyncProgress, TaskExecutorError>;
}
