//! Synchronization Worker
//! Handles synchronization task and sends web requests to remote data vendors
//!

use std::{collections::HashMap, error::{Error, self}, str::FromStr, sync::Arc, borrow::Borrow, fmt};
use tokio::sync::{Mutex, RwLock};
use async_trait::async_trait;
use derivative::Derivative;
use getset::{Getters, Setters};
use reqwest::{Client, RequestBuilder};
use serde_json::Value;
use url::Url;
use log::{info, trace, warn, error};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use tungstenite::{connect, Message};


use crate::{domain::synchronization::{
    sync_task::{SyncStatus, SyncTask},
    value_objects::task_spec::RequestMethod,
}, infrastructure::mq::message_bus::MessageBus};

// use anyhow::Result;

#[derive(Derivative, Debug,)]
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

#[async_trait]
pub trait SyncWorker: Send + Sync {
    // handles sync task, then updates its states and result
    // async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<&mut SyncTask, Box<dyn Error>>;
    async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), Box<dyn Error>>;
}

/// A marker trait that marks a long running worker
pub trait LongRunningWorker {}

/// A market trait for workers handling short tasks
pub trait ShortTaskHandlingWorker {}

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
        RequestMethod::Websocket => {
            return Err(RequestMethodNotSupported)
        }
    }
}

pub fn create_long_running_workers<W: SyncWorker + LongRunningWorker>(n: u32) -> Vec<W> {
    todo!()
}

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub enum WorkerState {
    #[derivative(Default)]
    Sleeping = 0,
    Working = 1,
}

#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct WebAPISyncWorker {
    state: WorkerState,
    http_client: Client,
}

/// WebAPISyncWorker is a ShortTaskHandlingWorker
impl ShortTaskHandlingWorker for WebAPISyncWorker {}

