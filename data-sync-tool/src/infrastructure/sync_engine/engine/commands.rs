//! Engine Related Commands
//! 

use getset::Getters;
use uuid::Uuid;

use crate::domain::synchronization::value_objects::task_spec::TaskSpecification;

type PlanId = Uuid;

#[derive(Debug, Getters, Clone)]
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
    AddPlan { plan: Plan , start_immediately: bool },
    RemovePlan(Uuid),
    StartSync,
    CancelSync,
    StartPlan(Uuid),
    CancelPlan(Uuid),
    PausePlan(Uuid)
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineResponse {
    ComponentShutdownComplete(String),
    ShutdownComplete,
    PlanAdded { plan_id: Uuid },
    PlanRemoved { plan_id: Uuid },
    SyncStarted,
    SyncCancelled,
    PlanStarted { plan_id: Uuid },
    PlanCancelled { plan_id: Uuid },
    Error { message: String, component: String }, // General error response
    // Additional responses as needed...
}