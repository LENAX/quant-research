use getset::{Getters, MutGetters};
use tokio::sync::{mpsc, broadcast};
use uuid::Uuid;

use super::{engine::commands::{EngineResponse, EngineCommands, Plan}, worker::commands::WorkerResult};

#[derive(Debug, Getters, MutGetters)]
#[getset(get = "pub", get_mut = "pub")]
pub struct EngineProxy {
    engine_command_tx: mpsc::Sender<EngineCommands>,
    engine_resp_rx: broadcast::Receiver<EngineResponse>,
    worker_result_rx: mpsc::Receiver<WorkerResult>,
}

impl EngineProxy {
    pub fn new(
        engine_cmd_sender: mpsc::Sender<EngineCommands>,
        engine_resp_receiver: broadcast::Receiver<EngineResponse>,
        worker_result_rx: mpsc::Receiver<WorkerResult>,
    ) -> Self {
        EngineProxy {
            engine_command_tx: engine_cmd_sender,
            engine_resp_rx: engine_resp_receiver,
            worker_result_rx,
        }
    }

    pub async fn shutdown(&mut self) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::Shutdown)
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::ShutdownComplete)
            .await
    }

    pub async fn add_plan(&mut self, plan: Plan, start_immediately: bool) -> Result<Uuid, String> {
        let plan_id = plan.plan_id;
        self.engine_command_tx
            .send(EngineCommands::AddPlan {
                plan,
                start_immediately,
            })
            .await
            .map_err(|e| e.to_string())?;
        
        match self
            .wait_for_response(EngineResponse::PlanAdded { plan_id: plan_id })
            .await
        {
            Ok(()) => Ok(plan_id),
            Err(_) => Err("Unexpected response while adding plan".to_string()),
        }
    }

    pub async fn remove_plan(&mut self, plan_id: Uuid) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::RemovePlan(plan_id))
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::PlanRemoved { plan_id })
            .await
    }

    pub async fn start_sync(&mut self) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::StartSync)
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::SyncStarted).await
    }

    pub async fn cancel_sync(&mut self) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::CancelSync)
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::SyncCancelled).await
    }

    pub async fn start_plan(&mut self, plan_id: Uuid) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::StartPlan(plan_id))
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::PlanStarted { plan_id })
            .await
    }

    pub async fn cancel_plan(&mut self, plan_id: Uuid) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::CancelPlan(plan_id))
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::PlanCancelled { plan_id })
            .await
    }

    async fn wait_for_response(&mut self, expected_response: EngineResponse) -> Result<(), String> {
        while let Ok(response) = self.engine_resp_rx.recv().await {
            if response == expected_response {
                return Ok(());
            }
        }
        Err("Failed to receive expected response from engine".to_string())
    }
}
