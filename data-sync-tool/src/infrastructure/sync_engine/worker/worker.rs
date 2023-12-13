use getset::Getters;
use tokio::sync::{mpsc, broadcast};
use uuid::Uuid;

use crate::infrastructure::sync_engine::task_manager::commands::TaskManagerResponse;

use super::commands::{WorkerCommand, WorkerResponse, WorkerResult};


#[derive(Debug)]
pub enum WorkerState {
    Created,
    Idle,
    Working { plan_id: Uuid, task_id: Uuid },
    Stopped
}

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct Worker {
    id: Uuid,
    cmd_rx: mpsc::Receiver<WorkerCommand>,
    task_rx: broadcast::Receiver<TaskManagerResponse>,
    resp_tx: mpsc::Sender<WorkerResponse>,
    result_tx: mpsc::Sender<WorkerResult>,
    assigned_plan_id: Option<Uuid>,
    state: WorkerState
}

impl Worker {
    pub fn new(
        id: Uuid,
        cmd_rx: mpsc::Receiver<WorkerCommand>,
        task_rx: broadcast::Receiver<TaskManagerResponse>,
        resp_tx: mpsc::Sender<WorkerResponse>,
        result_tx: mpsc::Sender<WorkerResult>
    ) -> Self {

        Self {
            id,
            cmd_rx,
            task_rx,
            resp_tx,
            result_tx,
            assigned_plan_id: None,
            state: WorkerState::Created
        }
    }

    pub async fn run(mut self) {

    }
}