#[async_trait]
impl SyncWorker for WebAPISyncWorker {
    async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), Box<dyn Error>> {
        self.state = WorkerState::Working;
        sync_task.start();
        let headers = build_headers(sync_task.spec().request_header());
        let request = build_request(
            &self.http_client,
            sync_task.spec().request_endpoint().as_str(),
            sync_task.spec().request_method().to_owned(),
            Some(headers),
            sync_task.spec().payload().clone(),
        )?;
        let resp = request.send().await;

        match resp {
            Ok(resp) => {
                info!("status: {}", resp.status());
                let json: Value = resp.json().await?;
                self.state = WorkerState::Sleeping;
                sync_task
                    .set_result(Some(json))
                    .set_end_time(Some(chrono::offset::Local::now()))
                    .finished();
                return Ok(());
            }
            Err(error) => {
                error!("error: {}", error);
                self.state = WorkerState::Sleeping;
                sync_task
                    .set_end_time(Some(chrono::offset::Local::now()))
                    .set_result_message(Some(error.to_string()))
                    .failed();
                return Err(Box::new(error));
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

#[derive(Debug)]
pub enum SyncWorkerData {
    Data(Value),
    StopCommandReceived
}

#[derive(Debug)]
pub enum SyncWorkerMessage {
    StopReceiveData,
    DataRecieverStopped
}

#[derive(Debug)]
pub enum SyncWorkerErrorMessage {
    NoDataReceived,
    WebsocketConnectionFailed(String),
    ConnectionDroppedTimeout,
    OtherError(String)
}

#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct WebsocketSyncWorker<MD, MW, ME> {
    state: WorkerState,
    // send data received from remote
    data_channel: Arc<RwLock<MD>>,
    // send and receive commands from other modules, typically when to stop receiving data
    worker_msg_channel: Arc<RwLock<MW>>,
    // send error messages
    error_msg_channel: Arc<RwLock<ME>>,
}

/// WebsocketSyncWorker is a long running work
impl<MD, MW, ME> LongRunningWorker for WebsocketSyncWorker<MD, MW, ME>
where 
    MD: MessageBus<SyncWorkerData> + std::marker::Send,
    MW: MessageBus<SyncWorkerMessage> + std::marker::Send,
    ME: MessageBus<SyncWorkerErrorMessage> + std::marker::Send + std::marker::Sync
{}

impl<MD, MW, ME> WebsocketSyncWorker<MD, MW, ME> 
where 
    MD: MessageBus<SyncWorkerData> + std::marker::Send,
    MW: MessageBus<SyncWorkerMessage> + std::marker::Send,
    ME: MessageBus<SyncWorkerErrorMessage> + std::marker::Send + std::marker::Sync
{
    fn new(
        data_channel: Arc<RwLock<MD>>,
        worker_msg_channel: Arc<RwLock<MW>>,
        error_msg_channel: Arc<RwLock<ME>>,
    ) -> WebsocketSyncWorker<MD, MW, ME> {
        return Self {
            state: WorkerState::Sleeping,
            data_channel,
            worker_msg_channel,
            error_msg_channel
        };
    }

    async fn close_all_channels(&self) {
        let mut dc_lock = self.data_channel.write().await;
        dc_lock.close().await;

        let mut wm_lock = self.worker_msg_channel.write().await;
        wm_lock.close().await;

        let mut err_msg_lock = self.error_msg_channel.write().await;
        err_msg_lock.close().await;
    }
}

/// Hint: Use python package easyquotation to fetch second level full market quotation
/// You can wrap up a python websocket service to host the data
/// note that the quote may not be that accurate, so it is not recommended to use it in backtesting

#[async_trait]
impl<MD, MW, ME> SyncWorker for WebsocketSyncWorker<MD, MW, ME> 
where 
    MD: MessageBus<SyncWorkerData> + std::marker::Send + std::marker::Sync,
    MW: MessageBus<SyncWorkerMessage> + std::marker::Send + std::marker::Sync,
    ME: MessageBus<SyncWorkerErrorMessage> + std::marker::Send + std::marker::Sync,
{
    async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), Box<dyn Error>> {
        self.state = WorkerState::Working;
        sync_task.start();
        if *sync_task.spec().request_method() != RequestMethod::Websocket {
            self.state = WorkerState::Sleeping;
            return Err(Box::new(RequestMethodNotSupported));
        }
        let request_url = sync_task.spec().request_endpoint();
        let connect_result = connect(request_url);
        match connect_result {
            Err(e) => {
                println!("Connection failed!");
                sync_task.failed();
                let err_channel = self.error_msg_channel.read().await;
                let _ = err_channel.send(SyncWorkerErrorMessage::WebsocketConnectionFailed(e.to_string())).await;
                drop(err_channel);
                self.close_all_channels().await;
                return Err(Box::new(e));
            },
            Ok((mut socket, _)) => {
                let msg_body = sync_task.spec().payload();
                if let Some(body) = msg_body {
                    let text_msg = body.to_string();
                    socket.write_message(Message::Text(text_msg)).expect("Failed to send message!");
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
                    let mut work_mq_lock = self.worker_msg_channel.write().await;
                    let command = work_mq_lock.receive().await;
                    println!("Worker acquired work mq lock! Command: {:?}", command);
                    println!("Receiving command: {:?}", command);
                    if let Some(command) = command {
                        match command {
                            SyncWorkerMessage::StopReceiveData => {
                                running = false;
                                let _ = work_mq_lock.send(SyncWorkerMessage::DataRecieverStopped).await;
                                drop(work_mq_lock);
                                let data_channel_lock = self.data_channel.read().await;
                                let _ = data_channel_lock.send(SyncWorkerData::StopCommandReceived).await.expect("send failed");
                                drop(data_channel_lock);
                                sync_task.finished();
                                info!("Sent stop receive command to receiver!");
                                println!("Worker released work mq lock! Stopped receiving data.")
                            },
                            data_reciever_stopped => {
                                println!("Why send me back this message? {:?}", data_reciever_stopped);
                            }
                        }
                    }
                    let result = socket.read_message();
                    match result {
                        Ok(msg) => {
                            // println!("Received msg from remote: {:?}", msg);
                            if let Message::Text(s) = msg {
                                // println!("Received text response: {}",s);
                                // serde_json::
                                let parse_result: Result<Value, serde_json::Error> = serde_json::from_str(&s);
                                match parse_result {
                                    Ok(value) => {
                                        let data_channel_lock = self.data_channel.read().await;
                                        let _ = data_channel_lock.send(SyncWorkerData::Data(value)).await.expect("send failed");
                                        drop(data_channel_lock);
                                        info!("Successfully send value");
                                    },
                                    Err(e) => {
                                        error!("Failed to parse text to json value");
                                        let err_channel_lock = self.error_msg_channel.read().await;
                                        let _ = err_channel_lock.send(SyncWorkerErrorMessage::OtherError(e.to_string())).await;
                                        drop(err_channel_lock);
                                    }
                                }
                            }
                            
                        },
                        Err(e) => {
                            error!("Failed to read data from socket! error: {}", e);
                            let err_channel_lock = self.error_msg_channel.read().await;
                            let _ = err_channel_lock.send(SyncWorkerErrorMessage::NoDataReceived).await;
                            drop(err_channel_lock);
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }

                println!("Worker finished... Closing channels...");
                self.close_all_channels().await;
                let _ = socket.close(None);
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use log::{info, trace, warn, error};
    use uuid::Uuid;
    
    use std::env;
    use env_logger;
    use std::io::Write;
    use env_logger::Builder;
    use log::LevelFilter;

    use chrono::Local;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::name::en::Name;
    use fake::Fake;
    use rand::Rng;

    use crate::{
        domain::synchronization::{
            sync_task::SyncTask,
            value_objects::task_spec::{RequestMethod, TaskSpecification},
        },
        infrastructure::{sync::worker::{
            build_headers, build_request, SyncWorker, WebAPISyncWorker,
        },
        mq::tokio_channel_mq::TokioMpscMessageBus,
        mq::message_bus::MessageBus,},
    };
    use serde_json::json;
    use serde_json::Value;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Cursor;
    use url::Url;
    use std::sync::Arc;
    use super::*;

    #[test]
    fn serde_json_should_work() {
        let data = r#"
        {"300841": {"name": "康华生物", "open": 59.8, "close": 59.82, "now": 58.85, "high": 60.16, "low": 58.02, "buy": 58.84, "sell": 58.85, "turnover": 2136462, "volume": 126397370.69, "bid1_volume": 1800, "bid1": 58.84, "bid2_volume": 100, "bid2": 58.77, "bid3_volume": 730, "bid3": 58.76, "bid4_volume": 2400, "bid4": 58.75, "bid5_volume": 600, "bid5": 58.74, "ask1_volume": 3175, "ask1": 58.85, "ask2_volume": 1325, "ask2": 58.86, "ask3_volume": 1800, "ask3": 58.88, "ask4_volume": 625, "ask4": 58.92, "ask5_volume": 400, "ask5": 58.93, "date": "2023-07-14", "time": "15:35:00"}}
        "#;
        let v: Value = serde_json::from_str(data).expect("Parse failed");

        // Access parts of the data by indexing with square brackets.
        println!("Please call {} at the number {}", v["300841"], v["300841"]["name"]);
    }

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
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
        (0..n).map(|_| {
            let fake_url = format!("http://{}", SafeEmail().fake::<String>());
            let request_endpoint = Url::parse(&fake_url).unwrap();
            let fake_headers: HashMap<String, String> = (0..5).map(|_| (Name().fake::<String>(), random_string(20))).collect();
            let fake_payload = Some(Arc::new(Value::String(random_string(50))));
            let fake_method = if rand::random() { RequestMethod::Get } else { RequestMethod::Post };
            let task_spec = TaskSpecification::new(&fake_url, if fake_method == RequestMethod::Get { "GET" } else { "POST" }, fake_headers, fake_payload).unwrap();

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
                Uuid::new_v4()
            )
        }).collect()
    }

    #[tokio::test]
    async fn websocket_worker_should_work() {
        let _ = env_logger::builder().is_test(true).try_init();
        warn!("!!");
        let data_channel = Arc::new(RwLock::new(TokioMpscMessageBus::<SyncWorkerData>::new(100)));
        let error_message_channel = Arc::new(RwLock::new(TokioMpscMessageBus::<SyncWorkerErrorMessage>::new(100)));
        let worker_message_channel = Arc::new(RwLock::new(TokioMpscMessageBus::<SyncWorkerMessage>::new(1000)));
        let wm_channel = worker_message_channel.clone();
        let wm_channel_clone = worker_message_channel.clone();
        let dc = data_channel.clone();
        let ec1 = error_message_channel.clone();
        let ec2 = error_message_channel.clone();
        // let ec3 = error_message_channel.clone();

        let mut test_worker = WebsocketSyncWorker::new(
            data_channel, worker_message_channel, error_message_channel);
        let mut test_task = SyncTask::default();
        let spec = TaskSpecification::new(
            "ws://localhost:8000/ws",
            "websocket",
            HashMap::new(),
            None,
        ).expect("Unrecognized request method");
        
        let stop_time = chrono::Local::now() + chrono::Duration::seconds(5);
        test_task.set_spec(spec);
        println!("updated task: {:?}", test_task);

        let handle_task = tokio::spawn(async move {
            let _ = test_worker.handle(&mut test_task).await;
        });
        
        let stop_receive_watcher_task = tokio::spawn(async move {
            loop {
                let now = chrono::Local::now();
                println!("The time is {:?}", now);

                if now >= stop_time {
                    println!("Trying to stop worker");
                    let wm_channel_lock = wm_channel.read().await;
                    println!("Acquired worker message lock in stop_receive_watcher_task!");
                    let result = wm_channel_lock.send(SyncWorkerMessage::StopReceiveData).await;
                    drop(wm_channel_lock);
                    if let Err(e) = result {
                        println!("Error occurred while trying to send command: {:?}", e);
                    }
                    
                    // Hey! Huge mistake here! You should not receive your own data without releasing the lock!
                    // Should acquire the lock again!
                    let mut wm_channel_lock = wm_channel.write().await;

                    // tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    let response = wm_channel_lock.receive().await;
                    drop(wm_channel_lock);
                    if let Some(msg) = response {
                        if let SyncWorkerMessage::DataRecieverStopped = msg {
                            println!("Instructed worker to stop. Exit.");
                            break;
                        }
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        });

        let receiver_task = tokio::spawn(async move {
            loop {
                let mut ec_lock = ec1.write().await;
                let err = ec_lock.receive().await;
                drop(ec_lock);

                if let Some(e) = err {
                    println!("Worker failed! Error: {:?}", e);
                    println!("receiver_task exited.");
                    break;
                }
                
                let mut data_channel_lock = dc.write().await;
                let message = data_channel_lock.receive().await;
                drop(data_channel_lock);
                match message {
                    Some(msg) => {
                        match msg {
                            SyncWorkerData::Data(d) => {
                                println!("Received: {:?} in receiver_task", d);
                            },
                            SyncWorkerData::StopCommandReceived => {
                                println!("Worker has stopped. Receiver exit.");
                                break;
                            }
                        }
                    },
                    None => { println!("No data received."); }
                }
                println!("receiver going to sleep for 100ms...");
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });

        let err_handle_task = tokio::spawn(async move {
            loop {
                let mut ec_lock = ec2.write().await;
                let err = ec_lock.receive().await;
                drop(ec_lock);

                if let Some(e) = err {
                    println!("Worker failed! Error: {:?}", e);
                    println!("err_handle_task exited.");
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });

        let _ = tokio::join!(
            handle_task, stop_receive_watcher_task, receiver_task
        );
        // let _ = tokio::join!(
        //     handle_task, stop_receive_watcher_task
        // );
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
        ).expect("request method not supported!");
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
