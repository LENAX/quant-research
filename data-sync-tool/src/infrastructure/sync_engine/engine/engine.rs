//! Synchronization Engine Implementation
//! 
//! 
//! 

use tokio::sync::mpsc;

use crate::infrastructure::sync_engine::{task_manager::commands::TaskManagerCommand, worker::commands::SupervisorCommands};



pub struct SyncEngine {
    task_manager_tx: mpsc::Sender<TaskManagerCommand>,
    supervisor_tx: mpsc::Sender<SupervisorCommands>
}