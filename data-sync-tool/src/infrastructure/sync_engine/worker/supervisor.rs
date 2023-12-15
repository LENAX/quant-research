//! Supervisor Implementation
//! Serve the role of managing and coordinating multiple workers
//!

use std::{collections::{HashMap, HashSet}, sync::Arc};

use getset::Getters;
use itertools::Itertools;
use log::{info, error};
use tokio::{sync::{broadcast, mpsc, Mutex}, select};
use uuid::Uuid;

use crate::{infrastructure::sync_engine::{
    task_manager::commands::{TaskManagerResponse, TaskManagerCommand}, ComponentState,
}, application::synchronization::dtos::task_manager};

use super::{
    commands::{
        SupervisorCommand, SupervisorResponse, WorkerCommand, WorkerResponse, WorkerResult,
    },
    worker::{Worker, self},
};

type WorkerId = Uuid;
type PlanId = Uuid;

#[derive(Debug)]
pub enum SupervisorError {
    WorkerFailedToSpawn(String)
}

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct Supervisor {
    cmd_rx: mpsc::Receiver<SupervisorCommand>,
    resp_tx: mpsc::Sender<SupervisorResponse>,
    task_manager_cmd_tx: mpsc::Sender<TaskManagerCommand>,
    task_manager_resp_rx: broadcast::Receiver<TaskManagerResponse>,
    worker_cmd_tx: HashMap<WorkerId, mpsc::Sender<WorkerCommand>>,
    worker_resp_rx: mpsc::Receiver<WorkerResponse>,
    worker_result_tx: mpsc::Sender<WorkerResult>,
    plans_to_sync: HashSet<Uuid>,
    worker_assignment: HashMap<WorkerId, Option<PlanId>>,
    state: ComponentState,
}

impl Supervisor {
    
    fn spawn_worker(
        worker_resp_tx: mpsc::Sender<WorkerResponse>,
        result_tx: mpsc::Sender<WorkerResult>,
        task_rx: broadcast::Receiver<TaskManagerResponse>,
    ) -> (WorkerId, mpsc::Sender<WorkerCommand>) {
        let (tx, rx) = mpsc::channel(32);
        let worker_id = WorkerId::new_v4(); // Generate or assign a unique WorkerId
        let _ = tokio::spawn(async move {
                let worker = Worker::new(
                    worker_id, rx, task_rx, worker_resp_tx, result_tx
                );
                info!("Worker {} created!", worker_id);
                worker.run().await;
            });
        return (worker_id, tx);
    }

