//! Task Specification
//! Contains the necessary data of performing data synchronization

use std::{str::FromStr, error::Error, string::ParseError, collections::HashMap, hash::Hash, sync::Arc};

use getset::{Getters, Setters};
use serde_json::Value;
use derivative::Derivative;
use url::Url;

use crate::domain::synchronization::{custom_errors::RequestMethodParsingError, sync_plan::CreateTaskRequest};

#[derive(Derivative)]
#[derivative(Default(bound=""))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RequestMethod {
    #[derivative(Default)]
    Get,
    Post,
    Websocket
}

impl FromStr for RequestMethod {
    type Err = RequestMethodParsingError;

    fn from_str(input: &str) -> Result<RequestMethod, Self::Err> {
        match input {
            "Get" => Ok(RequestMethod::Get),
            "get" => Ok(RequestMethod::Get),
            "GET" => Ok(RequestMethod::Get),
            "Post" => Ok(RequestMethod::Post),
            "post" => Ok(RequestMethod::Post),
            "POST" => Ok(RequestMethod::Post),
            "Websocket" => Ok(RequestMethod::Websocket),
            "websocket" => Ok(RequestMethod::Websocket),
            "ws" => Ok(RequestMethod::Websocket),
            _ => Err(RequestMethodParsingError),
        }
    }
}


#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct TaskSpecification {
    request_endpoint: Url,
    request_method: RequestMethod,
    request_header: HashMap<String, String>,
    payload: Option<Arc<Value>>
}

impl Default for TaskSpecification {
    fn default() -> Self {
        Self {
            request_endpoint: Url::parse("http://localhost/").unwrap(),
            request_method: RequestMethod::Get,
            request_header: HashMap::new(),
            payload: None
        }
    }
}

impl From<&CreateTaskRequest> for TaskSpecification {
    fn from(value: &CreateTaskRequest) -> Self {
        Self { 
            request_endpoint: value.url().clone(), 
            request_method: value.request_method().clone(), 
            request_header: value.request_header().clone(), 
            payload: value.payload().clone()
        }
    }
}

impl TaskSpecification {
    pub fn new(url: &str, request_method: &str, request_header: HashMap<String, String>, payload: Option<Arc<Value>>) -> Result<Self, Box<dyn Error>>  {
        let parsed_url = Url::parse(url)?;
        let request_method = RequestMethod::from_str(request_method)?;

        return Ok(Self {
            request_endpoint: parsed_url,
            request_header,
            request_method,
            payload
        });
    }
}

mod test {

}