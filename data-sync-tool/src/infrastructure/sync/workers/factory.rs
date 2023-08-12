use crate::infrastructure::{sync::workers::worker_traits::SyncWorkerDataMPSCSender, mq::message_bus::StaticClonableAsyncComponent};
use std::{collections::HashMap, sync::Arc};

use reqwest::{header::{HeaderMap, HeaderName, HeaderValue}, Client, RequestBuilder};
use serde_json::Value;

use crate::{domain::synchronization::value_objects::task_spec::RequestMethod, infrastructure::mq::message_bus::StaticClonableMpscMQ};

use super::{errors::RequestMethodNotSupported, worker::{WebsocketSyncWorker, WebAPISyncWorker}, worker_traits::{SyncWorkerMessageMPMCReceiver, SyncWorkerErrorMessageMPSCSender, ShortRunningWorker}};

/**
 * Factory methods, traits and builders for synchronization workers
 */

pub fn build_headers(header_map: &HashMap<String, String>) -> HeaderMap {
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

pub fn build_request(
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
    MD: SyncWorkerDataMPSCSender + StaticClonableMpscMQ,
    MW: SyncWorkerMessageMPMCReceiver + StaticClonableAsyncComponent,
    ME: SyncWorkerErrorMessageMPSCSender + StaticClonableMpscMQ,
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
