use derivative::Derivative;
use getset::{Getters, Setters};
use uuid::Uuid;

pub mod factory;
pub mod sync_rate_limiter;
// pub mod task_executor;
pub mod task_manager;
pub mod workers;
pub mod shared_traits;

#[derive(Derivative, Getters, Setters, Default, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct GetTaskRequest {
    sync_plan_id: Uuid,
}

impl GetTaskRequest {
    pub fn new(plan_id: Uuid) -> Self {
        Self { sync_plan_id: plan_id }
    }
}