/// Sync Task Executor Implementation
/// 

use getset::{Getters, Setters};
use serde_json::Value;
use derivative::Derivative;

use crate::domain::synchronization::rate_limiter::RateLimiter;

#[derive(Derivative, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct SyncTaskExecutor {
    rate_limiter: Box<dyn RateLimiter>
}