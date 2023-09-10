//! Synchronization Worker
//! Handles synchronization task and sends web requests to remote data vendors
//!

use std::sync::Arc;

use async_trait::async_trait;
use chrono::offset::Local;
use derivative::Derivative;
use getset::{Getters, MutGetters, Setters};
use log::{error, info};
use reqwest::{Client, RequestBuilder};
use serde_json::Value;
use tokio::{
    sync::Mutex,
    time::{sleep, Duration},
};
use tungstenite::{connect, Message};
use url::Url;
use uuid::Uuid;

use crate::{
    domain::synchronization::{sync_task::SyncTask, value_objects::task_spec::RequestMethod},
    infrastructure::{
        mq::message_bus::{MessageBusError, MessageBusReceiver, MessageBusSender},
        sync::{
            shared_traits::{
                StreamingData, StreamingDataMPMCSender, SyncTaskMPMCReceiver, SyncTaskMPMCSender,
                SyncWorkerErrorMPMCSender, TaskRequestMPMCSender,
            },
            GetTaskRequest,
        },
    },
};

use super::{
    errors::SyncWorkerError,
    factory_::{build_headers, build_request, WebAPISyncWorkerBuilder},
    worker_traits::{
        LongTaskHandlingWorker, ShortRunningWorker, ShortTaskHandlingWorker, SyncWorker,
    },
};

// Factory Methods

