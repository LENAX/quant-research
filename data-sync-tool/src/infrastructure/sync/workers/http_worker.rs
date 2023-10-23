/**
 * Implementation of WebAPISyncWorker
 */
use std::sync::Arc;

use async_trait::async_trait;
use chrono::offset::Local;
use derivative::Derivative;
use getset::{Getters, Setters};
use log::{error, info};
use reqwest::{Client, RequestBuilder};
use serde_json::Value;
use tokio::{
    sync::Mutex,
    time::{sleep, Duration},
};
use uuid::Uuid;

use crate::{
    domain::synchronization::sync_task::SyncTask,
    infrastructure::{
        mq::message_bus::MessageBusError,
        sync::{
            shared_traits::{
                SyncTaskMPMCReceiver, SyncTaskMPMCSender,
                TaskRequestMPMCSender,
            },
            GetTaskRequest,
        },
    },
};

use super::{
    errors::SyncWorkerError,
    worker_traits::{
        ShortRunningWorker, ShortTaskHandlingWorker, SyncWorker, WorkerState,
    }, factory::http_worker_factory::{build_headers, build_request, HttpWorkerBuilder, WebAPISyncWorkerBuilder},
};

// Factory Methods


/**
 * WebAPISyncWorker
 * A type of worker that requests for data by sending get or post requests
 */
#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct WebAPISyncWorker<
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
> {
    id: Uuid,
    state: WorkerState,
    http_client: Client,
    task_request_sender: TRS,
    todo_task_receiver: TTR,
    completed_task_sender: CTS,
    failed_task_sender: FTS,
    assigned_sync_plan_id: Option<Uuid>,
}

impl<TRS, TTR, CTS, FTS> WebAPISyncWorker<TRS, TTR, CTS, FTS>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
{
    pub fn new(
        http_client: Client,
        task_request_sender: TRS,
        todo_task_receiver: TTR,
        completed_task_sender: CTS,
        failed_task_sender: FTS,
    ) -> WebAPISyncWorker<TRS, TTR, CTS, FTS> {
        return Self {
            id: Uuid::new_v4(),
            state: WorkerState::default(),
            http_client,
            task_request_sender,
            todo_task_receiver,
            completed_task_sender,
            failed_task_sender,
            assigned_sync_plan_id: None,
        };
    }

    async fn request_task(&mut self) -> Result<Arc<Mutex<SyncTask>>, SyncWorkerError> {
        if !self.is_busy() {
            return Err(SyncWorkerError::NoTaskAssigned);
        }

        let plan_id = self.assigned_sync_plan_id.expect("Why plan id is missing?");
        let task_request = GetTaskRequest::new(plan_id);
        let req_result = self.task_request_sender.send(task_request).await;

        match req_result {
            Err(e) => match e {
                MessageBusError::SendFailed(_, _) => {
                    error!("Worker {} failed to request a task!", self.id);
                    return Err(SyncWorkerError::SendTaskRequestFailed(format!(
                        "Worker {} failed to request a task!",
                        self.id
                    )));
                }
                MessageBusError::SenderClosed => {
                    error!(
                        "Worker {}'s request sender channel is closed unexpectedly!",
                        self.id
                    );
                    return Err(SyncWorkerError::SendTaskRequestFailed(format!(
                        "Worker {}'s request sender channel is closed unexpectedly!",
                        self.id
                    )));
                }
                _ => {
                    error!("Worker {} failed to request a task!", self.id);
                    return Err(SyncWorkerError::SendTaskRequestFailed(format!(
                        "Worker {} failed to request a task!",
                        self.id
                    )));
                }
            },
            Ok(()) => {
                let todo_task_receive_result = self.todo_task_receiver.receive().await;
                match todo_task_receive_result {
                    None => {
                        error!("No task received in worker {}", self.id);
                        return Err(SyncWorkerError::NoTaskAssigned);
                    }
                    Some(task) => {
                        info!("Worker {} received 1 task.", self.id);
                        return Ok(task);
                    }
                }
            }
        }
    }

    fn build_request_header(
        &self,
        sync_task: &SyncTask,
    ) -> Result<RequestBuilder, SyncWorkerError> {
        let headers = build_headers(sync_task.spec().request_header());
        let build_request_result = build_request(
            &self.http_client,
            sync_task.spec().request_endpoint().as_str(),
            sync_task.spec().request_method().to_owned(),
            Some(headers),
            sync_task.spec().payload().clone(),
        );

        match build_request_result {
            Err(e) => {
                return Err(SyncWorkerError::BuildRequestFailed(e.to_string()));
            }
            Ok(request) => {
                return Ok(request);
            }
        }
    }

