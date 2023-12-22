use chrono::{DateTime, Local};
use serde_json::Value;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::infrastructure::sync_engine::task_manager::commands::TaskRequestResponse;

type PlanId = Uuid;
type WorkerId = Uuid;

#[derive(Debug)]
pub enum SupervisorCommand {
    Shutdown,
    AssignPlan { plan_id: PlanId , start_immediately: bool },
    CancelPlan(Uuid),
    StartSyncPlan(Uuid),
    CancelSyncPlan(Uuid),
    PauseSyncPlan(Uuid),
    StartAll,
    CancelAll,
    // TODO: Worker Management
    // AddWorker(usize),
    // DestroyWorker(usize)
}

#[derive(Debug)]
pub enum SupervisorResponse {
    ShutdownComplete,
    PlanAssigned {
        plan_id: Uuid,
    },
    PlanCancelled {
        worker_id: WorkerId,
        plan_id: Uuid,
    },
    AllStarted,
    AllCancelled,
    Error {
        message: String,
    }, // General error response
    // Additional responses as needed...
    
}

#[derive(Debug)]
pub enum WorkerCommand {
    Shutdown,
    AssignPlan { plan_id: Uuid, task_receiver: broadcast::Receiver<TaskRequestResponse>, start_immediately: bool },
    StartSync,
    PauseSync,
    CancelPlan(Uuid)
}

#[derive(Debug)]
pub enum WorkerStatus {
    Idle,
    Ready,
    Running,
    Stopped
}


#[derive(Debug)]
pub enum WorkerResponse {
    ShutdownComplete(WorkerId),
    PlanAssigned { worker_id: WorkerId, plan_id: PlanId, sync_started: bool },
    PlanCancelled { worker_id: WorkerId, plan_id: PlanId },
    StartOk { worker_id: WorkerId, plan_id: PlanId },
    StartFailed { worker_id: WorkerId, reason: String },
    PauseOk { worker_id: WorkerId, plan_id: PlanId },
    Error(String)
}

// Multiple workers will send result through an mpsc channel 
// There will be a dedicated domain service consuming the result
#[derive(Debug, Clone)]
pub enum WorkerResult {
    TaskCompleted {
        plan_id: Uuid,
        task_id: Uuid,
        result: Value,
        complete_time: DateTime<Local>,
    },
    TaskFailed {
        plan_id: Uuid,
        task_id: Uuid,
        message: String,
        current_datetime: DateTime<Local>,
    }
}
