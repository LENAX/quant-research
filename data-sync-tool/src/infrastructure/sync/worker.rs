//! Synchronization Worker
//! Handles synchronization task and sends web requests to remote data vendors
//!

use std::{collections::HashMap, error::{Error, self}, str::FromStr, sync::Arc, borrow::Borrow, fmt};
use tokio::sync::RwLock;
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

pub enum SyncWorkerMessage {
    StopReceiveData,
}

pub enum SyncWorkerErrorMessage {
    NoDataReceived,
    ConnectionDroppedTimeout,
    OtherError
}

#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct WebsocketSyncWorker {
    state: WorkerState,
    // send data received from remote
    data_channel: Arc<RwLock<dyn MessageBus<Value> + Sync + Send>>,
    // send and receive commands from other modules, typically when to stop receiving data
    worker_msg_channel: Arc<RwLock<dyn MessageBus<SyncWorkerMessage> + Sync + Send>>,
    // send error messages
    error_msg_channel: Arc<RwLock<dyn MessageBus<SyncWorkerErrorMessage> + Sync + Send>>
}

impl WebsocketSyncWorker {
    fn new(
        data_channel: Arc<RwLock<dyn MessageBus<Value> + Sync + Send>>,
        worker_msg_channel: Arc<RwLock<dyn MessageBus<SyncWorkerMessage> + Sync + Send>>,
        error_msg_channel: Arc<RwLock<dyn MessageBus<SyncWorkerErrorMessage>  + Sync + Send>>,
    ) -> WebsocketSyncWorker {
        return Self {
            state: WorkerState::Sleeping,
            data_channel,
            worker_msg_channel,
            error_msg_channel
        };
    }
}

#[async_trait]
impl SyncWorker for WebsocketSyncWorker {
    async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), Box<dyn Error>> {
        self.state = WorkerState::Working;
        if *sync_task.spec().request_method() != RequestMethod::Websocket {
            self.state = WorkerState::Sleeping;
            return Err(Box::new(RequestMethodNotSupported));
        }
        let request_url = sync_task.spec().request_endpoint();
        let (mut socket, response) =
            connect(request_url).expect("Failed to connect");
        let msg_body = sync_task.spec().payload();
        if let Some(body) = msg_body {
            let text_msg = body.to_string();
            socket.write_message(Message::Text(text_msg)).expect("Failed to send message!");
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
            let command = self.worker_msg_channel.write().await.receive().await;
            if let Ok(Some(command)) = command {
                match command {
                    SyncWorkerMessage::StopReceiveData => {
                        running = false;
                    }
                }
            }
            let result = socket.read_message();
            match result {
                Ok(msg) => {
                    if let Message::Text(s) = msg {
                        info!("Received text response: {}",s);
                        let parse_result: Result<Value, serde_json::Error> = serde_json::from_str(s.as_str());
                        match parse_result {
                            Ok(value) => {
                                let data_channel_lock = self.data_channel.read().await;
                                let _ = data_channel_lock.send(value).await.expect("send failed");
                                info!("Successfully send value {:?}", value);
                            },
                            Err(e) => {
                                error!("Failed to parse text to json value");
                                let err_channel_lock = self.error_msg_channel.read().await;
                                err_channel_lock.send(SyncWorkerErrorMessage::OtherError).await;
                            }
                        }
                    }
                    
                },
                Err(e) => {
                    error!("Failed to read data from socket! error: {}", e);
                    let err_channel_lock = self.error_msg_channel.read().await;
                    err_channel_lock.send(SyncWorkerErrorMessage::NoDataReceived).await;
                }
            }

        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use log::{info, trace, warn, error};
    use rbatis::error;
    
    use std::env;
    use env_logger;
    use std::io::Write;
    use env_logger::Builder;
    use log::LevelFilter;

    use chrono::Local;

    use crate::{
        domain::synchronization::{
            sync_task::SyncTask,
            value_objects::task_spec::{RequestMethod, TaskSpecification},
        },
        infrastructure::sync::worker::{
            build_headers, build_request, SyncWorker, WebAPISyncWorker,
        },
    };
    use serde_json::json;
    use serde_json::Value;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Cursor;
    use url::Url;
    use std::sync::Arc;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
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
