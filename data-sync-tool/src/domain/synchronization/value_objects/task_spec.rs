//! Task Specification
//! Contains the necessary data of performing data synchronization

use std::{
    collections::HashMap, error::Error, hash::Hash, str::FromStr, string::ParseError, sync::Arc,
};

use derivative::Derivative;
use getset::{Getters, Setters};
use reqwest::{RequestBuilder, Client, header::{HeaderMap, HeaderName, HeaderValue}, Method};
use serde_json::Value;
use url::Url;
use serde_qs;

use crate::domain::synchronization::{
    custom_errors::RequestMethodParsingError, sync_plan::CreateTaskRequest,
};

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RequestMethod {
    #[derivative(Default)]
    Get,
    Post,
    // Websocket,
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
            // "Websocket" => Ok(RequestMethod::Websocket),
            // "websocket" => Ok(RequestMethod::Websocket),
            // "ws" => Ok(RequestMethod::Websocket),
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
    payload: Option<Value>,
}

impl Default for TaskSpecification {
    fn default() -> Self {
        Self {
            request_endpoint: Url::parse("http://localhost/").unwrap(),
            request_method: RequestMethod::Get,
            request_header: HashMap::new(),
            payload: None,
        }
    }
}

impl From<&CreateTaskRequest> for TaskSpecification {
    fn from(value: &CreateTaskRequest) -> Self {
        Self {
            request_endpoint: value.url().clone(),
            request_method: value.request_method().clone(),
            request_header: value.request_header().clone(),
            payload: value.payload().clone(),
        }
    }
}

impl TaskSpecification {
    pub fn new(
        url: &str,
        request_method: &str,
        request_header: HashMap<String, String>,
        payload: Option<Value>,
    ) -> Result<Self, Box<dyn Error>> {
        let parsed_url = Url::parse(url)?;
        let request_method = RequestMethod::from_str(request_method)?;

        return Ok(Self {
            request_endpoint: parsed_url,
            request_header,
            request_method,
            payload,
        });
    }
    pub fn build_headers(&self, header_map: &HashMap<String, String>) -> HeaderMap {
        let header: HeaderMap = header_map
            .iter()
            .map(|(name, val)| {
                (
                    HeaderName::from_str(name.to_lowercase().as_str()),
                    HeaderValue::from_str(val.as_str()),
                )
            })
            .filter(|(k, v)| k.is_ok() && v.is_ok())
            .map(|(k, v)| (k.unwrap(), v.unwrap()))
            .collect();
        return header;
    }

    pub fn build_request(&self, http_client: &Client) -> Option<RequestBuilder> {
        let method = match self.request_method {
            RequestMethod::Get => Method::GET,
            RequestMethod::Post => Method::POST
        };

        let mut request_builder = http_client.request(method.clone(), self.request_endpoint.clone());

        // Add headers to the request
        for (key, value) in &self.request_header {
            request_builder = request_builder.header(key, value);
        }

        match self.request_method {
            RequestMethod::Get => {
                // If payload is present, assume it's query parameters for a GET request
                if let Some(payload) = &self.payload {
                    // Assuming payload is a map-like structure. Convert it to query parameters.
                    if let Ok(params) = serde_qs::to_string(payload) {
                        let url_with_params = format!("{}?{}", self.request_endpoint, params);
                        request_builder = http_client.request(method.clone(), Url::parse(&url_with_params).ok()?);
                        for (key, value) in &self.request_header {
                            request_builder = request_builder.header(key, value);
                        }
                    }
                }
            },
            _ => {
                // For non-GET requests, add payload as JSON
                if let Some(payload) = &self.payload {
                    request_builder = request_builder.json(payload);
                }
            }
        }

        Some(request_builder)
    }
}

mod test {}
