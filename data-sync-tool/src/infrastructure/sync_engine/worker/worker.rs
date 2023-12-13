use std::time::Duration;

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{infrastructure::sync_engine::task_manager::commands::TaskManagerCommand, application::synchronization::dtos::task_manager};



pub struct WorkerConfig {
    concurrency_limit: usize,
    max_retries: usize,
    task_timeout: Duration,
    memory_limit: Option<usize>,
    cpu_usage_limit: Option<f32>,
    log_level: LogLevel,
    api_endpoint: String,
    auth_token: Option<String>,
    region_affinity: Option<String>,
    // ... other fields ...
}

enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

enum WorkerState {
    Idle,
    Busy,
    Error
}

pub struct Worker {
    id: Uuid,
    assigned_plan_id: Option<Uuid>,
    task_manager_tx: mpsc::Sender<TaskManagerCommand>,
    state: WorkerState
    // ... other state ...
}

impl Worker {
    pub fn new(task_manager_tx: mpsc::Sender<TaskManagerCommand>) -> Worker {
        Worker { id: Uuid::new_v4(), assigned_plan_id: None, task_manager_tx: task_manager_tx }
    }

    pub async fn run(mut self) {
        // Request a task from the Supervisor
        
        let _ = self.task_manager_tx.send(TaskManagerCommand::GetNextTask(self.assigned_plan_id)).await;
        // Wait for the task and execute it
        // ...
    }
}

enum SupervisorCommand {
    RequestTask(Uuid), // Uuid to identify the worker
    // ... other commands ...
}
