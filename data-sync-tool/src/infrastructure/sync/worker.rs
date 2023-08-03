//! Synchronization Worker
//! Handles synchronization task and sends web requests to remote data vendors
//!

use async_trait::async_trait;
use derivative::Derivative;
use getset::{Getters, MutGetters, Setters};
use log::{error, info, trace, warn};
use reqwest::{Client, RequestBuilder};
use serde_json::Value;
use std::{
    borrow::Borrow,
    collections::HashMap,
    error::{self, Error},
    fmt,
    str::FromStr,
    sync::Arc,
};
use tokio::sync::{Mutex, RwLock};
use url::Url;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tungstenite::{connect, Message};
use uuid::Uuid;

use crate::{
    domain::synchronization::{
        sync_task::{SyncStatus, SyncTask},
        value_objects::task_spec::RequestMethod,
    },
    infrastructure::mq::message_bus::{
        BroadcastingMessageBusReceiver, MessageBus, MessageBusReceiver, MessageBusSender,
        StaticClonableAsyncComponent, StaticClonableMpscMQ, StaticMpscMQReceiver,
    },
};

#[derive(Derivative, Debug)]
pub struct RequestMethodNotSupported;
impl fmt::Display for RequestMethodNotSupported {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Request method is not supported for an http client!")
    }
}

impl error::Error for RequestMethodNotSupported {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncWorkerErrorMessage {
    NoDataReceived,
    BuildRequestFailed(String),
    JsonParseFailed(String),
    WebRequestFailed(String),
    WebsocketConnectionFailed(String),
    ConnectionDroppedTimeout,
    OtherError(String),
    WorkerStopped,
}

#[async_trait]
pub trait SyncWorker: Send + Sync {
    // handles sync task, then updates its states and result
    // async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<&mut SyncTask, Box<dyn Error>>;
    async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), SyncWorkerErrorMessage>;
}

/// A marker trait that marks a long running worker
pub trait LongTaskHandlingWorker {}

/// A market trait for workers handling short tasks
pub trait ShortTaskHandlingWorker {}
pub trait ShortRunningWorker: SyncWorker + ShortTaskHandlingWorker {}
pub trait LongRunningWorker: SyncWorker + LongTaskHandlingWorker {}

fn build_headers(header_map: &HashMap<String, String>) -> HeaderMap {
    let header: HeaderMap = header_map
        .iter()
        .map(|(name, val)| {
            (
                HeaderName::from_str(name.to_lowercase().as_str()),
                HeaderValue::from_str(val.as_str()),
            )
        })
        .filter(|(k, v)| k.is_ok() && v.is_ok())
        .map(|(k, v)| (k.unwrap(), v.unwrap()))
        .collect();
    return header;
}

fn build_request(
    http_client: &Client,
    url: &str,
    request_method: RequestMethod,
    headers: Option<HeaderMap>,
    params: Option<Arc<Value>>,
) -> Result<RequestBuilder, RequestMethodNotSupported> {
    match request_method {
        RequestMethod::Get => {
            let mut request = http_client.get(url);
            if let Some(params) = params {
                let new_url = format!("{}?{}", url, params);
                request = http_client.get(new_url);
            }
            if let Some(headers) = headers {
                request = request.headers(headers);
            }
            return Ok(request);
        }
        RequestMethod::Post => {
            let mut request = http_client.post(url);
            if let Some(headers) = headers {
                request = request.headers(headers);
            }
            if let Some(params) = params {
                let inner_value: &Value = params.borrow();
                request = request.json(inner_value);
            }
            return Ok(request);
        }
        RequestMethod::Websocket => return Err(RequestMethodNotSupported),
    }
}

pub fn create_websocket_sync_workers<MD, MW, ME>(
    n: usize,
    data_sender: MD,
    worker_command_receiver: MW,
    error_sender: ME,
) -> Vec<WebsocketSyncWorker<MD, MW, ME>>
where
    MD: MessageBusSender<SyncWorkerData> + StaticClonableMpscMQ + Clone,
    MW: MessageBusReceiver<SyncWorkerMessage>
        + StaticClonableAsyncComponent
        + BroadcastingMessageBusReceiver
        + Clone,
    ME: MessageBusSender<SyncWorkerErrorMessage> + StaticClonableMpscMQ + Clone,
{
    let mut workers = Vec::new();
    for _ in 0..n {
        let new_worker = WebsocketSyncWorker::new(
            data_sender.clone(),
            worker_command_receiver.clone(),
            error_sender.clone(),
        );
        workers.push(new_worker);
    }
    workers
}

