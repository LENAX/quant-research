use getset::Getters;
use log::{debug, error, info};
use reqwest::Client;
use serde_json::Value;
use tokio::{
    select,
    sync::{broadcast, mpsc},
    task::JoinHandle,
    time::{sleep, timeout, Duration},
};

use uuid::Uuid;

use crate::infrastructure::sync_engine::task_manager::{
    commands::{TaskManagerCommand, TaskManagerResponse, TaskRequestResponse},
    task_manager::Task,
};

use super::commands::{WorkerCommand, WorkerResponse, WorkerResult};

#[derive(Debug)]
pub enum WorkerError {
    ConnectionTimeout,
    TaskReceiverChannelClosed,
    PlanNotFound(Uuid),
    SendFailed,
    BuildRequestFailed,
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
        // schedule syncing to a separate task
        task_handle: JoinHandle<Result<(), WorkerError>>,
    },
    Stopped,
}

impl PartialEq for WorkerState {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (WorkerState::Idle, WorkerState::Idle) => true,
            (
                WorkerState::Working { plan_id: id1, .. },
                WorkerState::Working { plan_id: id2, .. },
            ) => id1 == id2,
            // Add comparisons for other variants...
            _ => false,
        }
    }
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

    pub async fn run(&mut self) {
        while let Some(command) = self.cmd_rx.recv().await {
            match command {
                WorkerCommand::Shutdown => self.handle_shutdown().await,
                WorkerCommand::AssignPlan {
                    plan_id,
                    task_receiver,
                    start_immediately,
                } => {
                    self.handle_assign_plan(plan_id, task_receiver, start_immediately)
                        .await
                }
                WorkerCommand::StartSync => self.handle_start_sync().await,
                WorkerCommand::CancelPlan(plan_id) => self.handle_cancel_plan(plan_id).await, // ... other commands ...
            }
        }
    }

    // Handling shutdown
    async fn handle_shutdown(&mut self) {
        if let WorkerState::Working { task_handle, .. } = &self.state {
            task_handle.abort();
        }
        self.state = WorkerState::Stopped;
        let _ = self
            .resp_tx
            .send(WorkerResponse::ShutdownComplete(self.id))
            .await;
    }

    // Assigning a new plan
    async fn handle_assign_plan(
        &mut self,
        plan_id: Uuid,
        task_receiver: broadcast::Receiver<TaskRequestResponse>,
        start_immediately: bool,
    ) {
        let task_receiver_clone = task_receiver.resubscribe();
        self.assigned_plan_id = Some(plan_id);
        self.task_rx = Some(task_receiver);

        if start_immediately {
            self.start_syncing_plan(
                plan_id,
                self.task_manager_cmd_tx.clone(),
                task_receiver_clone,
            )
            .await;
        } else {
            self.state = WorkerState::Ready { plan_id };
        }
    }

    // Starting synchronization
    async fn handle_start_sync(&mut self) {
        if let Some(plan_id) = self.assigned_plan_id {
            if let Some(task_rx) = &self.task_rx {
                let task_rx_clone = task_rx.resubscribe();
                self.start_syncing_plan(plan_id, self.task_manager_cmd_tx.clone(), task_rx_clone)
                    .await;
            }
        }
    }

    // Canceling a plan
    async fn handle_cancel_plan(&mut self, plan_id: Uuid) {
        if let WorkerState::Working {
            task_handle,
            plan_id: current_plan_id,
        } = &self.state
        {
            if *current_plan_id == plan_id {
                task_handle.abort();
                self.state = WorkerState::Idle;
                self.assigned_plan_id = None;
                let _ = self
                    .resp_tx
                    .send(WorkerResponse::PlanCancelled {
                        worker_id: self.id,
                        plan_id,
                    })
                    .await;
            }
        }
    }

    // Starting the task processing loop for a specific plan
    async fn start_syncing_plan(
        &mut self,
        plan_id: Uuid,
        task_manager_cmd_tx: mpsc::Sender<TaskManagerCommand>,
        mut task_req_receiver: broadcast::Receiver<TaskRequestResponse>,
    ) {
        let result_sender = self.result_tx.clone();
        let client = self.http_client.clone();
        let task_handle: JoinHandle<Result<(), WorkerError>> = tokio::spawn(async move {
            loop {
                // TODO: Add rate limiting

                if let Err(e) = task_manager_cmd_tx
                    .send(TaskManagerCommand::RequestTask(plan_id))
                    .await
                {
                    error!("Failed to send task request! Error: {}", e);
                    return Err(WorkerError::SendFailed);
                }

                match task_req_receiver.recv().await {
                    Ok(resp) => match resp {
                        TaskRequestResponse::NewTask(task) => {
                            debug!("Received a task: {:#?}", &task);
                            Worker::process_tasks(&task, &client, &result_sender);
                        }
                        TaskRequestResponse::NoTaskLeft => {
                            info!("No task left. Sync completed.");
                            return Ok(());
                        }
                        TaskRequestResponse::PlanNotFound(plan_id) => {
                            error!("Plan {} not found!", plan_id);
                            return Err(WorkerError::PlanNotFound(plan_id));
                        }
                    },
                    Err(e) => {
                        error!("Task receiver channel closed. Exit. Error: {}", e);
                        return Err(WorkerError::TaskReceiverChannelClosed);
                    }
                }
                sleep(Duration::from_millis(500)).await;
            }
        });

        self.state = WorkerState::Working {
            plan_id,
            task_handle,
        };
    }

    // Process tasks by sending request to remote data provider
    async fn process_tasks(task: &Task, client: &Client, result_tx: &mpsc::Sender<WorkerResult>) {
        if let Some(request_builder) = task.spec().build_request(client) {
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
                                complete_time: chrono::Local::now(),
                            };
                            if let Err(e) = result_tx.send(task_result).await {
                                error!("Failed to send result: {}", e);
                            }
                        }
                        Err(_) => {
                            error!("Unable to parse result");
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to send request: {}", e);
                }
            }
        }
    }
}
