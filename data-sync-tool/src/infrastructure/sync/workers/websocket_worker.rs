/**
 * Implementation of WebsocketSyncWorker
 */
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
    worker_traits::{LongTaskHandlingWorker, SyncWorker, WorkerState}, factory::websocket_worker_factory::WebsocketSyncWorkerBuilder,
};

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
    type BuilderType = WebsocketSyncWorkerBuilder<TRS, TTR, SDS, ES>;

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