pub fn create_web_api_sync_workers(n: usize) -> Vec<WebAPISyncWorker> {
    let mut workers = Vec::new();

    for _ in 0..n {
        let http_client = Client::new();
        let new_worker = WebAPISyncWorker::new(http_client);
        workers.push(new_worker);
    }
    workers
}

pub fn create_short_running_workers(n: usize) -> Vec<Box<dyn ShortRunningWorker>> {
    let mut workers: Vec<Box<dyn ShortRunningWorker>> = Vec::new();

    for _ in 0..n {
        let http_client = Client::new();
        let new_worker = WebAPISyncWorker::new(http_client);
        workers.push(Box::new(new_worker));
    }
    workers
}

#[derive(Derivative, PartialEq, Eq)]
#[derivative(Default(bound = ""))]
pub enum WorkerState {
    #[derivative(Default)]
    Sleeping,
    Working,
    Stopped,
}

#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct WebAPISyncWorker {
    state: WorkerState,
    http_client: Client,
}

/// WebAPISyncWorker is a ShortTaskHandlingWorker
impl ShortTaskHandlingWorker for WebAPISyncWorker {}
impl ShortRunningWorker for WebAPISyncWorker {}

#[async_trait]
impl SyncWorker for WebAPISyncWorker {
    async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), SyncWorkerErrorMessage> {
        self.state = WorkerState::Working;
        sync_task.start();
        let headers = build_headers(sync_task.spec().request_header());
        let build_request_result = build_request(
            &self.http_client,
            sync_task.spec().request_endpoint().as_str(),
            sync_task.spec().request_method().to_owned(),
            Some(headers),
            sync_task.spec().payload().clone(),
        );

        match build_request_result {
            Ok(request) => {
                let resp = request.send().await;

                match resp {
                    Ok(resp) => {
                        info!("status: {}", resp.status());
                        let parse_result: Result<Value, reqwest::Error> = resp.json().await;

                        match parse_result {
                            Ok(json_value) => {
                                self.state = WorkerState::Sleeping;
                                sync_task
                                    .set_result(Some(json_value))
                                    .set_end_time(Some(chrono::offset::Local::now()))
                                    .finished();
                            }
                            Err(e) => {
                                return Err(SyncWorkerErrorMessage::JsonParseFailed(e.to_string()));
                            }
                        }
                        return Ok(());
                    }
                    Err(e) => {
                        error!("error: {}", e);
                        self.state = WorkerState::Sleeping;
                        sync_task
                            .set_end_time(Some(chrono::offset::Local::now()))
                            .set_result_message(Some(e.to_string()))
                            .failed();
                        return Err(SyncWorkerErrorMessage::WebRequestFailed(e.to_string()));
                    }
                }
            }
            Err(e) => {
                return Err(SyncWorkerErrorMessage::BuildRequestFailed(e.to_string()));
            }
        }
    }
}

impl WebAPISyncWorker {
    fn new(http_client: Client) -> WebAPISyncWorker {
        return Self {
            state: WorkerState::Sleeping,
            http_client,
        };
    }
}

#[derive(Debug, Clone)]
pub enum SyncWorkerData {
    Data(Value),
    StopCommandReceived,
}

#[derive(Debug, Clone, Copy)]
pub enum SyncWorkerMessage {
    StopReceiveData,
    DataRecieverStopped,
}

#[derive(Derivative, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct WebsocketSyncWorker<MD, MW, ME> {
    id: Uuid,
    state: WorkerState,
    // send data received from remote
    data_sender: Option<MD>,
    // send and receive commands from other modules, typically when to stop receiving data
    worker_command_receiver: Option<MW>,
    // send error messages
    error_sender: Option<ME>,
}

