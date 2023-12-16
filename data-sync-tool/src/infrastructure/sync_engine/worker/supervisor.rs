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
                            self.handle_shutdown().await;
                        },
                        SupervisorCommand::AssignPlan { plan_id, start_immediately } => {
                            self.handle_assign_plan(plan_id, start_immediately).await;
                        },
                        SupervisorCommand::CancelPlan(plan_id) => {
                            self.handle_cancel_plan(plan_id).await;
                        },
                        SupervisorCommand::StartAll => {
                            self.handle_start_all().await;
                        },
                        SupervisorCommand::CancelAll => {
                            self.handle_cancel_all().await;
                        }
                    }
                },
                Some(worker_response) = self.worker_resp_rx.recv() => {
                    self.handle_worker_response(worker_response).await;
                }
            }
        }
    }

    async fn handle_shutdown(&mut self) {
        info!("Received shutdown command.");
        info!("Shutting down Workers...");
    
        let mut tasks = Vec::new();
        for (&worker_id, worker_tx) in self.worker_cmd_tx.iter() {
            let worker_tx_clone = worker_tx.clone();
            let worker_id_clone = worker_id; // Clone the worker_id
    
            let task = tokio::spawn(async move {
                if let Err(e) = worker_tx_clone.send(WorkerCommand::Shutdown).await {
                    error!("Failed to shutdown worker {}, Error: {}", worker_id_clone, e);
                }
            });
            tasks.push(task);
        }
    
        let _ = futures::future::join_all(tasks).await;
        info!("Waiting for all workers to shutdown...");
    
        while self.worker_assignment.len() > 0 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    
        info!("Shutting down Supervisor...");
        self.state = ComponentState::Stopped;
        let _ = self.resp_tx.send(SupervisorResponse::ShutdownComplete).await;
    }
    

    async fn handle_assign_plan(&mut self, plan_id: Uuid, start_immediately: bool) {
        // Register new plan
        // self.plans_to_sync.insert(plan_id);
    
        let worker_id = self.worker_assignment.iter_mut().find_map(|(id, state)| {
            if *state == WorkerAssignmentState::Idle {
                *state = WorkerAssignmentState::Ready;
                Some(*id)
            } else {
                None
            }
        });
    
        if let Some(worker_id) = worker_id {
            self.assign_plan_to_worker(worker_id, plan_id, start_immediately).await;
        } else {
            // If no worker is available, consider handling it (e.g., logging, spawning a new worker, etc.)
            error!("No available worker for plan {}", plan_id);
        }
    }

    async fn handle_cancel_plan(&mut self, plan_id: Uuid) {
        info!("Cancelling plan {}...", plan_id);
    
        let worker_id = self.worker_assignment.iter().find_map(|(id, state)| {
            if *state == WorkerAssignmentState::Running(plan_id) || *state == WorkerAssignmentState::PlanAssigned(plan_id) {
                Some(*id)
            } else {
                None
            }
        });
    
        if let Some(worker_id) = worker_id {
            if let Some(sender) = self.worker_cmd_tx.get(&worker_id) {
                if let Err(e) = sender.send(WorkerCommand::CancelPlan(plan_id)).await {
                    error!("Failed to send cancel command to worker {}: {}", worker_id, e);
                } else {
                    info!("Plan cancellation command sent to worker {}", worker_id);
                }
            } else {
                error!("Worker command sender not found for worker {}", worker_id);
            }
        } else {
            error!("Worker not found for plan {}", plan_id);
        }
    }
    
    async fn handle_start_all(&mut self) {
        let mut tasks = Vec::new();
        for (&worker_id, state) in self.worker_assignment.iter() {
            if matches!(state, WorkerAssignmentState::PlanAssigned(_)) {
                if let Some(sender) = self.worker_cmd_tx.get(&worker_id) {
                    let sender_clone = sender.clone();
                    let task = tokio::spawn(async move {
                        if let Err(e) = sender_clone.send(WorkerCommand::StartSync).await {
                            error!("Failed to send start command to worker {}: {}", worker_id, e);
                        }
                    });
                    tasks.push(task);
                }
            }
        }
    
        let _ = futures::future::join_all(tasks).await;
        let _ = self.resp_tx.send(SupervisorResponse::AllStarted).await;
    }

    async fn handle_cancel_all(&mut self) {
        let mut tasks = Vec::new();
        for (&worker_id, state) in self.worker_assignment.iter() {
            if let WorkerAssignmentState::Running(plan_id) | WorkerAssignmentState::PlanAssigned(plan_id) = state {
                if let Some(sender) = self.worker_cmd_tx.get(&worker_id) {
                    let sender_clone = sender.clone();
                    let plan_id_clone = *plan_id;
                    let task = tokio::spawn(async move {
                        if let Err(e) = sender_clone.send(WorkerCommand::CancelPlan(plan_id_clone)).await {
                            error!("Failed to send cancel command for plan {} to worker {}: {}", plan_id_clone, worker_id, e);
                        }
                    });
                    tasks.push(task);
                }
            }
        }
    
        let _ = futures::future::join_all(tasks).await;
        let _ = self.resp_tx.send(SupervisorResponse::AllCancelled).await;
    }
    

    async fn handle_worker_response(&mut self, response: WorkerResponse) {
        match response {
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
                let _ = self
                    .resp_tx
                    .send(SupervisorResponse::PlanAssigned { plan_id })
                    .await;
            },
            WorkerResponse::PlanCancelled { worker_id, plan_id } => {
                let worker_state = self.worker_assignment.get_mut(&worker_id);
                if let Some(worker_state) = worker_state {
                    if  matches!(worker_state, WorkerAssignmentState::PlanAssigned(_) | WorkerAssignmentState::Running(_)) {
                        *worker_state = WorkerAssignmentState::Idle;
                    }
                }

                info!("Successfully cancelled plan {} for worker {}.", plan_id, worker_id);
                let _ = self
                        .resp_tx
                        .send(SupervisorResponse::PlanCancelled { worker_id, plan_id })
                        .await;
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
            WorkerResponse::StartFailed{ worker_id, reason } => {
                error!("Failed to start worker {} because {}",worker_id, reason)
            }
            // ... handle other worker responses ...
        }
    
        // Send a response to the client module if necessary
        // For example, you might send a response for certain worker actions
    }
    
    async fn assign_plan_to_worker(&mut self, worker_id: Uuid, plan_id: Uuid, start_immediately: bool) {
        // Request the task receiver for the plan from the Task Manager
        let _ = self.task_manager_cmd_tx.send(TaskManagerCommand::RequestTaskReceiver { plan_id }).await;
    
        // Await the response from the Task Manager
        if let Ok(response) = self.task_manager_resp_rx.recv().await {
            if let TaskManagerResponse::TaskChannel { plan_id: received_plan_id, task_sender } = response {
                if received_plan_id == plan_id {
                    let task_receiver = task_sender.subscribe();
    
                    // Send the AssignPlan command to the worker
                    if let Some(worker_cmd_sender) = self.worker_cmd_tx.get(&worker_id) {
                        let command = WorkerCommand::AssignPlan {
                            plan_id,
                            task_receiver,
                            start_immediately
                        };
                        if let Err(e) = worker_cmd_sender.send(command).await {
                            error!("Failed to send AssignPlan command to worker {}: {}", worker_id, e);
                        } else {
                            info!("Assigned plan {} to worker {}.", plan_id, worker_id);
                        }
                    } else {
                        error!("Worker command sender not found for worker {}", worker_id);
                    }
                }
            }
        } else {
            error!("Failed to receive task channel from Task Manager for plan {}", plan_id);
        }
    }
    
}