    async fn execute_task<'a>(
        &'a self,
        sync_task: &'a mut Arc<Mutex<SyncTask>>,
    ) -> Result<&mut Arc<Mutex<SyncTask>>, SyncWorkerError> {
        let mut task_lock = sync_task.lock().await;
        task_lock.start(Local::now());
        let build_req_result = self.build_request_header(&task_lock);
        drop(task_lock);

        match build_req_result {
            Ok(request) => {
                let resp = request.send().await;
                match resp {
                    Ok(resp) => {
                        info!("status: {}", resp.status());
                        let parse_result: Result<Value, reqwest::Error> = resp.json().await;

                        match parse_result {
                            Ok(json_value) => {
                                let mut task_lock = sync_task.lock().await;
                                task_lock
                                    .set_result(Some(json_value))
                                    .finished(Local::now());
                                drop(task_lock);
                                return Ok(sync_task);
                            }
                            Err(e) => {
                                let mut task_lock = sync_task.lock().await;
                                task_lock
                                    .set_result_message(Some(e.to_string()))
                                    .failed(Local::now());
                                drop(task_lock);
                                return Err(SyncWorkerError::JsonParseFailed(e.to_string()));
                            }
                        }
                    }
                    Err(e) => {
                        error!("error: {}", e);
                        let mut task_lock = sync_task.lock().await;
                        task_lock
                            .set_result_message(Some(e.to_string()))
                            .failed(Local::now());
                        drop(task_lock);
                        return Err(SyncWorkerError::WebRequestFailed(e.to_string()));
                    }
                }
            }
            Err(e) => {
                error!("Failed to build request! Reason: {:?}", e);
                let mut task_lock = sync_task.lock().await;
                task_lock
                    .set_result_message(Some(format!("Failed to build request! Reason: {:?}", e)))
                    .failed(Local::now());
                drop(task_lock);
                return Err(e);
            }
        }
    }

    async fn send_completed_task(
        &self,
        completed_task: Arc<Mutex<SyncTask>>,
        n_retry: Option<usize>,
    ) -> Result<(), SyncWorkerError> {
        let task_lock = completed_task.lock().await;
        let task_id = *task_lock.id();
        drop(task_lock);

        for i in 0..n_retry.unwrap_or(5) {
            let send_result = self
                .completed_task_sender
                .send(Arc::clone(&completed_task))
                .await;
            match send_result {
                Ok(()) => {
                    info!(
                        "Successfully sent 1 finished task {} in worker {}!",
                        task_id, self.id
                    );
                    return Ok(());
                }
                Err(e) => {
                    error!(
                        "Failed to send finished task {} in worker {}! Reason: {:?}.",
                        task_id,
                        self.id(),
                        e
                    );
                }
            }
        }

        error!("Task {} is discarded.", task_id);
        Err(SyncWorkerError::CompleteTaskSendFailed)
    }

    async fn send_failed_task(
        &self,
        failed_task: Arc<Mutex<SyncTask>>,
        n_retry: Option<usize>,
    ) -> Result<(), SyncWorkerError> {
        let task_lock = failed_task.lock().await;
        let task_id = *task_lock.id();
        drop(task_lock);
        for i in 0..n_retry.unwrap_or(5) {
            let send_result = self.failed_task_sender.send(failed_task.clone()).await;
            if let Ok(()) = send_result {
                info!("Successfully sent failed task {} back to retry", task_id);
                return Ok(());
            } else {
                error!(
                    "Failed to send back failed task in worker {}! Reason: {:?}. Discard task {}..",
                    self.id(),
                    send_result,
                    task_id
                );
            }
        }
        Err(SyncWorkerError::ResendTaskFailed)
    }

    async fn handle_send_failed_task(
        &self,
        task: Arc<Mutex<SyncTask>>,
    ) -> Result<(), SyncWorkerError> {
        if let Err(_) = self.send_failed_task(task, Some(5)).await {
            error!(
                "Skipped task because worker {} cannot send it back",
                self.id
            );
            // Optionally, you can return the error here if necessary
            // return Err(some_error);
        }
        Ok(())
    }

    fn start_working(&mut self, sync_plan_id: Uuid) {
        self.state = WorkerState::Working;
        self.assigned_sync_plan_id = Some(sync_plan_id);
        info!(
            "Worker {} starts working on plan {}... at {}",
            self.id,
            sync_plan_id,
            Local::now()
        );
    }

    fn wait(&mut self) {
        self.state = WorkerState::Idle;
        info!("Worker {} starts waiting... at {}", self.id, Local::now());
    }

    fn stop(&mut self) {
        self.state = WorkerState::Stopped;
        self.assigned_sync_plan_id = None;
        info!("Worker {} stops working... at {}", self.id, Local::now());
    }

    fn is_busy(&self) -> bool {
        let busy_state = self.state == WorkerState::Working && self.assigned_sync_plan_id != None;
        info!("Is worker {} busy? {}", self.id, busy_state);
        return busy_state;
    }
}

