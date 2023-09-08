use crate::infrastructure::{
    mq::message_bus::StaticClonableAsyncComponent,
    sync::{
        factory::Builder,
        shared_traits::{
            StreamingDataMPMCSender, SyncTaskMPMCReceiver, SyncTaskMPMCSender,
            SyncWorkerErrorMPMCSender, TaskRequestMPMCSender,
        },
        workers::worker_traits::SyncTaskStreamingDataMPSCSender,
    },
};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, RequestBuilder,
};
use serde_json::Value;
use std::{borrow::Borrow, collections::HashMap, str::FromStr, sync::Arc};
use uuid::Uuid;

use crate::{
    domain::synchronization::value_objects::task_spec::RequestMethod,
    infrastructure::mq::message_bus::StaticClonableMpscMQ,
};

use super::{
    errors::RequestMethodNotSupported,
    worker::{WebAPISyncWorker, WebsocketSyncWorker, WorkerState},
    worker_traits::{ShortRunningWorker, SyncWorkerErrorMessageMPSCSender},
};

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

pub fn create_websocket_sync_workers<TRS, TTR, SDS, ES>(
    n: usize,
    task_request_senders: Vec<TRS>,
    todo_task_receivers: Vec<TTR>,
    data_senders: Vec<SDS>,
    error_senders: Vec<ES>,
) -> Vec<WebsocketSyncWorker<TRS, TTR, SDS, ES>>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    SDS: StreamingDataMPMCSender,
    ES: SyncWorkerErrorMPMCSender,
{
    let mut workers = Vec::new();
    (0..n)
        .zip(task_request_senders.into_iter())
        .zip(todo_task_receivers.into_iter())
        .zip(data_senders.into_iter())
        .zip(error_senders)
        .for_each(
            |((((_, req_sender), task_receiver), task_sender), failed_task_sender)| {
                let worker = WebsocketSyncWorker::new(
                    req_sender,
                    task_receiver,
                    task_sender,
                    failed_task_sender,
                );
                workers.push(worker);
            },
        );
    workers
}

pub fn create_web_api_sync_workers<
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
>(
    n: usize,
    task_request_senders: Vec<TRS>,
    todo_task_receivers: Vec<TTR>,
    completed_task_senders: Vec<CTS>,
    failed_task_senders: Vec<FTS>,
) -> Vec<WebAPISyncWorker<TRS, TTR, CTS, FTS>> {
    let mut workers = vec![];
    let http_client = &Client::new();

    (0..n)
        .zip(task_request_senders.into_iter())
        .zip(todo_task_receivers.into_iter())
        .zip(completed_task_senders.into_iter())
        .zip(failed_task_senders)
        .for_each(
            |((((_, req_sender), task_receiver), task_sender), failed_task_sender)| {
                let worker = WebAPISyncWorker::new(
                    http_client.clone(),
                    req_sender,
                    task_receiver,
                    task_sender,
                    failed_task_sender,
                );
                workers.push(worker);
            },
        );
    workers
}

pub fn create_short_running_workers<
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
>(
    n: usize,
    task_request_senders: Vec<TRS>,
    todo_task_receivers: Vec<TTR>,
    completed_task_senders: Vec<CTS>,
    failed_task_senders: Vec<FTS>,
) -> Vec<Box<dyn ShortRunningWorker>> {
    let http_client = &Client::new();
    let mut workers = vec![];

    (0..n)
        .zip(task_request_senders.into_iter())
        .zip(todo_task_receivers.into_iter())
        .zip(completed_task_senders.into_iter())
        .zip(failed_task_senders)
        .for_each(
            |((((_, req_sender), task_receiver), task_sender), failed_task_sender)| {
                let worker = WebAPISyncWorker::new(
                    http_client.clone(),
                    req_sender,
                    task_receiver,
                    task_sender,
                    failed_task_sender,
                );
                workers.push(Box::new(worker) as Box<dyn ShortRunningWorker>);
            },
        );
    workers
}

// Builders

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

impl<TRS, TTR, CTS, FTS> WebAPISyncWorkerBuilder<TRS, TTR, CTS, FTS>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
{
    pub fn new() -> Self {
        WebAPISyncWorkerBuilder {
            id: Some(Uuid::new_v4()),
            state: Some(WorkerState::default()),
            http_client: None,
            task_request_sender: None,
            todo_task_receiver: None,
            completed_task_sender: None,
            failed_task_sender: None,
            assigned_sync_plan_id: None,
        }
    }

    pub fn state(mut self, state: WorkerState) -> Self {
        self.state = Some(state);
        self
    }

    pub fn http_client(mut self, client: Client) -> Self {
        self.http_client = Some(client);
        self
    }

    pub fn task_request_sender(mut self, sender: TRS) -> Self {
        self.task_request_sender = Some(sender);
        self
    }

    pub fn todo_task_receiver(mut self, receiver: TTR) -> Self {
        self.todo_task_receiver = Some(receiver);
        self
    }

    pub fn completed_task_sender(mut self, sender: CTS) -> Self {
        self.completed_task_sender = Some(sender);
        self
    }

    pub fn assigned_sync_plan_id(mut self, sync_plan_id: Uuid) -> Self {
        self.assigned_sync_plan_id = Some(sync_plan_id);
        self
    }

    pub fn build(self) -> WebAPISyncWorker<TRS, TTR, CTS, FTS> {
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

impl<TRS, TTR, CTS, FTS> Builder for WebAPISyncWorkerBuilder<TRS, TTR, CTS, FTS>
where
    TRS: TaskRequestMPMCSender,
    TTR: SyncTaskMPMCReceiver,
    CTS: SyncTaskMPMCSender,
    FTS: SyncTaskMPMCSender,
{
    type Product = WebAPISyncWorker<TRS, TTR, CTS, FTS>;

    fn new() -> Self {
        WebAPISyncWorkerBuilder::new()
    }

    fn build(self) -> Self::Product {
        self.build()
    }
}
