//! Synchronization Engine Implementation
//!
//!
//!

use getset::Getters;
use log::{error, info};
use tokio::{
    select,
    sync::{broadcast, mpsc},
    time::{timeout, Duration},
};

use crate::infrastructure::sync_engine::{
    task_manager::commands::{TaskManagerCommand, TaskManagerResponse},
    worker::commands::{SupervisorCommand, SupervisorResponse},
    ComponentState,
};

use super::commands::{EngineCommands, EngineResponse};

#[derive(Debug)]
pub enum EngineState {
    Created,
    Running,
    Stopped,
}

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct SyncEngine {
    task_manager_tx: mpsc::Sender<TaskManagerCommand>,
    task_manager_resp_rx: broadcast::Receiver<TaskManagerResponse>,
    supervisor_tx: mpsc::Sender<SupervisorCommand>,
    supervisor_resp_rx: mpsc::Receiver<SupervisorResponse>,
    engine_rx: mpsc::Receiver<EngineCommands>,
    engine_tx: mpsc::Sender<EngineCommands>,
    engine_resp_tx: broadcast::Sender<EngineResponse>,
    state: ComponentState,
    response_timeout: Duration,
}

impl SyncEngine {
    pub fn new(
        task_manager_tx: mpsc::Sender<TaskManagerCommand>,
        task_manager_resp_rx: broadcast::Receiver<TaskManagerResponse>,
        supervisor_tx: mpsc::Sender<SupervisorCommand>,
        supervisor_resp_rx: mpsc::Receiver<SupervisorResponse>,
        response_timeout: Option<Duration>,
    ) -> (
        Self,
        mpsc::Sender<EngineCommands>,
        broadcast::Receiver<EngineResponse>,
    ) {
        let (engine_tx, engine_rx) = mpsc::channel::<EngineCommands>(100);
        let (engine_resp_tx, engine_resp_rx) = broadcast::channel::<EngineResponse>(100);
        let engine = Self {
            task_manager_tx,
            task_manager_resp_rx,
            supervisor_tx,
            supervisor_resp_rx,
            engine_rx,
            engine_tx: engine_tx.clone(),
            engine_resp_tx,
            state: ComponentState::Created,
            response_timeout: response_timeout.unwrap_or(Duration::from_secs(5)),
        };

        (engine, engine_tx, engine_resp_rx)
    }

    pub async fn run(mut self) {
        info!("Initializing Sync Engine...");
        self.state = ComponentState::Running;

        loop {
            select! {
                // The engine should stay responsive to external commands
                // Then wait for its submodules' response
                biased;

                Some(command) = self.engine_rx.recv() => {
                    self.handle_command(command).await;

                    if self.state == ComponentState::Stopped {
                        break;
                    }
                    // break;
                },
                tm_response = self.task_manager_resp_rx.recv() => {
                    match tm_response {
                        Ok(response) => { self.handle_task_manager_response(response).await; },
                        Err(e) => { error!("{}", e)}
                    }
                },
                sp_response = self.supervisor_resp_rx.recv() => {
                    match sp_response {
                        Some(response) => { self.handle_supervisor_response(response).await; },
                        None => { error!("Received no response from supervisor!")}
                    }
                },
                else => break,
            }
        }
        info!("Engine is down.");
    }

    async fn handle_command(&mut self, command: EngineCommands) {
        match command {
            EngineCommands::Shutdown => {
                info!("Shutdown command received!");
                self.handle_shutdown().await;
            }
            EngineCommands::AddPlan {
                plan,
                start_immediately,
            } => {
                info!("Add a new plan {}", plan.plan_id());
                let plan_id = plan.plan_id().clone();
                let _ = self
                    .task_manager_tx
                    .send(TaskManagerCommand::AddPlan(plan))
                    .await;
                let _ = self
                    .supervisor_tx
                    .send(SupervisorCommand::AssignPlan {
                        plan_id,
                        start_immediately,
                    })
                    .await;
            }
            EngineCommands::RemovePlan(plan_id) => {
                info!("Remove a plan {}", plan_id);
                let _ = self
                    .supervisor_tx
                    .send(SupervisorCommand::CancelPlan(plan_id))
                    .await;
                let _ = self
                    .task_manager_tx
                    .send(TaskManagerCommand::RemovePlan(plan_id))
                    .await;
            }
            EngineCommands::StartSync => {
                info!("Start syncing...");
                let _ = self.supervisor_tx.send(SupervisorCommand::StartAll).await;
            }
            EngineCommands::CancelSync => {
                info!("Stop syncing...");
                let _ = self.supervisor_tx.send(SupervisorCommand::CancelAll).await;
            }
            EngineCommands::StartPlan(plan_id) => {
                info!("Start syncing plan {}", plan_id);
                let _ = self.supervisor_tx.send(SupervisorCommand::StartSyncPlan(plan_id)).await;
            },
            EngineCommands::CancelPlan(plan_id) => {
                info!("Cancel syncing plan {}", plan_id);
                let _ = self.supervisor_tx.send(SupervisorCommand::CancelSyncPlan(plan_id)).await;
            },
        }
    }

