// Synchronization Task 
// Defines the runtime status and task spec of a synchronization process

use chrono::prelude::*;
// use fake::{Dummy, Fake};
use uuid::Uuid;
use getset::{Getters, Setters};
use super::value_objects::task_spec::TaskSpec;


#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SyncStatus {
    Pending,
    Running,
    Finished,
    Failed
}

#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
struct SyncTask<'a> {
    id: Uuid,
    sync_plan_id: Uuid,
    datasource_id: Option<Uuid>,
    datasource_name: String,
    dataset_id: Option<Uuid>,
    dataset_name: String,
    status: SyncStatus,
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    spec: TaskSpec<'a>, // data payload and specification of the task
}