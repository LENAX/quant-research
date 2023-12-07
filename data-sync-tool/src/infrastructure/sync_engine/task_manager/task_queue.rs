//! Task Queue Implementation
//! 
//! 

use std::{collections::VecDeque, sync::Arc};

use getset::{Getters, Setters, MutGetters};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::domain::synchronization::sync_task::SyncTask;

#[derive(Getters, Setters, Debug, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct TaskQueue {
    sync_plan_id: Uuid,
    tasks: VecDeque<Arc<Mutex<SyncTask>>>,
}