    async fn handle_task_manager_response(&mut self, response: TaskManagerResponse) {
        match response {
            TaskManagerResponse::PlanAdded { plan_id } => {
                // Handle plan added response...
                info!("Added new plan {}", plan_id);
                if let Err(e) = self
                    .engine_resp_tx
                    .send(EngineResponse::PlanAdded { plan_id })
                {
                    error!("Failed to send response: {}", e);
                }
            }
            TaskManagerResponse::PlanRemoved { plan_id } => {
                // Handle plan removed response...
                info!("Removed plan {}", plan_id);
                if let Err(e) = self
                    .engine_resp_tx
                    .send(EngineResponse::PlanRemoved { plan_id })
                {
                    error!("Failed to send response: {}", e);
                }
            }
            TaskManagerResponse::ShutdownComplete => {
                info!("Task Manager terminated.");
                if let Err(e) = self
                    .engine_resp_tx
                    .send(EngineResponse::ComponentShutdownComplete(
                        "TaskManager".to_string(),
                    ))
                {
                    error!("Failed to send response: {}", e);
                }
            }
            TaskManagerResponse::Error { message } => {
                info!("Task Manager error: {}", message);
                if let Err(e) = self.engine_resp_tx.send(EngineResponse::Error {
                    message,
                    component: "TaskManager".to_string(),
                }) {
                    error!("Failed to send response: {}", e);
                }
            }
            _ => {} // ... other responses ...
        }
    }

    async fn handle_supervisor_response(&mut self, response: SupervisorResponse) {
        match response {
            SupervisorResponse::PlanAssigned { plan_id } => {
                // Handle plan assigned response...
                info!("SupervisorResponse: Assigned new plan {}", plan_id);
            }
            SupervisorResponse::PlanCancelled {
                plan_id,
                worker_id: _,
            } => {
                // Handle plan cancelled response...
                info!("SupervisorResponse: Cancelled plan {}", plan_id);
            }
            SupervisorResponse::ShutdownComplete => {
                info!("Supervisor terminated.");
                if let Err(e) = self
                    .engine_resp_tx
                    .send(EngineResponse::ComponentShutdownComplete(
                        "Supervisor".to_string(),
                    ))
                {
                    error!("Failed to send response: {}", e);
                }
            }
            SupervisorResponse::AllStarted => {
                info!("SupervisorResponse: All workers are busy!");
                if let Err(e) = self.engine_resp_tx.send(EngineResponse::SyncStarted) {
                    error!("Failed to send response: {}", e);
                }
            }
            SupervisorResponse::AllCancelled => {
                info!("SupervisorResponse: All plans are cancelled!");
                if let Err(e) = self.engine_resp_tx.send(EngineResponse::SyncCancelled) {
                    error!("Failed to send response: {}", e);
                }
            }
            SupervisorResponse::Error { message } => {
                info!("Supervisor error: {}", message);
                if let Err(e) = self.engine_resp_tx.send(EngineResponse::Error {
                    message,
                    component: "Supervisor".to_string(),
                }) {
                    error!("Failed to send response: {}", e);
                }
            }
            // ... other responses ...
        }
    }

    async fn handle_shutdown(&mut self) {
        let _ = self.supervisor_tx.send(SupervisorCommand::Shutdown).await;
        let _ = self
            .task_manager_tx
            .send(TaskManagerCommand::Shutdown)
            .await;
        self.state = ComponentState::Stopped;
        if let Err(e) = self.engine_resp_tx.send(EngineResponse::ShutdownComplete) {
            error!("Failed to send response: {}", e);
        }
    }
}
