/// Task Executor Trait
/// Defines the common interface for task execution 
use std::error::Error;

use crate::infrastructure::web::request::Request;
use super::value_objects::sync_config::RateQuota;


pub trait RateLimiter {
    fn apply_limit(&mut self, quota: &RateQuota) -> &mut Self;
    fn limit(&mut self, request: &Request) -> Result<bool, Box<dyn Error>>;
}