// Synchronization Task
// Defines the runtime status and task spec of a synchronization process

use chrono::prelude::*;
// use fake::{ Fake};
use super::value_objects::task_spec::TaskSpecification;
use derivative::Derivative;
use getset::{Getters, Setters};
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
pub struct SyncTask<'a> {
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
    spec: TaskSpecification<'a>, // data payload and specification of the task
    result_message: Option<String>,
}

impl<'a> SyncTask<'a> {
    pub fn new(
        dataset_id: Uuid,
        dataset_name: &str,
        datasource_id: Uuid,
        datasource_name: &str,
        task_spec: TaskSpecification<'a>,
        sync_plan_id: Uuid,
    ) -> Self {
        let mut new_task = Self::default();
        new_task.set_sync_plan_id(Some(sync_plan_id))
                .set_dataset_id(Some(dataset_id))
                .set_datasource_id(Some(datasource_id))
                .set_dataset_name(Some(dataset_name.to_string()))
                .set_datasource_name(Some(datasource_name.to_string()))
                .set_spec(task_spec)
                .set_create_time(Local::now());
        return new_task;
    }

    /// Set task to pending status, waiting to be executed
    pub fn wait(&mut self) -> SyncStatus {
        self.set_status(SyncStatus::Pending);
        return self.status;
    }

    /// Set task to running status
    pub fn start(&mut self) -> SyncStatus {
        self.set_status(SyncStatus::Running);
        return self.status;
    }

    /// Set task to paused status
    pub fn cancel(&mut self) -> SyncStatus {
        self.set_status(SyncStatus::Cancelled);
        return self.status;
    }

    /// Set task to finished status
    pub fn finished(&mut self) -> SyncStatus {
        self.set_status(SyncStatus::Finished);
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
