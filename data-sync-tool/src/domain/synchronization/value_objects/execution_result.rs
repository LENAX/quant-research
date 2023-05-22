use getset::{Getters, Setters};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct ExecutionResult<'a> {
    sync_plan_id: &'a Uuid,
    task_id: &'a Uuid,
    data: Value,
    result_message: &'a str,
}