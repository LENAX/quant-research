use derivative::Derivative;
use getset::{Getters, Setters};

#[derive(Derivative, Debug, PartialEq, Eq, Clone, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub")]
pub struct RateQuota {
    max_line_per_request: u32,
    max_request_per_minute: u32,
    daily_limit: u32,
    max_concurrent_task: u32,
}

#[derive(Derivative, Debug, PartialEq, Eq, Clone, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub")]
pub struct SyncConfig {
    sync_rate_quota: RateQuota,
}