#[derive(Derivative, PartialEq, Eq, Clone, Copy)]
#[derivative(Default(bound = ""))]
pub enum WorkerState {
    #[derivative(Default)]
    // start with this state
    // change from working to Idle when `pause` is called
    Idle,
    // Becomes working state when `start_sync` is called
    Working,
    // Becomes this state when `stop` is called
    Stopped,
}

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

    pub fn builder() -> WebAPISyncWorkerBuilder<TRS, TTR, CTS, FTS> {
        WebAPISyncWorkerBuilder::new()
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
    async fn start_sync(&mut self, sync_plan_id: &Uuid) -> Result<(), SyncWorkerError> {
        self.start_working(*sync_plan_id);

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

    fn reassign_sync_plan(&mut self, sync_plan_id: &Uuid) -> Result<(), SyncWorkerError> {
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

#[derive(Derivative, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct WebsocketSyncWorker<TRS, TTR, SDS, ES> {
    id: Uuid,
    state: WorkerState,
    // channel of requesting task from task manager
    task_request_sender: Option<TRS>,
    todo_task_receiver: Option<TTR>,
    // send data received from remote
    data_sender: Option<SDS>,
    // send error messages
    error_sender: Option<ES>,
    assigned_sync_plan_id: Option<Uuid>,
}

/// WebsocketSyncWorker is a long running work
impl<TRS, TTR, SDS, ES> LongTaskHandlingWorker for WebsocketSyncWorker<TRS, TTR, SDS, ES>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    SDS: StreamingDataMPMCSender,
    ES: SyncWorkerErrorMPMCSender,
{
}

type WSConnection =
    tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>;

impl<TRS, TTR, SDS, ES> WebsocketSyncWorker<TRS, TTR, SDS, ES>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    SDS: StreamingDataMPMCSender,
    ES: SyncWorkerErrorMPMCSender,
{
    pub fn new(
        task_request_sender: TRS,
        todo_task_receiver: TTR,
        data_sender: SDS,
        error_sender: ES,
    ) -> Self {
        return Self {
            id: Uuid::new_v4(),
            state: WorkerState::Idle,
            task_request_sender: Some(task_request_sender),
            todo_task_receiver: Some(todo_task_receiver),
            data_sender: Some(data_sender),
            error_sender: Some(error_sender),
            assigned_sync_plan_id: None,
        };
    }

    async fn request_task(&mut self) -> Result<Arc<Mutex<SyncTask>>, SyncWorkerError> {
        if !self.is_busy() {
            return Err(SyncWorkerError::NoTaskAssigned);
        }

        let plan_id = self.assigned_sync_plan_id.expect("Why plan id is missing?");
        let task_request = GetTaskRequest::new(plan_id);
        let task_request_sender = self.inner_task_request_sender()?;
        let req_result = task_request_sender.send(task_request).await;

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
                let todo_task_receiver = self.inner_todo_task_receiver()?;

                let todo_task_receive_result = todo_task_receiver.receive().await;
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

    fn start_working(&mut self, sync_plan_id: Uuid) {
        self.state = WorkerState::Working;
        self.assigned_sync_plan_id = Some(sync_plan_id);
        info!(
            "WebsocketSyncWorker {} starts working on plan {}... at {}",
            self.id,
            sync_plan_id,
            Local::now()
        );
    }

    fn wait(&mut self) {
        self.state = WorkerState::Idle;
        info!(
            "WebsocketSyncWorker {} starts waiting... at {}",
            self.id,
            Local::now()
        );
    }

    fn stop(&mut self) {
        self.state = WorkerState::Stopped;
        self.assigned_sync_plan_id = None;
        info!(
            "WebsocketSyncWorker {} stops working... at {}",
            self.id,
            Local::now()
        );
    }

    fn is_busy(&self) -> bool {
        let busy_state = self.state == WorkerState::Working && self.assigned_sync_plan_id != None;
        info!("Is worker {} busy? {}", self.id, busy_state);
        return busy_state;
    }

    pub fn close_all_channels(&mut self) {
        info!("Closing data sender channel...");
        self.data_sender = None;
        info!("Done! Closing todo_task_receiver channel...");
        self.todo_task_receiver = None;
        info!("Done! Closing task_request_sender channel...");
        self.task_request_sender = None;
        info!("Done! Closing error_sender channel...");
        self.error_sender = None;
        self.state = WorkerState::Stopped;
        info!("All channel has closed.");
    }

    pub fn inner_data_sender(&self) -> Result<&SDS, SyncWorkerError> {
        if self.state == WorkerState::Stopped {
            return Err(SyncWorkerError::WorkerStopped);
        }
        match self.data_sender() {
            Some(_inner_sender) => Ok(_inner_sender),
            None => Err(SyncWorkerError::WorkerStopped),
        }
    }

    pub fn inner_task_request_sender(&self) -> Result<&TRS, SyncWorkerError> {
        match self.task_request_sender() {
            Some(_inner_sender) => Ok(_inner_sender),
            None => Err(SyncWorkerError::WorkerStopped),
        }
    }

    pub fn inner_todo_task_receiver(&mut self) -> Result<&mut TTR, SyncWorkerError> {
        match self.todo_task_receiver_mut() {
            Some(_inner_sender) => Ok(_inner_sender),
            None => Err(SyncWorkerError::WorkerStopped),
        }
    }

    pub fn inner_error_sender(&self) -> Result<&ES, SyncWorkerError> {
        match self.error_sender() {
            Some(_inner_sender) => Ok(_inner_sender),
            None => Err(SyncWorkerError::WorkerStopped),
        }
    }

    fn validate_task_spec(
        &mut self,
        sync_task: &Arc<Mutex<SyncTask>>,
    ) -> Result<(), SyncWorkerError> {
        let task_lock = sync_task.blocking_lock();
        if *task_lock.spec().request_method() != RequestMethod::Websocket {
            self.wait();
            return Err(SyncWorkerError::BuildRequestFailed(String::from(
                "Request method not supported!",
            )));
        } else {
            Ok(())
        }
    }

    async fn try_connect_ws_endpoint<'a>(
        &self,
        request_url: &Url,
    ) -> Result<WSConnection, SyncWorkerError> {
        // let connect_result = tokio::spawn_blo
        // connect(request_url);
        let url = request_url.clone();
        let join_result = tokio::task::spawn_blocking(move || {
            let conn = connect(url);
            return conn;
        })
        .await;

        match join_result {
            Ok(conn_result) => match conn_result {
                Ok(conn) => {
                    let (ws_connection, other) = conn;
                    info!("Response from remote: {:?}", other);
                    return Ok(ws_connection);
                }
                Err(e) => {
                    return Err(SyncWorkerError::WebsocketConnectionFailed(e.to_string()));
                }
            },
            Err(e) => {
                error!(
                    "Encountered JoinError in WebsocketWorker {}. Original Error: {:?}",
                    self.id, e
                );
                return Err(SyncWorkerError::WebsocketConnectionFailed(e.to_string()));
            }
        }
    }

    async fn try_send_message(&self, connection: &mut WSConnection, message: String) {
        let _ = tokio::task::block_in_place(|| {
            connection
                .write_message(Message::Text(message))
                .expect("Failed to send message!");
        });
    }

    async fn try_read_message(
        &self,
        connection: &mut WSConnection,
    ) -> Result<Message, SyncWorkerError> {
        let result = tokio::task::block_in_place(|| {
            let msg = connection.read_message();
            return msg;
        });
        match result {
            Ok(msg) => {
                return Ok(msg);
            }
            Err(e) => {
                error!("{:?}", e);
                return Err(SyncWorkerError::WSReadError(e.to_string()));
            }
        }
    }

    fn try_parse_message(&self, message: &str) -> Result<Value, SyncWorkerError> {
        let parse_result: Result<Value, serde_json::Error> = serde_json::from_str(message);
        match parse_result {
            Ok(value) => {
                return Ok(value);
            }
            Err(e) => {
                error!("Failed to parse text to json value");
                return Err(SyncWorkerError::JsonParseFailed(e.to_string()));
            }
        }
    }

    async fn try_send_data(&self, task_id: Uuid, data: Value) -> Result<(), SyncWorkerError> {
        let data_sender = self.inner_data_sender()?;
        if self.assigned_sync_plan_id.is_none() {
            error!(
                "While trying to send data, no sync plan is assigned to worker {}",
                self.id
            );
            return Err(SyncWorkerError::NoTaskAssigned);
        }

        let plan_id = self.assigned_sync_plan_id().unwrap();
        let stream_data = StreamingData::new(plan_id, task_id, Some(data), Local::now());
        let send_result = data_sender.send(stream_data).await;

        match send_result {
            Ok(_) => return Ok(()),
            Err(e) => {
                return Err(SyncWorkerError::WSStreamDataSendFailed(e.to_string()));
            }
        }
    }
}

#[async_trait]
impl<TRS, TTR, SDS, ES> SyncWorker for WebsocketSyncWorker<TRS, TTR, SDS, ES>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    SDS: StreamingDataMPMCSender,
    ES: SyncWorkerErrorMPMCSender,
{
    // type BuilderType = WebAPISyncWorkerBuilder<TRS, TTR, CTS, FTS>;

    fn pause(&mut self) -> Result<(), SyncWorkerError> {
        Ok(self.wait())
    }

    fn stop(&mut self) -> Result<(), SyncWorkerError> {
        self.close_all_channels();
        Ok(self.stop())
    }

    fn reassign_sync_plan(&mut self, sync_plan_id: &Uuid) -> Result<(), SyncWorkerError> {
        if self.is_busy() {
            self.stop();
        }
        self.assigned_sync_plan_id = Some(*sync_plan_id);
        Ok(())
    }

    fn current_state(&self) -> WorkerState {
        return self.state.clone();
    }

    async fn start_sync(&mut self, sync_plan_id: &Uuid) -> Result<(), SyncWorkerError> {
        self.start_working(*sync_plan_id);

        let sync_task = self.request_task().await?;
        let mut task_lock = sync_task.lock().await;
        let request_url = task_lock.spec().request_endpoint();
        let task_id = *task_lock.id();
        let msg_body = task_lock.spec().payload().clone();
        let mut connection = self.try_connect_ws_endpoint(request_url).await?;
        task_lock.start(Local::now());
        drop(task_lock);
        
        if let Some(body) = msg_body {
            let text_msg = body.to_string();
            self.try_send_message(&mut connection, text_msg).await;
        }

        while self.is_busy() {
            let read_result = self.try_read_message(&mut connection).await;
            match read_result {
                Ok(msg) => match msg {
                    Message::Text(s) => {
                        let parse_result = self.try_parse_message(&s);
                        match parse_result {
                            Ok(data) => {
                                let _ = self.try_send_data(task_id, data).await;
                            }
                            Err(e) => {
                                error!("Cannot parse value in websocket sync worker {}", self.id);
                            }
                        }
                    }
                    _ => {
                        info!("Received other message: {}", msg);
                    }
                },
                Err(e) => {
                    error!("{:?}", e);
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use log::{error, info, warn};
    use std::collections::HashMap;
    use uuid::Uuid;

    use env_logger;
    use std::env;

    use chrono::Local;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::name::en::Name;
    use fake::Fake;
    use rand::Rng;

    use super::*;
    use crate::{
        domain::synchronization::{
            sync_task::SyncTask,
            value_objects::task_spec::{RequestMethod, TaskSpecification},
        },
        infrastructure::mq::factory::{
            create_tokio_broadcasting_channel, create_tokio_mpsc_channel,
        },
    };
    use serde_json::json;
    use serde_json::Value;

    use std::sync::Arc;
    use url::Url;

    fn init_logger() {
        env::set_var("RUST_LOG", "info");
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn serde_json_should_work() {
        let data = r#"
        {"300841": {"name": "康华生物", "open": 59.8, "close": 59.82, "now": 58.85, "high": 60.16, "low": 58.02, "buy": 58.84, "sell": 58.85, "turnover": 2136462, "volume": 126397370.69, "bid1_volume": 1800, "bid1": 58.84, "bid2_volume": 100, "bid2": 58.77, "bid3_volume": 730, "bid3": 58.76, "bid4_volume": 2400, "bid4": 58.75, "bid5_volume": 600, "bid5": 58.74, "ask1_volume": 3175, "ask1": 58.85, "ask2_volume": 1325, "ask2": 58.86, "ask3_volume": 1800, "ask3": 58.88, "ask4_volume": 625, "ask4": 58.92, "ask5_volume": 400, "ask5": 58.93, "date": "2023-07-14", "time": "15:35:00"}}
        "#;
        let v: Value = serde_json::from_str(data).expect("Parse failed");

        // Access parts of the data by indexing with square brackets.
        println!(
            "Please call {} at the number {}",
            v["300841"], v["300841"]["name"]
        );
    }

    fn random_string(len: usize) -> String {
        let mut rng = rand::thread_rng();
        std::iter::repeat(())
            .map(|()| rng.sample(rand::distributions::Alphanumeric))
            .map(char::from)
            .take(len)
            .collect()
    }

    pub fn generate_random_sync_tasks(n: u32) -> Vec<Arc<Mutex<SyncTask>>> {
        (0..n)
            .map(|_| {
                let fake_url = format!("http://{}", SafeEmail().fake::<String>());
                let request_endpoint = Url::parse(&fake_url).unwrap();
                let fake_headers: HashMap<String, String> = (0..5)
                    .map(|_| (Name().fake::<String>(), random_string(20)))
                    .collect();
                let fake_payload = Some(Arc::new(Value::String(random_string(50))));
                let fake_method = if rand::random() {
                    RequestMethod::Get
                } else {
                    RequestMethod::Post
                };
                let task_spec = TaskSpecification::new(
                    &fake_url,
                    if fake_method == RequestMethod::Get {
                        "GET"
                    } else {
                        "POST"
                    },
                    fake_headers,
                    fake_payload,
                )
                .unwrap();

                let start_time = Local::now();
                let create_time = Local::now();
                let dataset_name = Some(random_string(10));
                let datasource_name = Some(random_string(10));
                Arc::new(Mutex::new(SyncTask::new(
                    Uuid::new_v4(),
                    &dataset_name.unwrap(),
                    Uuid::new_v4(),
                    &datasource_name.unwrap(),
                    task_spec,
                    Uuid::new_v4(),
                    None,
                )))
            })
            .collect()
    }

    // #[tokio::test]
    // async fn websocket_worker_should_work() {
    //     init_logger();
    //     // let tokio_mpsc_creator = get_mq_factory::<StreamingData>(SupportedMQImpl::InMemoryTokioChannel, MQType::Mpsc);

    //     let (task_sender, mut task_receiver) = create_tokio_mpsc_channel::<StreamingData>(1000);

    //     let (mut worker_command_sender, worker_command_receiver) =
    //         create_tokio_broadcasting_channel::<SyncWorkerMessage>(500);
    //     let mut wc_receiver2 = worker_command_receiver.clone();

    //     let (error_sender, mut error_receiver) = create_tokio_mpsc_channel::<SyncWorkerError>(500);
    //     // let error_receiver2 = error_receiver.clone();
    //     let mut test_worker =
    //         WebsocketSyncWorker::new(task_sender, worker_command_receiver, error_sender);
    //     let mut test_task = Arc<Mutex<SyncTask>>::default();
    //     let spec =
    //         TaskSpecification::new("ws://localhost:8000/ws", "websocket", HashMap::new(), None)
    //             .expect("Unrecognized request method");

    //     let stop_time = chrono::Local::now() + chrono::Duration::seconds(10);
    //     test_task.set_spec(spec);
    //     println!("updated task: {:?}", test_task);

    //     let handle_task = tokio::spawn(async move {
    //         let _ = test_worker.handle(&mut test_task).await;
    //     });

    //     let stop_receive_watcher_task = tokio::spawn(async move {
    //         loop {
    //             let now = chrono::Local::now();
    //             // println!("The time is {:?}", now);

    //             if now >= stop_time {
    //                 println!("Trying to stop worker");
    //                 let result = worker_command_sender
    //                     .send(SyncWorkerMessage::StopReceiveData)
    //                     .await;
    //                 match result {
    //                     Ok(()) => {
    //                         info!("Command sent. Stop.");
    //                         worker_command_sender.close().await;
    //                         break;
    //                     }
    //                     Err(e) => {
    //                         println!("Error occurred while trying to send command: {:?}", e);
    //                         tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    //                     }
    //                 }
    //             }
    //             tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    //         }
    //     });

    //     let receiver_task = tokio::spawn(async move {
    //         loop {
    //             let err = error_receiver.try_recv();
    //             if let Ok(e) = err {
    //                 println!("Worker failed! Error: {:#?}", e);
    //                 println!("receiver_task exited.");
    //                 break;
    //             }

    //             let worker_command = wc_receiver2.try_recv();
    //             if let Ok(command) = worker_command {
    //                 info!("Received {:#?}! Stopping receiver task.", command);
    //                 wc_receiver2.close();
    //                 // task_receiver.close();
    //                 drop(task_receiver);
    //                 break;
    //             }
    //             info!("Receiving data..");
    //             let message = task_receiver.receive().await;
    //             match message {
    //                 Some(msg) => match msg {
    //                     StreamingData::Data(d) => {
    //                         println!("Received: {:#?} in receiver_task", d);
    //                     }
    //                     StreamingData::StopCommandReceived => {
    //                         println!("Worker has stopped. Receiver exit.");
    //                         break;
    //                     }
    //                 },
    //                 None => {
    //                     println!("No data received.");
    //                     task_receiver.close();
    //                     break;
    //                 }
    //             }
    //             println!("receiver going to sleep for 100ms...");
    //             tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    //         }
    //     });

    //     let _ = tokio::join!(handle_task, stop_receive_watcher_task, receiver_task);
    //     println!("Websocket test done!");
    // }

    #[test]
    fn logging_should_work() {
        init_logger();

        log::warn!("warn");
        log::info!("info");
        log::debug!("debug");
        info!("Print an info!");
        warn!("This is a warning!");
        error!("Error!");
    }

    #[tokio::test]
    async fn it_should_build_and_send_a_post_request() {
        let client = reqwest::Client::new();
        let payload = json!({
            "api_name": "stock_basic",
            "token": "a11e32e820d49141b0bcff711d6c4d66dda7e69d228ed0ac20d22750",
            "params": {
                "list_stauts": "L"
            },
            "fields": "ts_code,name,area,industry,list_date"
        });
        let request = build_request(
            &client,
            "http://api.tushare.pro",
            RequestMethod::Post,
            None,
            Some(Arc::new(payload)),
        )
        .expect("request method not supported!");
        let resp = request.send().await;
        match resp {
            Ok(resp) => {
                println!("status: {}", resp.status());
                let json: Result<Value, reqwest::Error> = resp.json().await;

                if let Ok(json) = json {
                    println!("value: {:?}", json);
                }
                return ();
            }
            Err(error) => {
                eprintln!("error: {}", error);
            }
        }
    }

    #[test]
    fn it_should_build_header_from_hashmap() {
        let hashmap: HashMap<String, String> = [
            ("Cookie", "_gh_sess=73%2FX%2F0EoXU1Tj4slthgAt%2B%2BQIdQJekXSbcXBbFfDv0erH%2BHv0oPGtZ7hfDCNpHVtEpFs%2BJcMXgCjK%2BFUG%2BrKS6v6rOAZeZF%2B0bMyia%2BNhr5HmavEbo5y8NbKY0xtXw966S%2FY9ILmNDD%2FZYSUfLX0fIcr1z7Fj5VGeEOeqXLlKtDuH8Y6Cqc%2F1kMpZ3A0uJCTKnKHmi4VmWPDNvkuDSPRNqc6DodQdUOA7w5rEzsqSn2aHjf3C1znSmGss1BNyE1jRreBpGNkjP3PaCJnyNgByOuu%2BHYFhOm3kA%3D%3D--NTzXHWgMFd29sCbV--lLZIz%2FqJxMV%2FxR0JGQEYFg%3D%3D; path=/; secure; HttpOnly; SameSite=Lax"),
            ("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let header_map = build_headers(&hashmap);
        println!("header map: {:?}", header_map);
    }

    // #[tokio::test]
    // async fn it_should_send_request() {
    //     let client = reqwest::Client::new();
    //     let mut test_worker = WebAPISyncWorker::new(client);
    //     let payload = json!({
    //         "api_name": "news",
    //         "token": "a11e32e820d49141b0bcff711d6c4d66dda7e69d228ed0ac20d22750",
    //         "params": {
    //             "start_date":"","end_date":"","src":"","limit":"","offset":""
    //         },
    //         "fields": ["datetime", "content", "title"]
    //     });

    //     let spec = TaskSpecification::new(
    //         "http://api.tushare.pro",
    //         "post",
    //         HashMap::new(),
    //         Some(Arc::new(payload)),
    //     )
    //     .unwrap();
    //     let mut test_task = Arc<Mutex<SyncTask>>::default();
    //     test_task.set_spec(spec);
    //     let _ = test_worker.handle(&mut test_task).await;

    //     println!("updated task: {:?}", test_task);
    // }
}