/// WebAPISyncWorker is a ShortTaskHandlingWorker
impl<TRS, TTR, CTS, FTS> ShortTaskHandlingWorker for WebAPISyncWorker<TRS, TTR, CTS, FTS>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
{
}

impl<TRS, TTR, CTS, FTS> ShortRunningWorker for WebAPISyncWorker<TRS, TTR, CTS, FTS>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
{
}

#[async_trait]
impl<TRS, TTR, CTS, FTS> SyncWorker for WebAPISyncWorker<TRS, TTR, CTS, FTS>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
{
    type BuilderType = WebAPISyncWorkerBuilder<TRS, TTR, CTS, FTS>;

    async fn start_sync(&mut self) -> Result<(), SyncWorkerError> {
        if self.assigned_sync_plan_id.is_none() {
            return Err(SyncWorkerError::NoPlanAssigned(format!("Worker {} has no assigned sync plan", self.id)));
        }

        self.start_working(self.assigned_sync_plan_id.expect("Assigned sync plan id should not be empty"));

        while self.is_busy() {
            // request a task
            let task_request_result = self.request_task().await;
            match task_request_result {
                Ok(mut task) => {
                    let task_lock = task.lock().await;
                    let task_id = *task_lock.id();
                    drop(task_lock);

                    let result = self.execute_task(&mut task).await;

                    match result {
                        Ok(finished_task) => {
                            let send_result = self
                                .send_completed_task(finished_task.clone(), Some(5))
                                .await;
                            if send_result.is_err() {
                                error!("Cannot send task! Discarded.");
                            }
                        }
                        Err(e) => match e {
                            SyncWorkerError::BuildRequestFailed(build_err) => {
                                error!("Failed to build request header in worker {}", self.id);
                                self.wait();
                                return Err(SyncWorkerError::BuildRequestFailed(build_err));
                            }
                            SyncWorkerError::ConnectionDroppedTimeout => {
                                error!(
                                    "Connection timed out while requesting data for plan {:?}.",
                                    self.assigned_sync_plan_id()
                                );
                                let _ = self.handle_send_failed_task(task).await;
                            }
                            SyncWorkerError::JsonParseFailed(reason) => {
                                error!(
                                    "Failed to parse result in worker {}. Reason: {}",
                                    self.id, reason
                                );
                                let _ = self.handle_send_failed_task(task).await;
                            }
                            SyncWorkerError::NoDataReceived => {
                                error!(
                                    "No data received while executing task {} in worker {}.",
                                    task_id, self.id
                                );
                                let _ = self.handle_send_failed_task(task).await;
                            }
                            SyncWorkerError::WebRequestFailed(reason) => {
                                error!(
                                    "WebRequestFailed while executing task {} in worker {}.",
                                    task_id, self.id
                                );
                                let _ = self.handle_send_failed_task(task).await;
                            }
                            SyncWorkerError::WorkerStopped => {
                                error!("Worker {} is asked to stop!", self.id);
                                return Err(e);
                            }
                            _ => {
                                error!("Encountered error: {:?}", e);
                                self.wait();
                                return Err(e);
                            }
                        },
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to fetch task in worker {}! Reason: {:?}",
                        self.id, e
                    );
                }
            }
            sleep(Duration::from_millis(500)).await;
        }

        // for compilation purpose
        Ok(())
    }

    fn pause(&mut self) -> Result<(), SyncWorkerError> {
        Ok(self.wait())
    }

    fn stop(&mut self) -> Result<(), SyncWorkerError> {
        Ok(self.stop())
    }

    fn assign_sync_plan(&mut self, sync_plan_id: &Uuid) -> Result<(), SyncWorkerError> {
        if self.is_busy() {
            self.stop();
        }
        self.assigned_sync_plan_id = Some(*sync_plan_id);
        Ok(())
    }

    fn current_state(&self) -> WorkerState {
        return self.state;
    }
}