use std::collections::HashMap;

/// Web Request Object
/// 

use derivative::Derivative;
use getset::{Getters, Setters};
use serde_json::Value;
use url::Url;

use crate::domain::synchronization::value_objects::task_spec::{RequestMethod, TaskSpecification};

#[derive(Derivative)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Request<'a> {
    url:&'a Url,
    header: &'a HashMap<&'a str, &'a str>,
    request_method: RequestMethod,
    payload: Option<&'a Value>
}

impl<'a> From<TaskSpecification<'a>> for Request<'a> {
    fn from(value: TaskSpecification) -> Self {
        Self { 
            url: value.request_endpoint(), 
            header: value.request_header(), 
            request_method: *value.request_method(),
            payload: *value.payload() 
        }
    }
}