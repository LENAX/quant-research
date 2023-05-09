//! Task Specification
//! Contains the necessary data of performing data synchronization

use getset::{Getters, Setters};
use serde_json::Value;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RequestMethod {
    Get,
    Post
}

#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct TaskSpec<'a> {
    request_endpoint: &'a str,
    request_method: RequestMethod,
    payload: Value
}