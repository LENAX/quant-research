use std::sync::Arc;

use chrono::Duration;
use getset::{Getters, MutGetters, Setters};
use tokio::sync::{Mutex, RwLock};
use uuid::Uuid;

use crate::{
    domain::synchronization::{rate_limiter::RateLimiter, value_objects::sync_config::RateQuota},
    infrastructure::sync::{
        factory::Builder, task_manager::sync_rate_limiter::WebRequestRateLimiter,
    },
};
/**
 * Rate Limiter Factory and Builders
 */

pub fn create_rate_limiter<RLB: RateLimiterBuilder>(rate_quota: &RateQuota) -> RLB::Product
where
    RLB::Product: RateLimiter,
{
    let rate_limiter_builder = RLB::new();
    rate_limiter_builder
        .default()
        .with_max_minute_request(*rate_quota.max_request_per_minute())
        .with_remaining_daily_requests(*rate_quota.daily_limit())
        .with_cooldown_seconds(*rate_quota.cooldown_seconds())
        .build()
}

pub trait RateLimiterBuilder: Builder {
    fn with_max_minute_request(self, max_minute_request: u32) -> Self;
    fn with_remaining_daily_requests(self, remaining_munute_requests: u32) -> Self;
    fn with_cooldown_seconds(self, cooldown_seconds: u32) -> Self;
}

/// WebRequestRateLimiter Builder
#[derive(Debug, MutGetters, Getters, Setters)]
pub struct WebRequestRateLimiterBuilder {
    id: Option<Uuid>,
    max_minute_request: Option<u32>,
    remaining_minute_requests: Option<u32>,
    remaining_daily_requests: Option<u32>,
    cooldown_seconds: Option<u32>,
    count_down: Option<Duration>,
    last_request_time: Option<chrono::DateTime<chrono::Local>>,
}

impl Default for WebRequestRateLimiterBuilder {
    fn default() -> Self {
        Self {
            id: Some(Uuid::new_v4()),
            max_minute_request: Some(60),
            remaining_minute_requests: Some(60),
            remaining_daily_requests: todo!(),
            cooldown_seconds: todo!(),
            count_down: todo!(),
            last_request_time: todo!(),
        }
    }
}

impl RateLimiterBuilder for WebRequestRateLimiterBuilder {
    fn with_max_minute_request(mut self, max: u32) -> Self {
        self.max_minute_request = Some(max);
        self
    }

    fn with_remaining_daily_requests(mut self, remaining: Option<u32>) -> Self {
        self.remaining_daily_requests = remaining;
        self
    }

    fn with_cooldown_seconds(mut self, seconds: u32) -> Self {
        self.cooldown_seconds = Some(seconds);
        self
    }
}

impl Builder for WebRequestRateLimiterBuilder {
    type Product = WebRequestRateLimiter;

    fn new() -> Self {
        WebRequestRateLimiterBuilder {
            id: None,
            max_minute_request: None,
            remaining_minute_requests: None,
            remaining_daily_requests: None,
            cooldown_seconds: None,
            count_down: None,
            last_request_time: None,
        }
    }

    fn build(self) -> Self::Product {
        WebRequestRateLimiter {
            id: self.id.unwrap_or_else(Uuid::new_v4),
            max_minute_request: Arc::new(RwLock::new(self.max_minute_request.unwrap_or(60))),
            remaining_minute_requests: Arc::new(Mutex::new(self.max_minute_request.unwrap_or(60))),
            remaining_daily_requests: Arc::new(Mutex::new(Some(
                self.remaining_daily_requests.unwrap_or(1000),
            ))),
            cooldown_seconds: Arc::new(RwLock::new(self.cooldown_seconds.unwrap_or(60))),
            count_down: Arc::new(Mutex::new(self.count_down)),
            last_request_time: Arc::new(Mutex::new(self.last_request_time)),
        }
    }
}
