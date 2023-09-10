use uuid::Uuid;

use crate::infrastructure::sync::{
    factory::Builder,
    shared_traits::{SyncTaskMPMCReceiver, SyncTaskMPMCSender, TaskRequestMPMCSender},
    workers::{
        websocket_worker::WebsocketSyncWorker,
        worker_traits::{SyncWorker, WorkerState},
    },
};

/**
 * Websocket Sync Worker Builders and Factory method
 */

// Factory method to create a WebsocketSyncWorker.
// This method abstracts the construction of a WebsocketSyncWorker by using the builder pattern.
// It takes in the necessary channels and returns a constructed worker.
pub fn create_websocket_worker<
    // WB is a type that implements both the Builder and WebsocketWorkerBuilder traits.
    WB: Builder + WebsocketWorkerBuilder<TRS, TTR, SDS, ES>,
    // TRS is a type that represents a sender for task requests.
    TRS: TaskRequestMPMCSender,
    // TTR is a type that represents a receiver for tasks that need to be done.
    TTR: SyncTaskMPMCReceiver,
    // SDS is a type that represents a sender for data received from a remote source.
    SDS: SyncTaskMPMCSender,
    // ES is a type that represents a sender for error messages.
    ES: SyncTaskMPMCSender,
>(
    task_request_sender: TRS, // Sender for task requests.
    todo_task_receiver: TTR,  // Receiver for tasks that need to be done.
    data_sender: SDS,         // Sender for data received from a remote source.
    error_sender: ES,         // Sender for error messages.
) -> WB::Product
where
    // This constraint ensures that the product of the builder (i.e., the constructed worker)
    // implements the SyncWorker trait.
    WB::Product: SyncWorker,
{
    // The builder pattern is used to set each attribute of the WebsocketSyncWorker.
    // Each method prefixed with "with_" sets a specific attribute.
    WB::new()
        .with_task_request_sender(task_request_sender) // Set the sender for task requests.
        .with_todo_task_receiver(todo_task_receiver) // Set the receiver for tasks that need to be done.
        .with_data_sender(data_sender) // Set the sender for data received from a remote source.
        .with_error_sender(error_sender) // Set the sender for error messages.
        .build() // Construct the WebsocketSyncWorker with the provided attributes.
}

pub trait WebsocketWorkerBuilder<TRS, TTR, SDS, ES> {
    fn with_task_request_sender(self, sender: TRS) -> Self;

    fn with_todo_task_receiver(self, receiver: TTR) -> Self;

    fn with_data_sender(self, sender: SDS) -> Self;

    fn with_error_sender(self, sender: ES) -> Self;

    fn with_assigned_sync_plan_id(self, id: Uuid) -> Self;
}

pub struct WebsocketSyncWorkerBuilder<TRS, TTR, SDS, ES> {
    id: Option<Uuid>,
    state: Option<WorkerState>,
    task_request_sender: Option<TRS>,
    todo_task_receiver: Option<TTR>,
    data_sender: Option<SDS>,
    error_sender: Option<ES>,
    assigned_sync_plan_id: Option<Uuid>,
}

impl<TRS, TTR, SDS, ES> WebsocketWorkerBuilder<TRS, TTR, SDS, ES>
    for WebsocketSyncWorkerBuilder<TRS, TTR, SDS, ES>
{
    fn with_task_request_sender(mut self, sender: TRS) -> Self {
        self.task_request_sender = Some(sender);
        self
    }

    fn with_todo_task_receiver(mut self, receiver: TTR) -> Self {
        self.todo_task_receiver = Some(receiver);
        self
    }

    fn with_data_sender(mut self, sender: SDS) -> Self {
        self.data_sender = Some(sender);
        self
    }

    fn with_error_sender(mut self, sender: ES) -> Self {
        self.error_sender = Some(sender);
        self
    }

    fn with_assigned_sync_plan_id(mut self, sync_plan_id: Uuid) -> Self {
        self.assigned_sync_plan_id = Some(sync_plan_id);
        self
    }
}

impl<TRS, TTR, SDS, ES> Builder for WebsocketSyncWorkerBuilder<TRS, TTR, SDS, ES> {
    type Product = WebsocketSyncWorker<TRS, TTR, SDS, ES>;

    fn new() -> Self {
        WebsocketSyncWorkerBuilder {
            id: Some(Uuid::new_v4()),
            state: Some(WorkerState::default()),
            task_request_sender: None,
            todo_task_receiver: None,
            data_sender: None,
            error_sender: None,
            assigned_sync_plan_id: None,
        }
    }

    fn build(self) -> Self::Product {
        WebsocketSyncWorker {
            id: self.id.expect("ID must be set"),
            state: self.state.expect("WorkerState must be set"),
            task_request_sender: self.task_request_sender,
            todo_task_receiver: self.todo_task_receiver,
            data_sender: self.data_sender,
            error_sender: self.error_sender,
            assigned_sync_plan_id: self.assigned_sync_plan_id,
        }
    }
}