/// WebsocketSyncWorker is a long running work
impl<MD, MW, ME> LongTaskHandlingWorker for WebsocketSyncWorker<MD, MW, ME>
where
    MD: MessageBusSender<SyncWorkerData> + StaticClonableMpscMQ,
    MW: MessageBusReceiver<SyncWorkerMessage>
        + StaticClonableAsyncComponent
        + BroadcastingMessageBusReceiver,
    ME: MessageBusReceiver<SyncWorkerErrorMessage> + StaticClonableMpscMQ,
{
}

impl<MD, MW, ME> WebsocketSyncWorker<MD, MW, ME>
where
    MD: MessageBusSender<SyncWorkerData> + StaticClonableMpscMQ,
    MW: MessageBusReceiver<SyncWorkerMessage>
        + StaticClonableAsyncComponent
        + BroadcastingMessageBusReceiver,
    ME: MessageBusSender<SyncWorkerErrorMessage> + StaticClonableMpscMQ,
{
    fn new(
        data_sender: MD,
        worker_command_receiver: MW,
        error_sender: ME,
    ) -> WebsocketSyncWorker<MD, MW, ME> {
        return Self {
            id: Uuid::new_v4(),
            state: WorkerState::Sleeping,
            data_sender: Some(data_sender),
            worker_command_receiver: Some(worker_command_receiver),
            error_sender: Some(error_sender),
        };
    }

    fn close_all_channels(&mut self) {
        info!("Closing data sender channel...");
        self.data_sender = None;
        info!("Done! Closing worker_command_receiver channel...");
        self.worker_command_receiver = None;
        info!("Done! Closing error_sender channel...");
        self.error_sender = None;
        self.state = WorkerState::Stopped;
        info!("All channel has closed.");
    }

    fn inner_data_sender(&self) -> Result<&MD, SyncWorkerErrorMessage> {
        if self.state == WorkerState::Stopped {
            return Err(SyncWorkerErrorMessage::WorkerStopped);
        }
        match self.data_sender() {
            Some(_inner_sender) => Ok(_inner_sender),
            None => Err(SyncWorkerErrorMessage::WorkerStopped),
        }
    }

    fn inner_command_receiver(&mut self) -> Result<&mut MW, SyncWorkerErrorMessage> {
        if self.state == WorkerState::Stopped {
            return Err(SyncWorkerErrorMessage::WorkerStopped);
        }
        match self.worker_command_receiver_mut() {
            Some(_inner_sender) => Ok(_inner_sender),
            None => Err(SyncWorkerErrorMessage::WorkerStopped),
        }
    }

    fn inner_error_sender(&self) -> Result<&ME, SyncWorkerErrorMessage> {
        match self.error_sender() {
            Some(_inner_sender) => Ok(_inner_sender),
            None => Err(SyncWorkerErrorMessage::WorkerStopped),
        }
    }
}

