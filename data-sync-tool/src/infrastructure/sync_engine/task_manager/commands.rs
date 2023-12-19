//! TaskManager Related Commands
//!

use tokio::sync::broadcast;
use uuid::Uuid;

use crate::infrastructure::sync_engine::engine::commands::Plan;

use super::task_manager::Task;

type PlanId = Uuid;

#[derive(Debug)]
pub enum TaskManagerCommand {
    // Lifecycle Control
    Shutdown,

    // Task Control
    AddPlan(Plan),
    RemovePlan(PlanId),
    RequestTask(PlanId),
    RequestTaskReceiver {
        plan_id: Uuid,
        worker_id: Uuid,
        start_immediately: bool,
    },
}

#[derive(Debug, Clone)]
pub enum TaskManagerResponse {
    // Lifecycle Control Responses
    ShutdownComplete,

    // Task Control Responses
    PlanAdded {
        plan_id: Uuid,
    },
    PlanRemoved {
        plan_id: Uuid,
    },
    Error {
        message: String,
    }, // General error response
    // Additional responses as needed...
    // subscribe to sender to get the receiver
    TaskChannel {
        worker_id: Uuid,
        plan_id: Uuid,
        task_sender: broadcast::Sender<TaskRequestResponse>,
        start_immediately: bool
    },
}

// TODO: Separate Task Sending from other command response
#[derive(Debug, Clone)]
pub enum TaskRequestResponse {
    NewTask(Task),
    NoTaskLeft,
    PlanNotFound(Uuid),
}
