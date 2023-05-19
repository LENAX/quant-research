// Synchronization Task 
// Defines the runtime status and task spec of a synchronization process

use chrono::prelude::*;
// use fake::{ Fake};
use uuid::Uuid;
use getset::{Getters, Setters};
use super::value_objects::task_spec::TaskSpec;
use derivative::Derivative;

#[derive(Derivative)]
#[derivative(Default(bound=""))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SyncStatus {
    #[derivative(Default)]
    Created,
    Pending,
    Running,
    Finished,
    Failed
}

#[derive(Derivative)]
#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters, Default)]
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
    spec: TaskSpec<'a>, // data payload and specification of the task
    result_message: Option<String>,
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