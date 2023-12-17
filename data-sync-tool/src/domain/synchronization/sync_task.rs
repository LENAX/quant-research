// Synchronization Task
// Defines the runtime status and task spec of a synchronization process

use chrono::prelude::*;
use log::info;
// use fake::{ Fake};
use super::{
    sync_plan::CreateTaskRequest,
    value_objects::task_spec::{RequestMethod, TaskSpecification},
};
use derivative::Derivative;
use getset::{Getters, Setters};
use serde_json::Value;
use uuid::Uuid;

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SyncStatus {
    #[derivative(Default)]
    Created,
    Pending,
    Running,
    Failed,
    Cancelled,
    Finished,
}

#[derive(Derivative, Debug, PartialEq, Eq, Clone, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub")]
pub struct SyncTask {
    id: Uuid,
    sync_plan_id: Option<Uuid>,
    datasource_id: Option<Uuid>,
    datasource_name: Option<String>,
    dataset_id: Option<Uuid>,
    dataset_name: Option<String>,
    status: SyncStatus,
    start_time: DateTime<Local>,
    end_time: Option<DateTime<Local>>,
    create_time: DateTime<Local>,
    spec: TaskSpecification, // data payload and specification of the task
    n_retry_left: usize,
    result: Option<Value>,
    result_message: Option<String>,
}

impl From<&CreateTaskRequest> for SyncTask {
    fn from(req: &CreateTaskRequest) -> Self {
        let mut new_task = Self::default();
        let task_spec: TaskSpecification = TaskSpecification::from(req);

        new_task.set_spec(task_spec);
        return new_task;
    }
}

impl SyncTask {
    pub fn new(
        dataset_id: Uuid,
        dataset_name: &str,
        datasource_id: Uuid,
        datasource_name: &str,
        task_spec: TaskSpecification,
        sync_plan_id: Uuid,
        n_retry_left: Option<usize>,
    ) -> Self {
        let mut new_task = Self::default();
        new_task
            .set_sync_plan_id(Some(sync_plan_id))
            .set_dataset_id(Some(dataset_id))
            .set_datasource_id(Some(datasource_id))
            .set_dataset_name(Some(dataset_name.to_string()))
            .set_datasource_name(Some(datasource_name.to_string()))
            .set_spec(task_spec)
            .set_status(SyncStatus::Created)
            .set_create_time(Local::now())
            .set_n_retry_left(n_retry_left.unwrap_or(10));
        return new_task;
    }

    pub fn is_long_running(&self) -> bool {
        match self.spec().request_method() {
            RequestMethod::Get => false,
            RequestMethod::Post => false,
            // RequestMethod::Websocket => true,
        }
    }

    /// Set task to pending status, waiting to be executed
    pub fn wait(&mut self) -> SyncStatus {
        self.set_status(SyncStatus::Pending);
        return self.status;
    }

    /// Set task to running status
    pub fn start(&mut self, start_time: DateTime<Local>) -> SyncStatus {
        self.set_status(SyncStatus::Running);
        self.set_start_time(start_time);
        info!("Task {} started at {}", self.id, self.start_time());
        return self.status;
    }

    /// Set task to paused status
    pub fn cancel(&mut self, end_time: DateTime<Local>) -> SyncStatus {
        self.set_status(SyncStatus::Cancelled);
        self.set_end_time(Some(end_time));
        info!("Task {} cancelled at {}", self.id, end_time);
        return self.status;
    }

    pub fn failed(&mut self, end_time: DateTime<Local>) -> SyncStatus {
        self.set_status(SyncStatus::Failed);
        self.set_end_time(Some(end_time));

        if self.n_retry_left > 0 {
            self.n_retry_left -= 1;
            info!("Task {} has {} retries left", self.id, self.n_retry_left);
        } else {
            info!("Task {} has no retry left!", self.id);
        }

        return self.status;
    }

    /// Set task to finished status
    pub fn finished(&mut self, end_time: DateTime<Local>) -> SyncStatus {
        self.set_status(SyncStatus::Finished);
        self.set_end_time(Some(end_time));
        info!("Task {} finished at {}", self.id, end_time);
        return self.status;
    }
}

#[cfg(test)]
mod test {
    use super::SyncTask;

    #[test]
    fn it_should_create_an_empty_task() {
        let task = SyncTask::default();
        println!("{:#?}", task);
    }
}
