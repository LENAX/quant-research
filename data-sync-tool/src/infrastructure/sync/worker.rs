//! Synchronization Worker
//! Handles synchronization task and sends web requests to remote data vendors
//!

use std::{collections::HashMap, error::Error, str::FromStr};

use async_trait::async_trait;
use derivative::Derivative;
use getset::{Getters, Setters};
use reqwest::{Client, RequestBuilder};
use serde_json::Value;
use url::Url;

use log::{error, info};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use crate::domain::synchronization::{
    sync_task::{SyncStatus, SyncTask},
    value_objects::task_spec::RequestMethod,
};

#[async_trait]
pub trait SyncWorker: Send + Sync {
    // handles sync task, then updates its states and result
    // async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<&mut SyncTask, Box<dyn Error>>;
    async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), Box<dyn Error>>;
}

fn build_headers(header_map: &HashMap<&str, &str>) -> HeaderMap {
    let header: HeaderMap = header_map
        .iter()
        .map(|(name, val)| {
            (
                HeaderName::from_str(name.to_lowercase().as_ref()),
                HeaderValue::from_str(val.as_ref()),
            )
        })
        .filter(|(k, v)| k.is_ok() && v.is_ok())
        .map(|(k, v)| (k.unwrap(), v.unwrap()))
        .collect();
    return header;
}

fn build_request(http_client: &Client, url: &str, request_method: RequestMethod, headers: Option<HeaderMap>, params: Option<&Value>) -> RequestBuilder {
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
            return request;
        },
        RequestMethod::Post => {
            let mut request = http_client.post(url);
            if let Some(headers) = headers {
                request = request.headers(headers);
            }
            if let Some(params) = params {
                request = request.json(params);
            }
            return request;
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
    // async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<&mut SyncTask, Box<dyn Error>> {
    async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), Box<dyn Error>> {
        self.state = WorkerState::Working;
        sync_task.start();
        let headers = build_headers(sync_task.spec().request_header());
        let request = build_request(
            &self.http_client, 
            sync_task.spec().request_endpoint().as_str(),
            sync_task.spec().request_method().to_owned(),
            Some(headers),
            *sync_task.spec().payload()
        );
        let resp = request.send().await;

        match resp {
            Ok(resp) => {
                info!("status: {}", resp.status());
                let json: Value = resp.json().await?;
                sync_task
                    .set_result(Some(json))
                    .set_end_time(Some(chrono::offset::Local::now()))
                    .finished();
                return Ok(());
            }
            Err(error) => {
                error!("error: {}", error);
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        domain::synchronization::{
            sync_task::SyncTask,
            value_objects::task_spec::{RequestMethod, TaskSpecification},
        },
        infrastructure::sync::worker::{SyncWorker, WebAPISyncWorker, build_headers, build_request},
    };
    use polars::prelude::*;
    use serde_json::Value;
    use std::fs::File;
    use std::io::BufReader;
    use std::io::Cursor;
    use url::Url;
    use serde_json::json;

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
            Some(&payload),
        );
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
        let hashmap: HashMap<&str, &str> = [
            ("Cookie", "_gh_sess=73%2FX%2F0EoXU1Tj4slthgAt%2B%2BQIdQJekXSbcXBbFfDv0erH%2BHv0oPGtZ7hfDCNpHVtEpFs%2BJcMXgCjK%2BFUG%2BrKS6v6rOAZeZF%2B0bMyia%2BNhr5HmavEbo5y8NbKY0xtXw966S%2FY9ILmNDD%2FZYSUfLX0fIcr1z7Fj5VGeEOeqXLlKtDuH8Y6Cqc%2F1kMpZ3A0uJCTKnKHmi4VmWPDNvkuMDPRNqc6DodQdUOA7w5rEzsqSn2aHjf3C1znSmGss1BNyE1jRreBpGNkjP3PaCJnyNgByOuu%2BHYFhOm3kA%3D%3D--NTzXHWgMFd29sCbV--lLZIz%2FqJxMV%2FxR0JGQEYFg%3D%3D; path=/; secure; HttpOnly; SameSite=Lax"),
            ("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36")]
            .iter()
            .map(|(k, v)| (*k, *v))
            .collect();

        let header_map = build_headers(&hashmap);
        println!("header map: {:?}", header_map);
    }

    #[tokio::test]
    async fn it_should_send_request() {
        let client = reqwest::Client::new();
        let mut test_worker = WebAPISyncWorker::new(client);
        let payload = json!({
            "api_name": "stock_basic",
            "token": "a11e32e820d49141b0bcff711d6c4d66dda7e69d228ed0ac20d22750",
            "params": {
                "list_stauts": "L"
            },
            "fields": "ts_code,name,area,industry,list_date"
        });
        
        let spec = TaskSpecification::new(
            "http://api.tushare.pro",
            "post",
            HashMap::new(),
            Some(&payload)
        ).unwrap();
        let mut test_task = SyncTask::default();
        test_task.set_spec(spec);
        let _ = test_worker.handle(&mut test_task).await;

        println!("updated task: {:?}", test_task);
    }
}
