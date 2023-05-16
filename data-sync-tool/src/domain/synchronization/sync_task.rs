// Synchronization Task 
// Defines the runtime status and task spec of a synchronization process

use chrono::prelude::*;
// use fake::{Dummy, Fake};
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
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    spec: Option<TaskSpec<'a>>, // data payload and specification of the task
    result_message: Option<String>,
}

// impl Default for SyncTask<'_> {
//     fn default() -> Self {
//         return SyncTask{
//             id: Uuid::new_v4(),
//             sync_plan_id: None,
//             datasource_id: None,
//             datasource_name: None,
//             dataset_id: None,
//             dataset_name: None,
//             status: SyncStatus::Created,
//             start_time: None,
//             end_time: None,
//             spec: None,
//             result_message: None,
//         }
//     }
// }


#[cfg(test)]
mod test {
    use super::SyncTask;


    #[test]
    fn it_should_create_an_empty_task() {
        let task = SyncTask::default();
        println!("{:#?}", task);
    }
}