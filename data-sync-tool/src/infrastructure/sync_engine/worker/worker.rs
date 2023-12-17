use getset::Getters;
use log::{debug, error, info};
use reqwest::Client;
use serde_json::Value;
use tokio::{
    select,
    sync::{broadcast, mpsc},
    task::JoinHandle,
    time::{timeout, Duration, sleep},
};

use uuid::Uuid;

use crate::infrastructure::sync_engine::task_manager::commands::{
    TaskManagerCommand, TaskManagerResponse, TaskRequestResponse,
};

use super::commands::{WorkerCommand, WorkerResponse, WorkerResult};

#[derive(Debug)]
pub enum WorkerError {
    ConnectionTimeout,
    TaskReceiverChannelClosed,
    PlanNotFound(Uuid),
    SendFailed
}

#[derive(Debug)]
pub enum WorkerState {
    Created,
    Idle,
    Ready {
        plan_id: Uuid,
    },
    Working {
        plan_id: Uuid,
        task_id: Option<Uuid>,
        // schedule syncing to a separate task
        task_handle: JoinHandle<Result<(), WorkerError>>,
    },
    Stopped,
}

#[derive(Debug, Getters)]
#[getset(get = "pub")]
pub struct Worker {
    id: Uuid,
    cmd_rx: mpsc::Receiver<WorkerCommand>,
    task_manager_cmd_tx: mpsc::Sender<TaskManagerCommand>,
    tm_resp_rx: broadcast::Receiver<TaskManagerResponse>,
    task_rx: Option<broadcast::Receiver<TaskRequestResponse>>,
    resp_tx: mpsc::Sender<WorkerResponse>,
    result_tx: mpsc::Sender<WorkerResult>,
    assigned_plan_id: Option<Uuid>,
    http_client: Client,
    // TODO: Add websocket streaming support
    state: WorkerState,
    response_timeout: Duration,
    request_per_minute: usize,
}

impl Worker {
    pub fn new(
        id: Uuid,
        cmd_rx: mpsc::Receiver<WorkerCommand>,
        task_manager_cmd_tx: mpsc::Sender<TaskManagerCommand>,
        tm_resp_rx: broadcast::Receiver<TaskManagerResponse>,
        resp_tx: mpsc::Sender<WorkerResponse>,
        result_tx: mpsc::Sender<WorkerResult>,
        response_timeout: Option<Duration>,
        request_per_minute: Option<usize>,
    ) -> Self {
        Self {
            id,
            cmd_rx,
            task_manager_cmd_tx,
            tm_resp_rx,
            task_rx: None,
            resp_tx,
            result_tx,
            assigned_plan_id: None,
            http_client: Client::new(),
            state: WorkerState::Created,
            response_timeout: response_timeout.unwrap_or(Duration::from_secs(5)),
            request_per_minute: request_per_minute.unwrap_or(50),
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
                            break;
                        },
                        WorkerCommand::AssignPlan { plan_id, task_receiver, start_immediately } => {
                            info!("Worker {} assigned to plan {}", self.id, plan_id);
                            self.assigned_plan_id = Some(plan_id);
                            self.state = WorkerState::Ready { plan_id }; // Assuming task_id is generated here
                            debug!("Worker {}'s state: {:?}", self.id, self.state);
                            self.task_rx = Some(task_receiver);

                            if start_immediately {
                                // Start a new task to handle actual data syncing
                                if let Some(task_req_receiver) = self.task_rx {
                                    let task_manager_cmd_tx_clone = self.task_manager_cmd_tx.clone();
                                    let mut task_req_receiver_clone = task_req_receiver.resubscribe();
                                    let result_sender = self.result_tx.clone();
                                    let client = self.http_client.clone();
                                    let task_handle: JoinHandle<Result<(), WorkerError>> = tokio::spawn(async move {
                                        loop {
                                            if let Err(e) = task_manager_cmd_tx_clone.send(TaskManagerCommand::RequestTask(plan_id)).await {
                                                error!("Failed to send task request!");
                                                return Err(WorkerError::SendFailed);
                                            }

                                            match task_req_receiver_clone.recv().await {
                                                Ok(resp) => {
                                                    match resp {
                                                        TaskRequestResponse::NewTask(task) => {
                                                            debug!("Received a task: {:#?}", &task);
                                                            if let Some(request_builder) = task.spec().build_request(&client) {
                                                                match request_builder.send().await {
                                                                    Ok(resp) => {
                                                                        info!("status: {}", resp.status());
                                                                        let parse_result: Result<Value, reqwest::Error> = resp.json().await;
                                                                        match parse_result {
                                                                            Ok(data) => {
                                                                                let task_result = WorkerResult::TaskCompleted {
                                                                                    plan_id: *task.plan_id(),
                                                                                    task_id: *task.task_id(),
                                                                                    result: data,
                                                                                    complete_time: chrono::Local::now()
                                                                                };
                                                                                if let Err(e) = result_sender.send(task_result).await {
                                                                                    error!("Failed to send result: {}",e);
                                                                                }
                                                                            },
                                                                            Err(_) => {
                                                                                error!("Unable to parse result");
                                                                            },
                                                                        }
                                                                    },
                                                                    Err(e) => {
                                                                        error!("Failed to send request: {}", e);
                                                                    }
                                                                }
                                                            }
                                                        },
                                                        TaskRequestResponse::NoTaskLeft => {
                                                            info!("No task left. Sync completed.");
                                                            return Ok(());
                                                        },
                                                        TaskRequestResponse::PlanNotFound(plan_id) => {
                                                            error!("Plan {} not found!", plan_id);
                                                            return Err(WorkerError::PlanNotFound(plan_id));
                                                        },
                                                    }
                                                },
                                                Err(e) => {
                                                    error!("Task receiver channel closed. Exit. Error: {}", e);
                                                    return Err(WorkerError::TaskReceiverChannelClosed);
                                                },
                                            }
                                            sleep(Duration::from_millis(500)).await;
                                        }
                                    });
                                }
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
                response_result = timeout(self.response_timeout, self.tm_resp_rx.recv()) => {
                    match response_result {
                        Ok(result) => {
                            match result {
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
                        },
                        Err(_) => todo!(),
                    }
                }
            }
        }

        if let Err(e) = self
            .resp_tx
            .send(WorkerResponse::ShutdownComplete(self.id))
            .await
        {
            error!("Failed to send shutdown complete. error: {}", e);
        }
    }
}
