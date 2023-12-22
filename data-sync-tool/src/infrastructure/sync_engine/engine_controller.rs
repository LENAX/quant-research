use async_trait::async_trait;
use getset::{Getters, MutGetters};
use log::info;
use tokio::sync::{mpsc, broadcast};
use uuid::Uuid;

use super::{engine::commands::{EngineResponse, EngineCommands, Plan}, worker::commands::WorkerResult};



#[async_trait]
pub trait SyncEngineControl: Send + Sync {

    async fn fetch_next_worker_result(&mut self) -> Result<WorkerResult, String>;

    async fn process_worker_results<F>(&mut self, callback: F) -> Result<(), String>
    where
        F: FnMut(WorkerResult) -> () + Send;

    fn subscribe_to_worker_results(&self) -> broadcast::Receiver<WorkerResult>;

    async fn shutdown(&mut self) -> Result<(), String>;

    async fn add_plan(&mut self, plan: Plan, start_immediately: bool) -> Result<Uuid, String>;

    async fn remove_plan(&mut self, plan_id: Uuid) -> Result<(), String>;

    async fn start_sync(&mut self) -> Result<(), String>;

    async fn cancel_sync(&mut self) -> Result<(), String>;

    async fn pause_sync(&mut self) -> Result<(), String>;

    async fn start_plan(&mut self, plan_id: Uuid) -> Result<(), String>;

    async fn cancel_plan(&mut self, plan_id: Uuid) -> Result<(), String>;

    async fn pause_sync_plan(&mut self, plan_id: Uuid) -> Result<(), String>;
    // Add other method signatures...
}

#[derive(Debug, Getters, MutGetters)]
#[getset(get = "pub", get_mut = "pub")]
pub(crate) struct EngineController {
    engine_command_tx: mpsc::Sender<EngineCommands>,
    engine_resp_rx: broadcast::Receiver<EngineResponse>,
    worker_result_rx: broadcast::Receiver<WorkerResult>,
}

impl EngineController {
    pub fn new(
        engine_cmd_sender: mpsc::Sender<EngineCommands>,
        engine_resp_receiver: broadcast::Receiver<EngineResponse>,
        worker_result_rx: broadcast::Receiver<WorkerResult>,
    ) -> Self {
        EngineController {
            engine_command_tx: engine_cmd_sender,
            engine_resp_rx: engine_resp_receiver,
            worker_result_rx,
        }
    }

    async fn wait_for_response(&mut self, expected_response: EngineResponse) -> Result<(), String> {
        info!("Waiting for engine's response...");
        while let Ok(response) = self.engine_resp_rx.recv().await {
            info!("Engine response: {:?}", response);
            if response == expected_response {
                return Ok(());
            }
        }
        Err("Failed to receive expected response from engine".to_string())
    }
}

#[async_trait]
impl SyncEngineControl for EngineController {
    async fn fetch_next_worker_result(&mut self) -> Result<WorkerResult, String> {
        self.worker_result_rx.recv().await
            .map_err(|e| format!("Failed to receive worker result: {}", e))
    }

    /// Continuously processes worker results.
    /// This method will keep running and process each worker result as it arrives.
    /// You can pass a callback function to process each result.
    async fn process_worker_results<F>(&mut self, mut callback: F) -> Result<(), String>
    where
        F: FnMut(WorkerResult) -> () + Send,
    {
        while let Ok(result) = self.worker_result_rx.recv().await {
            callback(result);
        }
        Err("Worker result channel closed".to_string())
    }

    fn subscribe_to_worker_results(&self) -> broadcast::Receiver<WorkerResult> {
        self.worker_result_rx.resubscribe()
    }

    async fn shutdown(&mut self) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::Shutdown)
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::ShutdownComplete)
            .await
    }

    async fn add_plan(&mut self, plan: Plan, start_immediately: bool) -> Result<Uuid, String> {
        let plan_id = plan.plan_id;
        info!("Sending command add_plan to engine...");
        self.engine_command_tx
            .send(EngineCommands::AddPlan {
                plan,
                start_immediately,
            })
            .await
            .map_err(|e| e.to_string())?;
        
        info!("Waiting for response...");
        match self
            .wait_for_response(EngineResponse::PlanAdded { plan_id })
            .await
        {
            Ok(()) => Ok(plan_id),
            Err(_) => Err("Unexpected response while adding plan".to_string()),
        }
    }

    async fn remove_plan(&mut self, plan_id: Uuid) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::RemovePlan(plan_id))
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::PlanRemoved { plan_id })
            .await
    }

    async fn start_sync(&mut self) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::StartSync)
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::SyncStarted).await
    }

    async fn cancel_sync(&mut self) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::CancelSync)
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::SyncCancelled).await
    }

    async fn pause_sync(&mut self) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::CancelSync)
            .await
            .map_err(|e| e.to_string())
    }

    async fn start_plan(&mut self, plan_id: Uuid) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::StartPlan(plan_id))
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::PlanStarted { plan_id })
            .await
    }

    async fn cancel_plan(&mut self, plan_id: Uuid) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::CancelPlan(plan_id))
            .await
            .map_err(|e| e.to_string())?;
        self.wait_for_response(EngineResponse::PlanCancelled { plan_id })
            .await
    }

    async fn pause_sync_plan(&mut self, plan_id: Uuid) -> Result<(), String> {
        self.engine_command_tx
            .send(EngineCommands::PausePlan(plan_id))
            .await
            .map_err(|e| e.to_string())
    }

    
}
