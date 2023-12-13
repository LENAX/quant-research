//! Engine Related Commands
//! 

use getset::Getters;
use uuid::Uuid;

use crate::domain::synchronization::value_objects::task_spec::TaskSpecification;

type PlanId = Uuid;

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct Plan {
    // simplified sync plan
    pub plan_id: Uuid,
    pub task_specs: Vec<TaskSpecification>
}

#[derive(Debug)]
pub enum ProgressManagement {
    ImmediateReport,
    ListenForProgress
}

#[derive(Debug)]
pub enum EngineCommands {
    Shutdown,
    AddPlan(Plan),
    RemovePlan(Uuid),
    StartSync,
    CancelSync,
    StartPlan(Uuid),
    CancelPlan(Uuid)
}


#[derive(Debug, Clone)]
pub enum EngineResponse {
    ShutdownComplete,
    PlanAdded { plan_id: Uuid },
    PlanRemoved { plan_id: Uuid },
    SyncStarted,
    SyncCancelled,
    PlanStarted { plan_id: Uuid },
    PlanCancelled { plan_id: Uuid },
    Error { message: String }, // General error response
    // Additional responses as needed...
}