//! Task Specification
//! Contains the necessary data of performing data synchronization

use std::str::FromStr;

use getset::{Getters, Setters};
use serde_json::Value;
use derivative::Derivative;
use url::Url;

#[derive(Derivative)]
#[derivative(Default(bound=""))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RequestMethod {
    #[derivative(Default)]
    Get,
    Post
}

impl FromStr for RequestMethod {
    type Err = ();

    fn from_str(input: &str) -> Result<RequestMethod, Self::Err> {
        match input {
            "Get" => Ok(RequestMethod::Get),
            "get" => Ok(RequestMethod::Get),
            "GET" => Ok(RequestMethod::Get),
            "Post" => Ok(RequestMethod::Post),
            "post" => Ok(RequestMethod::Post),
            "POST" => Ok(RequestMethod::Post),
            _ => Err(()),
        }
    }
}


#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct TaskSpec<'a> {
    request_endpoint: Url,
    request_method: RequestMethod,
    payload:  Option<&'a Value>
}

impl<'a> Default for TaskSpec<'a> {
    fn default() -> Self {
        Self {
            request_endpoint: Url::parse("http://localhost/").unwrap(),
            request_method: RequestMethod::Get,
            payload: None
        }
    }
}

impl<'a> TaskSpec<'a> {
    
}

mod test {

}