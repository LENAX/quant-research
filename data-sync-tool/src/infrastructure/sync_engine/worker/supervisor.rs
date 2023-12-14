//! Supervisor Implementation
//! Serve the role of managing and coordinating multiple workers
//!

use std::{collections::{HashMap, HashSet}, sync::Arc};

use getset::Getters;
use log::info;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

use crate::{infrastructure::sync_engine::{
    task_manager::commands::{TaskManagerResponse, TaskManagerCommand}, ComponentState,
}, application::synchronization::dtos::task_manager};

use super::{
    commands::{
        SupervisorCommand, SupervisorResponse, WorkerCommand, WorkerResponse, WorkerResult,
    },
    worker::Worker,
};

type WorkerId = Uuid;
type PlanId = Uuid;

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct Supervisor {
    cmd_rx: mpsc::Receiver<SupervisorCommand>,
    resp_tx: mpsc::Sender<SupervisorResponse>,
    task_manager_cmd_tx: mpsc::Sender<TaskManagerCommand>,
    task_manager_resp_rx: broadcast::Receiver<TaskManagerResponse>,
    worker_cmd_tx: HashMap<WorkerId, mpsc::Sender<WorkerCommand>>,
    worker_resp_rx: mpsc::Receiver<WorkerResponse>,
    plans_to_sync: HashSet<Uuid>,
    worker_assignment: HashMap<WorkerId, Option<PlanId>>,
    state: ComponentState,
}

impl Supervisor {
    pub fn new(
        n_workers: usize,
        task_manager_cmd_tx: mpsc::Sender<TaskManagerCommand>,
        task_manager_resp_rx: broadcast::Receiver<TaskManagerResponse>,
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
        let mut worker_assignment = HashMap::new();

        for _ in 0..n_workers {
            let (tx, rx) = mpsc::channel(32);
            let worker_id = WorkerId::new_v4(); // Generate or assign a unique WorkerId
            let worker_resp_tx_clone = worker_resp_tx.clone();
            let result_tx_clone = result_tx.clone();
            let task_rx_clone = task_rx.resubscribe();
            worker_assignment.insert(worker_id, None);

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
                task_manager_cmd_tx,
                task_manager_resp_rx,
                worker_cmd_tx,
                worker_resp_rx,
                plans_to_sync: HashSet::new(),
                worker_assignment,
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
                    self.plans_to_sync.insert(plan_id);
                    let _ = self
                        .resp_tx
                        .send(SupervisorResponse::PlanAssigned { plan_id })
                        .await;
                }
                SupervisorCommand::CancelPlan(plan_id) => {
                    // Cancel the plan...
                    self.plans_to_sync.remove(&plan_id);
                    let _ = self
                        .resp_tx
                        .send(SupervisorResponse::PlanCancelled { plan_id })
                        .await;
                }
                SupervisorCommand::StartAll => {
                    // Assume the engine starts from the fresh stage
                    // Are there any other state should be considered?

                    // StartAll command prepares the engine into syncing state
                    // Assume the engine start from the fresh stage
                    // then:
                    // 1. engine iterate all the plans need sync
                    // 2. request task manager for task receivers
                    // 3. find available workers and assign plans to them along with the corresponding task receivers

                    let mut tasks = Vec::new();
                    let worker_cmd_tx_ref = Arc::new(self.worker_cmd_tx.clone());

                    // TODO: Assign plan to worker

                    for &plan_id in &self.plans_to_sync {
                        let task_manager_cmd_tx = self.task_manager_cmd_tx.clone();
                        let worker_cmd_tx_ref_clone = Arc::clone(&worker_cmd_tx_ref);

                        // Clone the receiver for each task
                        let mut task_manager_resp_rx = self.task_manager_resp_rx.resubscribe();

                        // Spawn an async block for each plan
                        let task = tokio::spawn(async move {
                            // Request task_receiver for this plan from TaskManager
                            let _ = task_manager_cmd_tx.send(TaskManagerCommand::RequestTaskReceiver { plan_id }).await;

                            // Process response from TaskManager
                            if let Ok(response) = task_manager_resp_rx.recv().await {
                                if let TaskManagerResponse::TaskChannel { plan_id: received_plan_id, task_sender } = response {
                                    if received_plan_id == plan_id {
                                        let task_receiver = task_sender.subscribe();

                                        // Find an available worker to assign this plan
                                        if let Some((worker_id, worker_tx)) = worker_cmd_tx_ref_clone.iter().find(|&(_id, _tx)| {
                                            // Implement logic to select a suitable worker
                                            // TODO: Send message to ask for availability, or just use worker_assignment to find a free worker.
                                            true // Placeholder, replace with actual logic
                                        }) {
                                            // record worker assignment
                                            // self.worker_assignment.insert(*worker_id, Some(plan_id));
                                            // Assign the plan and the task_receiver to the selected worker
                                            let _ = worker_tx.send(WorkerCommand::AssignPlan { plan_id, task_receiver, start_immediately: true }).await;
                                        }
                                    }
                                }
                            }
                        });

                        tasks.push(task);
                    }

                    // Wait for all tasks to finish
                    let _ = futures::future::join_all(tasks).await;
                
                    // Notify that all plans have been instructed to start
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