#[async_trait]
impl<MD, MW, ME> SyncWorker for WebsocketSyncWorker<MD, MW, ME>
where
    MD: MessageBusSender<SyncWorkerData> + StaticClonableMpscMQ,
    MW: MessageBusReceiver<SyncWorkerMessage>
        + StaticClonableAsyncComponent
        + BroadcastingMessageBusReceiver,
    ME: MessageBusSender<SyncWorkerErrorMessage> + StaticClonableMpscMQ,
{
    async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), SyncWorkerErrorMessage> {
        self.state = WorkerState::Working;
        sync_task.start();
        if *sync_task.spec().request_method() != RequestMethod::Websocket {
            self.state = WorkerState::Sleeping;
            return Err(SyncWorkerErrorMessage::BuildRequestFailed(String::from(
                "Request method not supported!",
            )));
        }
        let request_url = sync_task.spec().request_endpoint();
        let connect_result = connect(request_url);
        match connect_result {
            Err(e) => {
                println!("Connection failed!");
                sync_task.failed();
                // let err_channel = self.error_msg_channel.read().await;
                let _ = self
                    .inner_error_sender()?
                    .send(SyncWorkerErrorMessage::WebsocketConnectionFailed(
                        e.to_string(),
                    ))
                    .await;
                // drop(err_channel);
                self.close_all_channels();
                return Err(SyncWorkerErrorMessage::WebsocketConnectionFailed(
                    e.to_string(),
                ));
            }
            Ok((mut socket, _)) => {
                let msg_body = sync_task.spec().payload();
                if let Some(body) = msg_body {
                    let text_msg = body.to_string();
                    socket
                        .write_message(Message::Text(text_msg))
                        .expect("Failed to send message!");
                    println!("Message sent!");
                }

                // When should receive end?
                // Now we naively assume that worker should stop when no data is received.
                // TODO: Support various types of closing strategy
                // The rough idea is to have these async tasks working concurrently:
                // 1. data receiving and sending task
                // 2. worker message handling task
                // 3. error message handling task
                let mut running = true;
                while running {
                    let recv_command_result = self.inner_command_receiver()?.try_recv();
                    match recv_command_result {
                        Ok(command) => match command {
                            SyncWorkerMessage::StopReceiveData => {
                                running = false;
                                sync_task.finished();
                                info!("Sent stop receive command to receiver!");
                                println!("Stopped receiving data.")
                            }
                            data_reciever_stopped => {
                                println!(
                                    "Why send me back this message? {:?}",
                                    data_reciever_stopped
                                );
                            }
                        },
                        Err(e) => {
                            info!("No command received. Message: {:#?}", e);
                        }
                    }

                    let result = socket.read_message();
                    match result {
                        Ok(msg) => {
                            // println!("Received msg from remote: {:?}", msg);
                            if let Message::Text(s) = msg {
                                // println!("Received text response: {}",s);
                                // serde_json::
                                let parse_result: Result<Value, serde_json::Error> =
                                    serde_json::from_str(&s);
                                match parse_result {
                                    Ok(value) => {
                                        let result = self
                                            .inner_data_sender()?
                                            .send(SyncWorkerData::Data(value))
                                            .await;
                                        match result {
                                            Ok(()) => {
                                                info!("Successfully send value!");
                                            }
                                            Err(e) => {
                                                // TODO: if worker fail to send data, should it try again?
                                                // If so, on what condition? Should it retry with a limit?
                                                error!("error: {e}");
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        error!("Failed to parse text to json value");
                                        let r = self.inner_error_sender()?.try_send(
                                            SyncWorkerErrorMessage::OtherError(e.to_string()),
                                        );

                                        if let Err(send_err) = r {
                                            error!("Could not send error back! {:?}", send_err);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to read data from socket! error: {}", e);
                            let _ = self
                                .inner_error_sender()?
                                .send(SyncWorkerErrorMessage::NoDataReceived)
                                .await;
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }

                println!("Worker finished... Closing channels...");
                self.close_all_channels();
                let _ = socket.close(None);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use log::{error, info, trace, warn};
    use std::collections::HashMap;
    use uuid::Uuid;

    use env_logger;
    use env_logger::Builder;
    use log::LevelFilter;
    use std::env;
    use std::io::Write;

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
        infrastructure::{
            mq::{
                factory::{
                    create_tokio_broadcasting_channel, create_tokio_mpsc_channel, get_mq_factory,
                    MQType, SupportedMQImpl,
                },
                message_bus::MessageBus,
            },
            sync::worker::{build_headers, build_request, SyncWorker, WebAPISyncWorker},
        },
    };
    use serde_json::json;
    use serde_json::Value;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Cursor;
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

    pub fn generate_random_sync_tasks(n: u32) -> Vec<SyncTask> {
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
                SyncTask::new(
                    Uuid::new_v4(),
                    &dataset_name.unwrap(),
                    Uuid::new_v4(),
                    &datasource_name.unwrap(),
                    task_spec,
                    Uuid::new_v4(),
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn websocket_worker_should_work() {
        init_logger();
        // let tokio_mpsc_creator = get_mq_factory::<SyncWorkerData>(SupportedMQImpl::InMemoryTokioChannel, MQType::Mpsc);

        let (task_sender, mut task_receiver) = create_tokio_mpsc_channel::<SyncWorkerData>(1000);

        let (mut worker_command_sender, worker_command_receiver) =
            create_tokio_broadcasting_channel::<SyncWorkerMessage>(500);
        let mut wc_receiver2 = worker_command_receiver.clone();

        let (error_sender, mut error_receiver) =
            create_tokio_mpsc_channel::<SyncWorkerErrorMessage>(500);
        // let error_receiver2 = error_receiver.clone();
        let mut test_worker =
            WebsocketSyncWorker::new(task_sender, worker_command_receiver, error_sender);
        let mut test_task = SyncTask::default();
        let spec =
            TaskSpecification::new("ws://localhost:8000/ws", "websocket", HashMap::new(), None)
                .expect("Unrecognized request method");

        let stop_time = chrono::Local::now() + chrono::Duration::seconds(10);
        test_task.set_spec(spec);
        println!("updated task: {:?}", test_task);

        let handle_task = tokio::spawn(async move {
            let _ = test_worker.handle(&mut test_task).await;
        });

        let stop_receive_watcher_task = tokio::spawn(async move {
            loop {
                let now = chrono::Local::now();
                // println!("The time is {:?}", now);

                if now >= stop_time {
                    println!("Trying to stop worker");
                    let result = worker_command_sender
                        .send(SyncWorkerMessage::StopReceiveData)
                        .await;
                    match result {
                        Ok(()) => {
                            info!("Command sent. Stop.");
                            worker_command_sender.close().await;
                            break;
                        }
                        Err(e) => {
                            println!("Error occurred while trying to send command: {:?}", e);
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        }
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let receiver_task = tokio::spawn(async move {
            loop {
                let err = error_receiver.try_recv();
                if let Ok(e) = err {
                    println!("Worker failed! Error: {:#?}", e);
                    println!("receiver_task exited.");
                    break;
                }

                let worker_command = wc_receiver2.try_recv();
                if let Ok(command) = worker_command {
                    info!("Received {:#?}! Stopping receiver task.", command);
                    wc_receiver2.close();
                    // task_receiver.close();
                    drop(task_receiver);
                    break;
                }
                info!("Receiving data..");
                let message = task_receiver.receive().await;
                match message {
                    Some(msg) => match msg {
                        SyncWorkerData::Data(d) => {
                            println!("Received: {:#?} in receiver_task", d);
                        }
                        SyncWorkerData::StopCommandReceived => {
                            println!("Worker has stopped. Receiver exit.");
                            break;
                        }
                    },
                    None => {
                        println!("No data received.");
                        task_receiver.close();
                        break;
                    }
                }
                println!("receiver going to sleep for 100ms...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        let _ = tokio::join!(handle_task, stop_receive_watcher_task, receiver_task);
        println!("Websocket test done!");
    }

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
            ("Cookie", "_gh_sess=73%2FX%2F0EoXU1Tj4slthgAt%2B%2BQIdQJekXSbcXBbFfDv0erH%2BHv0oPGtZ7hfDCNpHVtEpFs%2BJcMXgCjK%2BFUG%2BrKS6v6rOAZeZF%2B0bMyia%2BNhr5HmavEbo5y8NbKY0xtXw966S%2FY9ILmNDD%2FZYSUfLX0fIcr1z7Fj5VGeEOeqXLlKtDuH8Y6Cqc%2F1kMpZ3A0uJCTKnKHmi4VmWPDNvkuMDPRNqc6DodQdUOA7w5rEzsqSn2aHjf3C1znSmGss1BNyE1jRreBpGNkjP3PaCJnyNgByOuu%2BHYFhOm3kA%3D%3D--NTzXHWgMFd29sCbV--lLZIz%2FqJxMV%2FxR0JGQEYFg%3D%3D; path=/; secure; HttpOnly; SameSite=Lax"),
            ("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let header_map = build_headers(&hashmap);
        println!("header map: {:?}", header_map);
    }

    #[tokio::test]
    async fn it_should_send_request() {
        let client = reqwest::Client::new();
        let mut test_worker = WebAPISyncWorker::new(client);
        let payload = json!({
            "api_name": "news",
            "token": "a11e32e820d49141b0bcff711d6c4d66dda7e69d228ed0ac20d22750",
            "params": {
                "start_date":"","end_date":"","src":"","limit":"","offset":""
            },
            "fields": ["datetime", "content", "title"]
        });

        let spec = TaskSpecification::new(
            "http://api.tushare.pro",
            "post",
            HashMap::new(),
            Some(Arc::new(payload)),
        )
        .unwrap();
        let mut test_task = SyncTask::default();
        test_task.set_spec(spec);
        let _ = test_worker.handle(&mut test_task).await;

        println!("updated task: {:?}", test_task);
    }
}
