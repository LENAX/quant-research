//! Engine Related Commands
//! 

use uuid::Uuid;

use crate::infrastructure::sync_engine::message::ControlMessage;

type PlanId = Uuid;



#[derive(Debug)]
pub enum ProgressManagement {
    ImmediateReport,
    ListenForProgress
}

#[derive(Debug)]
pub enum EngineCommands {
    EngineStateControl(ControlMessage),
}