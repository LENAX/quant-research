//! TaskManager Related Commands
//! 

use std::sync::Arc;

use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{infrastructure::sync_engine::message::ControlMessage, domain::synchronization::sync_plan::SyncPlan};

type PlanId = Uuid;

#[derive(Debug)]
pub enum SyncControl {
    StartAll,
    PauseAll,
    ResumeAll,
    StopAll,
    Start(PlanId),
    Pause(PlanId),
    Resume(PlanId),
    Stop(PlanId),
    AddPlan(Arc<Mutex<SyncPlan>>),
    RemovePlan(PlanId)
}


#[derive(Debug)]
pub enum TaskManagerCommand {
    LifecycleControl(ControlMessage),
    SyncControl(SyncControl),
}