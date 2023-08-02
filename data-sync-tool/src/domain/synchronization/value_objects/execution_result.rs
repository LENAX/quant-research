use getset::{Getters, Setters};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct ExecutionResult {
    sync_plan_id: Uuid,
    task_id: Uuid,
    dataset_id: Uuid,
    datasource_id: Uuid,
    data: Value,
    result_message: String,
}
