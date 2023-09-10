use std::{collections::HashMap, str::FromStr, sync::Arc, borrow::Borrow};

/**
 * WebAPISyncWorker's Builder and Factory method
 */

use reqwest::{Client, header::{HeaderMap, HeaderName, HeaderValue}, RequestBuilder};
use serde_json::Value;
use uuid::Uuid;

use crate::{infrastructure::sync::{
    factory::Builder,
    shared_traits::{SyncTaskMPMCReceiver, SyncTaskMPMCSender, TaskRequestMPMCSender},
    workers::{
        http_worker::WebAPISyncWorker,
        worker_traits::{SyncWorker, WorkerState}, errors::RequestMethodNotSupported,
    },
}, domain::synchronization::value_objects::task_spec::RequestMethod};


pub fn create_http_worker<
    WB: Builder + HttpWorkerBuilder<TRS, TTR, CTS, FTS>,
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
>(
    task_request_sender: TRS,
    todo_task_receiver: TTR,
    completed_task_sender: CTS,
    failed_task_sender: FTS,
) -> WB::Product
where
    WB::Product: SyncWorker,
{
    WB::new()
        .with_task_request_sender(task_request_sender)
        .with_todo_task_receiver(todo_task_receiver)
        .with_completed_task_sender(completed_task_sender)
        .with_failed_task_sender(failed_task_sender)
        .build()
}

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

pub trait HttpWorkerBuilder<TRS, TTR, CTS, FTS>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
{
    fn with_http_client(self, http_client: Client) -> Self;

    fn with_task_request_sender(self, sender: TRS) -> Self;

    fn with_todo_task_receiver(self, receiver: TTR) -> Self;

    fn with_completed_task_sender(self, sender: CTS) -> Self;

    fn with_failed_task_sender(self, sender: FTS) -> Self;

    fn with_assigned_sync_plan_id(self, id: Uuid) -> Self;
}

pub struct WebAPISyncWorkerBuilder<
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
> {
    id: Option<Uuid>,
    state: Option<WorkerState>,
    http_client: Option<Client>,
    task_request_sender: Option<TRS>,
    todo_task_receiver: Option<TTR>,
    completed_task_sender: Option<CTS>,
    failed_task_sender: Option<FTS>,
    assigned_sync_plan_id: Option<Uuid>,
}

impl<TRS, TTR, CTS, FTS> HttpWorkerBuilder<TRS, TTR, CTS, FTS>
    for WebAPISyncWorkerBuilder<TRS, TTR, CTS, FTS>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
{
    fn with_http_client(mut self, client: Client) -> Self {
        self.http_client = Some(client);
        self
    }

    fn with_task_request_sender(mut self, sender: TRS) -> Self {
        self.task_request_sender = Some(sender);
        self
    }

    fn with_todo_task_receiver(mut self, receiver: TTR) -> Self {
        self.todo_task_receiver = Some(receiver);
        self
    }

    fn with_completed_task_sender(mut self, sender: CTS) -> Self {
        self.completed_task_sender = Some(sender);
        self
    }

    fn with_failed_task_sender(mut self, sender: FTS) -> Self {
        self.failed_task_sender = Some(sender);
        self
    }

    fn with_assigned_sync_plan_id(mut self, sync_plan_id: Uuid) -> Self {
        self.assigned_sync_plan_id = Some(sync_plan_id);
        self
    }
}

impl<TRS, TTR, CTS, FTS> Builder for WebAPISyncWorkerBuilder<TRS, TTR, CTS, FTS>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
{
    type Product = WebAPISyncWorker<TRS, TTR, CTS, FTS>;

    fn new() -> Self {
        WebAPISyncWorkerBuilder {
            id: Some(Uuid::new_v4()),
            state: Some(WorkerState::default()),
            http_client: Some(reqwest::Client::new()),
            task_request_sender: None,
            todo_task_receiver: None,
            completed_task_sender: None,
            failed_task_sender: None,
            assigned_sync_plan_id: None,
        }
    }

    fn build(self) -> Self::Product {
        WebAPISyncWorker::new(
            self.http_client.expect("HTTP Client must be set"),
            self.task_request_sender
                .expect("Task request sender must be set"),
            self.todo_task_receiver
                .expect("Todo task receiver is required!"),
            self.completed_task_sender
                .expect("Completed task sender must be set"),
            self.failed_task_sender
                .expect("Failed task sender must be set"),
        )
    }
}
