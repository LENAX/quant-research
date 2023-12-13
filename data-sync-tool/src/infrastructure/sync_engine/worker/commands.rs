//! Worker and Supervisor Commands
//! 

use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::synchronization::sync_task::SyncTask;

use super::worker::WorkerConfig;


pub enum SupervisorCommand {
    LaunchWorkers(usize),
    TerminateWorkers(Vec<Uuid>), // Assuming workers are identified by Uuids
    PauseAll,
    ResumeAll,
    AdjustWorkerPool(usize),
    DistributeTasks,
    Shutdown,
}

pub enum WorkerCommands {
    ExecuteTask(SyncTask),
    CancelTask,
    PauseTask,
    ResumeTask,
    UpdateConfiguration(WorkerConfig),
    Shutdown,
}