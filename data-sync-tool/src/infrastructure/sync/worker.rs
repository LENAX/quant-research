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
    // async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<&mut SyncTask, Box<dyn Error>>;
    async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), Box<dyn Error>>;
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
    // async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<&mut SyncTask, Box<dyn Error>> {
    async fn handle(&mut self, sync_task: &mut SyncTask) -> Result<(), Box<dyn Error>> {
        self.state = WorkerState::Working;
        sync_task.start();
        match sync_task.spec().request_method() {
            RequestMethod::Get => {
                let mut get_request = self.http_client
                    .get(sync_task.spec().request_endpoint().to_string());
                // if sync_task.spec().request_header().len() > 0 {
                //     let mut header_map = HeaderMap::new();
                //     for (key, value) in sync_task.spec().request_header() {
                //         let header_key = String::from(*key);
                //         header_map.insert(header_key.as_str(), HeaderValue::from_str(value)?);
                //     }
                //     get_request = get_request.headers(header_map);
                // }
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
                        return Ok(());
                    },
                    Err(error) => {
                        error!("error: {}", error);
                        sync_task.set_end_time(Some(chrono::offset::Local::now()))
                                 .set_result_message(Some(error.to_string()))
                                 .failed();
                        return Err(Box::new(error));
                    }
                }
            },
            RequestMethod::Post => {
                let mut post_request = self.http_client.post(sync_task.spec().request_endpoint().to_string());
                // if sync_task.spec().request_header().len() > 0 {
                //     let mut header_map = HeaderMap::new();
                //     for (key, value) in sync_task.spec().request_header() {
                //         header_map.insert(key.clone(), HeaderValue::from_str(value)?);
                //     }
                //     post_request = post_request.headers(header_map);
                // }
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
                        return Ok(());
                    },
                    Err(error) => {
                        error!("error: {}", error);
                        sync_task.set_end_time(Some(chrono::offset::Local::now()))
                                 .set_result_message(Some(error.to_string()))
                                 .failed();
                        return Err(Box::new(error));
                    }
                }
            },
            
        }
        // return Ok(sync_task);
    }
}

impl WebAPISyncWorker {
    fn new(http_client: Client) -> WebAPISyncWorker {
        return Self { state: WorkerState::Sleeping, http_client }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{infrastructure::sync::worker::{WebAPISyncWorker, SyncWorker}, domain::synchronization::{sync_task::SyncTask, value_objects::task_spec::{TaskSpecification, RequestMethod}}};
    use serde_json::Value;
    use url::Url;
    use polars::prelude::*;
    use std::io::Cursor;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn it_should_read_json_as_df() {
        let mut file = File::open("D:\\pprojects\\quant-research\\data-sync-tool\\data\\stock_list.json").unwrap();

        // Create a DataFrame
        let df = JsonReader::new(&mut file).finish().unwrap();
        println!("df: {:?}", df);
    }

    #[tokio::test]
    async fn it_should_send_request() {
        let client = reqwest::Client::new();
        let mut test_worker = WebAPISyncWorker::new(client);
        let url = Url::parse("http://api.tushare.pro").unwrap();
        let data = r#"
        {
            "api_name": "stock_basic",
            "token": "a11e32e820d49141b0bcff711d6c4d66dda7e69d228ed0ac20d22750",
            "params": {
                "list_stauts": "L"
            },
            "fields": "ts_code,name,area,industry,list_date"
        }"#;
        let payload: Value = serde_json::from_str(data).unwrap();
        let spec = TaskSpecification::new(url.as_str(), "post", HashMap::new(), Some(&payload)).unwrap();
        let mut test_task = SyncTask::default();
        test_task.set_spec(spec);
        let result = test_worker.handle(&mut test_task).await;

        match result {
            Ok(()) => {
                let data = test_task.result();
                if let Some(data) = data {
                    if let Some(map) = data.as_object() {
                        for key in map.keys() {
                            println!("key: {}", key);
                        }
                        if let Some(data_value) = map.get("data") {
                            let data_string = serde_json::to_string(&data_value).unwrap();
                            println!("Data: {:?}", &data_string[0..120]);
                            // let reader = Cursor::new(data_string);
                            // let df = JsonReader::new(reader).finish().unwrap();
                            // println!("df: {:?}", df);
                            // if let Some(items) = data_value.get("items") {
                            //     let data_string = serde_json::to_string(&items).unwrap();
                            //     println!("Data: {:?}", &data_string[0..100]);
                            //     let reader = Cursor::new(data_string);
                            //     let df = JsonReader::new(reader).finish().unwrap();
                            //     println!("df: {:?}", df);
                            // }
                        }
                    }
                }
            },
            Err(err) => {
                eprint!("err: {}", err);
            }
        }
    }
} 