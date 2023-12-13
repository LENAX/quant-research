//! Supervisor Implementation
//! Serve the role of managing and coordinating multiple workers
//!

use std::collections::HashMap;

use getset::Getters;
use log::info;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::infrastructure::sync_engine::{
    task_manager::commands::TaskManagerResponse, ComponentState,
};

use super::{
    commands::{
        SupervisorCommand, SupervisorResponse, WorkerCommand, WorkerResponse, WorkerResult,
    },
    worker::Worker,
};

type WorkerId = Uuid;

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct Supervisor {
    cmd_rx: mpsc::Receiver<SupervisorCommand>,
    resp_tx: mpsc::Sender<SupervisorResponse>,
    worker_cmd_tx: HashMap<WorkerId, mpsc::Sender<WorkerCommand>>,
    worker_resp_rx: mpsc::Receiver<WorkerResponse>,
    state: ComponentState,
}

impl Supervisor {
    pub fn new(
        n_workers: usize,
        task_rx: broadcast::Receiver<TaskManagerResponse>,
        result_tx: mpsc::Sender<WorkerResult>,
    ) -> (
        Self,
        mpsc::Sender<SupervisorCommand>,
        mpsc::Receiver<SupervisorResponse>,
    ) {
        let (cmd_tx, cmd_rx) = mpsc::channel(32);
        let (resp_tx, resp_rx) = mpsc::channel(32);
        let (worker_resp_tx, worker_resp_rx) = mpsc::channel(32); // Assuming a channel for worker responses

        let mut worker_cmd_tx = HashMap::new();

        for _ in 0..n_workers {
            let (tx, rx) = mpsc::channel(32);
            let worker_id = WorkerId::new_v4(); // Generate or assign a unique WorkerId
            let worker_resp_tx_clone = worker_resp_tx.clone();
            let result_tx_clone = result_tx.clone();
            let task_rx_clone = task_rx.resubscribe();

            // Spawn a new worker - Implement this according to your worker logic
            let _ = tokio::spawn(async move {
                let worker = Worker::new(
                    worker_id, rx, task_rx_clone, worker_resp_tx_clone, result_tx_clone
                );
                info!("Worker {} created!", worker_id);
                worker.run().await;
            });

            worker_cmd_tx.insert(worker_id, tx);
        }

        (
            Supervisor {
                cmd_rx,
                resp_tx,
                worker_cmd_tx,
                worker_resp_rx,
                state: ComponentState::Created,
            },
            cmd_tx,
            resp_rx,
        )
    }

    pub async fn run(mut self) {
        self.state = ComponentState::Running;
        while let Some(command) = self.cmd_rx.recv().await {
            match command {
                SupervisorCommand::Shutdown => {
                    // Perform shutdown logic...
                    self.state = ComponentState::Stopped;
                    let _ = self
                        .resp_tx
                        .send(SupervisorResponse::ShutdownComplete)
                        .await;
                    break;
                }
                SupervisorCommand::AssignPlan(plan_id) => {
                    // Assign the plan to a worker...
                    let _ = self
                        .resp_tx
                        .send(SupervisorResponse::PlanAssigned { plan_id })
                        .await;
                }
                SupervisorCommand::CancelPlan(plan_id) => {
                    // Cancel the plan...
                    let _ = self
                        .resp_tx
                        .send(SupervisorResponse::PlanCancelled { plan_id })
                        .await;
                }
                SupervisorCommand::StartAll => {
                    // Logic to start all plans...
                    let _ = self.resp_tx.send(SupervisorResponse::AllStarted).await;
                }
                SupervisorCommand::CancelAll => {
                    // Logic to cancel all plans...
                    let _ = self.resp_tx.send(SupervisorResponse::AllCancelled).await;
                } // TODO: Implement worker management commands
            }
        }
    }
}
