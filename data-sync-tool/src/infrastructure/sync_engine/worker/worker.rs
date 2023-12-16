use getset::Getters;
use log::info;
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
        info!("Worker {} started", self.id);
        self.state = WorkerState::Idle;

        loop {
            tokio::select! {
                Some(command) = self.cmd_rx.recv() => {
                    match command {
                        WorkerCommand::Shutdown => {
                            info!("Worker {} shutting down", self.id);
                            self.state = WorkerState::Stopped;
                            let _ = self.resp_tx.send(WorkerResponse::ShutdownComplete(self.id)).await;
                            break;
                        },
                        WorkerCommand::AssignPlan { plan_id, task_receiver, start_immediately } => {
                            info!("Worker {} assigned to plan {}", self.id, plan_id);
                            self.assigned_plan_id = Some(plan_id);
                            self.state = WorkerState::Working { plan_id, task_id: Uuid::new_v4() }; // Assuming task_id is generated here
                            
                            if start_immediately {
                                // Implement logic to start processing the plan
                            }

                            let _ = self.resp_tx.send(WorkerResponse::PlanAssigned {
                                worker_id: self.id, 
                                plan_id, 
                                sync_started: start_immediately
                            }).await;
                        },
                        WorkerCommand::StartSync => {
                            // Implement logic to start syncing if a plan is assigned
                            if let Some(plan_id) = self.assigned_plan_id {
                                // Start syncing logic here
                                let _ = self.resp_tx.send(WorkerResponse::StartOk { worker_id: self.id, plan_id }).await;
                            } else {
                                let _ = self.resp_tx.send(WorkerResponse::StartFailed { worker_id: self.id, reason: "No plan assigned".to_string() }).await;
                            }
                        },
                        WorkerCommand::CancelPlan(plan_id) => {
                            // Implement logic to cancel the plan
                            if self.assigned_plan_id == Some(plan_id) {
                                // Cancel the plan logic here
                                self.assigned_plan_id = None;
                                self.state = WorkerState::Idle;
                                let _ = self.resp_tx.send(WorkerResponse::PlanCancelled { worker_id: self.id, plan_id }).await;
                            }
                        },
                        WorkerCommand::CheckStatus => {
                            // Implement logic to check and report the status
                        },
                        // Other commands as needed
                    }
                },
                task_response = self.task_rx.recv() => {
                    // Implement logic to handle task manager response
                    match task_response {
                        Ok(response) => {
                            match response {
                                TaskManagerResponse::ShutdownComplete => todo!(),
                                TaskManagerResponse::PlanAdded { plan_id } => todo!(),
                                TaskManagerResponse::PlanRemoved { plan_id } => todo!(),
                                TaskManagerResponse::Error { message } => todo!(),
                                TaskManagerResponse::TaskChannel { plan_id, task_sender } => todo!(),
                            }
                        },
                        Err(_) => todo!(),
                    }
                    
                }
            }
        }
    }
}