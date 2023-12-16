//! Supervisor Implementation
//! Serve the role of managing and coordinating multiple workers
//!

use std::collections::{HashMap, HashSet};

use getset::Getters;
use log::{info, error, debug};
use tokio::{sync::{broadcast, mpsc}, select, time::{sleep, Duration}};
use uuid::Uuid;

use crate::infrastructure::sync_engine::{
    task_manager::commands::{TaskManagerResponse, TaskManagerCommand}, ComponentState,
};

use super::{
    commands::{
        SupervisorCommand, SupervisorResponse, WorkerCommand, WorkerResponse, WorkerResult,
    },
    worker::Worker,
};

type WorkerId = Uuid;
type PlanId = Uuid;

#[derive(Debug, PartialEq, Eq)]
pub enum WorkerAssignmentState {
    Idle,
    Ready, // Picked for plan assignment
    PlanAssigned(PlanId),
    Running(PlanId)
}


#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct Supervisor {
    cmd_rx: mpsc::Receiver<SupervisorCommand>,
    resp_tx: mpsc::Sender<SupervisorResponse>,
    task_manager_cmd_tx: mpsc::Sender<TaskManagerCommand>,
    task_manager_resp_rx: broadcast::Receiver<TaskManagerResponse>,
    worker_cmd_tx: HashMap<WorkerId, mpsc::Sender<WorkerCommand>>,
    worker_resp_tx: mpsc::Sender<WorkerResponse>,
    worker_resp_rx: mpsc::Receiver<WorkerResponse>,
    worker_result_tx: mpsc::Sender<WorkerResult>,
    plans_to_sync: HashSet<Uuid>,
    worker_assignment: HashMap<WorkerId, WorkerAssignmentState>,
    state: ComponentState,
}

impl Supervisor {
    
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

            worker_assignment.insert(worker_id, WorkerAssignmentState::Idle);
            worker_cmd_tx.insert(worker_id, tx);
        }

        (
            Supervisor {
                cmd_rx,
                resp_tx,
                task_manager_cmd_tx,
                task_manager_resp_rx,
                worker_cmd_tx,
                worker_resp_tx,
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

                            let _ = futures::future::join_all(tasks).await;
                            info!("Waiting for all workers to shutdown...");
                            while self.worker_assignment.len() > 0 {
                                sleep(Duration::from_millis(100)).await;
                            }

                            info!("Shutting down Supervisor...");
                            self.state = ComponentState::Stopped;
                            let _ = self
                                .resp_tx
                                .send(SupervisorResponse::ShutdownComplete)
                                .await;
                            break;
                        }
                        SupervisorCommand::AssignPlan { plan_id, start_immediately } => {
                            // Register new plan
                            // self.plans_to_sync.insert(plan_id);

                            // Find an idle worker and update its state
                            let worker_id = {
                                self.worker_assignment.iter_mut().find_map(|(id, state)| {
                                    if *state == WorkerAssignmentState::Idle {
                                        *state = WorkerAssignmentState::Ready;
                                        Some(*id)
                                    } else {
                                        None
                                    }
                                })
                            };

                            // if no worker is available, spawn a new worker
                            let mut new_worker_id = Uuid::new_v4();
                            if worker_id.is_none() {
                                let (worker_id, worker_cmd_tx) = Supervisor::spawn_worker(
                                        self.worker_resp_tx.clone(), 
                                        self.worker_result_tx.clone(), 
                                        self.task_manager_resp_rx.resubscribe()
                                    );
                                // register new worker
                                self.worker_cmd_tx.insert(worker_id, worker_cmd_tx);
                                self.worker_assignment.insert(worker_id, WorkerAssignmentState::Ready);
                                new_worker_id = worker_id;
                                debug!("Registered new worker {}", worker_id);
                            }

                            // assign the plan to a worker...
                            let worker_cmd_sender_result = self.worker_cmd_tx.get(&worker_id.unwrap_or(new_worker_id));
                            if let Some(worker_cmd_sender) = worker_cmd_sender_result {
                                info!("Requesting task receiver...");
                                let _ = self.task_manager_cmd_tx.send(TaskManagerCommand::RequestTaskReceiver { plan_id }).await;
                                if let Ok(response) = self.task_manager_resp_rx.recv().await {
                                    if let TaskManagerResponse::TaskChannel { plan_id: received_plan_id, task_sender } = response {
                                        if received_plan_id == plan_id {
                                            let task_receiver = task_sender.subscribe();
                                            let send_result = worker_cmd_sender.send(WorkerCommand::AssignPlan {
                                                plan_id: plan_id, task_receiver: task_receiver, start_immediately
                                            }).await;
                                            if let Err(e) = send_result {
                                                error!("Failed to send command to worker {}: {}", &worker_id.unwrap_or(new_worker_id), e);
                                            }
                                        }
                                    }
                                }   
                            }

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
                            // Instruct Workers to Start Syncing All Plans
        
                            let mut tasks = Vec::new();
                            let worker_cmd_tx_ref = &self.worker_cmd_tx;

                            for (&worker_id, state) in self.worker_assignment.iter() {
                                if matches!(state, WorkerAssignmentState::PlanAssigned(_)) {
                                    let worker_cmd_sender = worker_cmd_tx_ref.get(&worker_id);
                                    match worker_cmd_sender {
                                        Some(sender) => {
                                            let sender_clone = sender.clone();
                                            let task = tokio::spawn(async move {
                                                let send_result = sender_clone.send(WorkerCommand::StartSync).await;
                                                if let Err(_) = send_result {
                                                    error!("Failed to send start command to worker {}", &worker_id);
                                                }
                                            });
                                            tasks.push(task);
                                        },
                                        None => {
                                            error!("Worker command sender not found!")
                                        }
                                    }
                                }
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
                },
                Some(worker_response) = self.worker_resp_rx.recv() => {
                    // Handle worker responses
                    match worker_response {
                        WorkerResponse::ShutdownComplete(worker_id) => {
                            // Process task completion
                            // need to confirm worker is down
                            info!("Worker {} is down.", worker_id);

                            // Remove it from worker assignment map
                            self.worker_assignment.remove(&worker_id);
                        },
                        WorkerResponse::PlanAssigned { worker_id, plan_id, sync_started } => {
                            // Handle task failure
                            info!("Successfully assigned plan {} to worker {}.", plan_id, worker_id);
                            if sync_started {
                                self.worker_assignment.insert(worker_id, WorkerAssignmentState::Running(plan_id));
                            } else {
                                self.worker_assignment.insert(worker_id, WorkerAssignmentState::PlanAssigned(plan_id));
                            }
                            
                        },
                        WorkerResponse::PlanCancelled { worker_id, plan_id } => {
                            todo!()
                        },
                        WorkerResponse::StartOk { worker_id, plan_id } => {
                            info!("Worker {} is syncing plan {}", worker_id, plan_id);
                            let worker_state = self.worker_assignment.get_mut(&worker_id);
                            if let Some(worker_state) = worker_state {
                                if *worker_state == WorkerAssignmentState::PlanAssigned(plan_id) {
                                    *worker_state = WorkerAssignmentState::Running(plan_id);
                                }
                            }
                        },
                        WorkerResponse::StartFailed(reason) => {
                            error!("Failed to start worker because {}", reason)
                        }
                        // ... handle other worker responses ...
                    }
                }
            }
        }
    }
}