    pub fn new(
        n_workers: usize,
        task_manager_cmd_tx: mpsc::Sender<TaskManagerCommand>,
        task_manager_resp_rx: broadcast::Receiver<TaskManagerResponse>,
        task_rx: broadcast::Receiver<TaskManagerResponse>,
        worker_result_tx: mpsc::Sender<WorkerResult>,
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
            let (worker_id, tx) = Supervisor::spawn_worker(
                worker_resp_tx.clone(), worker_result_tx.clone(), task_rx.resubscribe()
            );

            worker_assignment.insert(worker_id, None);
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
                worker_result_tx,
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

        loop {
            select! {
                Some(command) = self.cmd_rx.recv() => {
                    match command {
                        SupervisorCommand::Shutdown => {
                            // Perform shutdown logic...
                            info!("Received shutdown command.");
                            info!("Shutting down Workers...");
                            let mut tasks = Vec::new();
                            for (wid, worker_cmd_tx) in self.worker_cmd_tx.into_iter() {
                                let worker_cmd_tx_clone = worker_cmd_tx.clone();
                                let task = tokio::spawn(async move {
                                    if let Err(e) = worker_cmd_tx_clone.send(WorkerCommand::Shutdown).await {
                                        error!("Failed to shutdown worker {}, Error: {}", wid, e);
                                    }
                                });
                                tasks.push(task);
                            }
                            info!("Waiting for all workers to shutdown...");
                            let _ = futures::future::join_all(tasks).await;

                            info!("Shutting down Supervisor...");
                            self.state = ComponentState::Stopped;
                            let _ = self
                                .resp_tx
                                .send(SupervisorResponse::ShutdownComplete)
                                .await;
                            break;
                        }
                        SupervisorCommand::AssignPlan(plan_id) => {
                            // Register new plan
                            self.plans_to_sync.insert(plan_id);

                            // Find an idle worker
                            let worker_id = {
                                self.worker_assignment.iter_mut().find_map(|(id, pid)| {
                                    if pid.is_none() {
                                        *pid = Some(plan_id);
                                        Some(*id)
                                    } else {
                                        None
                                    }
                                })
                            };

                            // if no worker is available, spawn a new worker
                            match worker_id {
                                Some(wid) => {
                                    todo!()
                                },
                                None => {
                                    let (worker_resp_tx, worker_resp_rx) = mpsc::channel(32);
                                    let (worker_id, worker_cmd_tx) = Supervisor::spawn_worker(
                                        worker_resp_tx, self.worker_result_tx.clone(), self.task_manager_resp_rx.resubscribe()
                                    );
                                }
                            }
                            // Otherwise assign the plan to a worker...

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
        
                            // reference to worker_assignment hashmap
                            let worker_assignment_ref = Arc::new(Mutex::new(self.worker_assignment.clone()));
                            // TODO: Assign plan to worker
        
                            for &plan_id in &self.plans_to_sync {
                                let task_manager_cmd_tx = self.task_manager_cmd_tx.clone();
                                let worker_cmd_tx_ref_clone = Arc::clone(&worker_cmd_tx_ref);
                                let worker_assignment_ref_clone = Arc::clone(&worker_assignment_ref);
                                
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
                        
                                                // First, determine the suitable worker
                                                let worker_id = {
                                                    let mut worker_assignment = worker_assignment_ref_clone.lock().await;
                                                    worker_assignment.iter_mut().find_map(|(id, pid)| {
                                                        if pid.is_none() {
                                                            *pid = Some(plan_id);
                                                            Some(*id)
                                                        } else {
                                                            None
                                                        }
                                                    })
                                                };
                        
                                                // Then, send the command to the selected worker
                                                if let Some(id) = worker_id {
                                                    if let Some(worker_tx) = worker_cmd_tx_ref_clone.get(&id) {
                                                        let r = worker_tx.send(WorkerCommand::AssignPlan { plan_id, task_receiver, start_immediately: true }).await;
                                                        if let Err(e) = r {
                                                            error!("Failed to send command {}", e);
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });
        
                                tasks.push(task);
                            }
        
                            // Wait for all tasks to finish
                            let _ = futures::future::join_all(tasks).await;
                            // Synchronize the changes back to Supervisor's worker_assignment
                            let updated_worker_assignment = Arc::try_unwrap(worker_assignment_ref)
                                .expect("Arc::try_unwrap failed")
                                .into_inner();
                            self.worker_assignment = updated_worker_assignment;
                        
                            // Notify that all plans have been instructed to start
                            let _ = self.resp_tx.send(SupervisorResponse::AllStarted).await;
                        }
                        SupervisorCommand::CancelAll => {
                            // Logic to cancel all plans...
                            let _ = self.resp_tx.send(SupervisorResponse::AllCancelled).await;
                        } // TODO: Implement worker management commands
                    }
                },
                Some(worker_response) = self.worker_resp_rx.recv() => {
                    // Handle worker responses
                    match worker_response {
                        WorkerResponse::ShutdownComplete(worker_id) => {
                            // Process task completion
                            // ...
                            info!("Worker {} is down.", worker_id);
                        },
                        WorkerResponse::PlanAssignmentConfirmed { worker_id, plan_id } => {
                            // Handle task failure
                            info!("Successfully assigned plan {} to worker {}.", plan_id, worker_id);
                        },
                        // ... handle other worker responses ...
                    }
                }
            }
        }
    }
}
