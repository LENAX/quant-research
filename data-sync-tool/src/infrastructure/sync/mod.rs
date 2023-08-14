use derivative::Derivative;
use getset::{Getters, Setters};
use uuid::Uuid;

pub mod factory;
pub mod shared_traits;
pub mod task_executor;
pub mod task_manager;
pub mod workers;

#[derive(Derivative, Getters, Setters, Default, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct GetTaskRequest {
    sync_plan_id: Uuid,
}

impl GetTaskRequest {
    pub fn new(plan_id: Uuid) -> Self {
        Self {
            sync_plan_id: plan_id,
        }
    }
}
