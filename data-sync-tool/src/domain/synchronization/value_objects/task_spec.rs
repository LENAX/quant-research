//! Task Specification
//! Contains the necessary data of performing data synchronization

use std::{str::FromStr, error::Error, string::ParseError};

use getset::{Getters, Setters};
use serde_json::Value;
use derivative::Derivative;
use url::Url;

use crate::domain::synchronization::custom_errors::RequestMethodParsingError;

#[derive(Derivative)]
#[derivative(Default(bound=""))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RequestMethod {
    #[derivative(Default)]
    Get,
    Post
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
            _ => Err(RequestMethodParsingError),
        }
    }
}


#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct TaskSpecification<'a> {
    request_endpoint: Url,
    request_method: RequestMethod,
    payload: Option<&'a Value>
}

impl<'a> Default for TaskSpecification<'a> {
    fn default() -> Self {
        Self {
            request_endpoint: Url::parse("http://localhost/").unwrap(),
            request_method: RequestMethod::Get,
            payload: None
        }
    }
}

impl<'a> TaskSpecification<'a> {
    pub fn new(url: &'a str, request_method: &'a str, payload: Option<&'a Value>) -> Result<Self, Box<dyn Error>>  {
        let parsed_url = Url::parse(url)?;
        let request_method = RequestMethod::from_str(request_method)?;

        return Ok(Self {
            request_endpoint: parsed_url,
            request_method,
            payload
        });
    }
}

mod test {

}