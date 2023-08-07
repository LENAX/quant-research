use derivative::Derivative;
use getset::{Getters, Setters};

use crate::{
    application::synchronization::dtos::task_manager::CreateRateLimiterRequest,
    infrastructure::sync::sync_rate_limiter::RateLimiterImpls,
};

#[derive(Derivative, Debug, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct RateQuota {
    max_line_per_request: u32,
    max_request_per_minute: u32,
    daily_limit: u32,
    cooldown_seconds: u32,
    max_concurrent_task: u32,
    use_impl: RateLimiterImpls,
    max_retry: u32,
}

#[derive(Derivative, Debug, Clone, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub")]
pub struct SyncConfig {
    sync_rate_quota: Option<RateQuota>,
}
