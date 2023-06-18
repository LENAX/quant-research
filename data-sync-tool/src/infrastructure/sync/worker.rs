//! Synchronization Worker
//! Handles synchronization task and sends web requests to remote data vendors
//! 

use std::error::Error;

use async_trait::async_trait;
use derivative::Derivative;
use reqwest::Client;
use getset::{Getters, Setters};
use serde_json::Value;
use url::Url;

use log::{info, error};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};

use crate::domain::synchronization::{sync_task::{SyncTask, SyncStatus}, value_objects::task_spec::RequestMethod};

#[async_trait]
pub trait SyncWorker: Send + Sync {
    // handles sync task, then updates its states and result
    async fn handle<'a>(&mut self, sync_task: &'a mut SyncTask<'a>) -> Result<&'a mut SyncTask<'a>, Box<dyn Error>>;
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
    http_client: Client
}

#[async_trait]
impl SyncWorker for WebAPISyncWorker {
    async fn handle<'a>(&mut self, sync_task: &'a mut SyncTask<'a>) -> Result<&'a mut SyncTask<'a>, Box<dyn Error>> {
        self.state = WorkerState::Working;
        sync_task.start();
        match sync_task.spec().request_method() {
            RequestMethod::Get => {
                let mut get_request = self.http_client
                    .get(sync_task.spec().request_endpoint().to_string());
                if sync_task.spec().request_header().len() > 0 {
                    let mut header_map = HeaderMap::new();
                    for (key, value) in sync_task.spec().request_header() {
                        header_map.insert(*key, HeaderValue::from_str(value)?);
                    }
                    get_request = get_request.headers(header_map);
                }
                if let Some(get_param) = *sync_task.spec().payload() {
                    let new_url = format!("{}?{}",sync_task.spec().request_endpoint().to_string(), get_param);
                    get_request = self.http_client
                                      .get(new_url);
                }

                let resp = get_request.send().await;
                
                match resp {
                    Ok(resp) => {
                        info!("status: {}", resp.status());
                        let json: Value = resp.json().await?;
                        sync_task.set_result(Some(json))
                                 .set_end_time(Some(chrono::offset::Local::now()))
                                 .finished();
                    },
                    Err(error) => {
                        error!("error: {}", error);
                        sync_task.set_end_time(Some(chrono::offset::Local::now()))
                                 .set_result_message(Some(error.to_string()))
                                 .failed();
                    }
                }
            },
            RequestMethod::Post => {
                let mut post_request = self.http_client.post(sync_task.spec().request_endpoint().to_string());
                if sync_task.spec().request_header().len() > 0 {
                    let mut header_map = HeaderMap::new();
                    for (key, value) in sync_task.spec().request_header() {
                        header_map.insert(*key, HeaderValue::from_str(value)?);
                    }
                    post_request = post_request.headers(header_map);
                }
                if let Some(get_param) = *sync_task.spec().payload() {
                    post_request = post_request.json(get_param);
                }

                let resp = post_request.send().await;
                match resp {
                    Ok(resp) => {
                        info!("status: {}", resp.status());
                        let json: Value = resp.json().await?;
                        sync_task.set_result(Some(json))
                                 .set_end_time(Some(chrono::offset::Local::now()))
                                 .finished();
                    },
                    Err(error) => {
                        error!("error: {}", error);
                        sync_task.set_end_time(Some(chrono::offset::Local::now()))
                                 .set_result_message(Some(error.to_string()))
                                 .failed();
                    }
                }
            },
            
        }
        return Ok(sync_task);
    }
}

impl WebAPISyncWorker {
    fn new(http_client: Client) -> WebAPISyncWorker {
        return Self { state: WorkerState::Sleeping, http_client }
    }
}

#[cfg(test)]
mod tests {
    use crate::{infrastructure::sync::worker::WebAPISyncWorker, domain::synchronization::{sync_task::SyncTask, value_objects::task_spec::{TaskSpecification, RequestMethod}}};
    use serde_json::Value;
    use url::Url;

    #[tokio::test]
    async fn it_should_send_request() {
        let client = reqwest::Client::new();
        let test_worker = WebAPISyncWorker::new(client);
        let url = Url::parse("http://api.tushare.pro").unwrap();
        let data = r#"
        {
            "api_name": "stock_basic",
            "token": "1c95a2ba46ee2f296c8d206a9e576fa1dc62d4ecfdb87e74b656674e",
            "params": {
                "list_stauts": "L"
            },
            "fields": "ts_code,name,area,industry,list_date"
        }"#;
        let payload: Value = serde_json::from_str(data).unwrap();
        let spec = TaskSpecification::default()
            .set_request_endpoint(url)
            .set_request_method(RequestMethod::Post)
            .set_payload(Some(&payload));
        let test_task = SyncTask::default().set_spec(spec);
    }
} 