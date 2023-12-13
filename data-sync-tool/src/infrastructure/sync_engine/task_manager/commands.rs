//! TaskManager Related Commands
//! 

use uuid::Uuid;

use crate::{infrastructure::sync_engine::{message::ControlMessage, engine::commands::Plan}, domain::synchronization::{sync_plan::SyncPlan, value_objects::task_spec::TaskSpecification}};

type PlanId = Uuid;

#[derive(Debug)]
pub enum TaskManagerCommand {
    // Lifecycle Control
    Shutdown,
    
    // Task Control
    AddPlan(Plan),
    RemovePlan(PlanId),
    RequestTask(PlanId),

}

#[derive(Debug, Clone)]
pub enum TaskManagerResponse {
    // Lifecycle Control Responses
    ShutdownComplete,

    // Task Control Responses
    PlanAdded { plan_id: Uuid },
    PlanRemoved { plan_id: Uuid },
    Error { message: String }, // General error response
    // Additional responses as needed...
}

// TODO: Separate Task Sending from other command response