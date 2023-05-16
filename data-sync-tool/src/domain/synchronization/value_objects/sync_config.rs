use getset::{Getters, Setters};

#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct Quota {
    max_line_per_request: int,
    max_request_per_minute: int,
    daily_limit: int,
    max_concurrent_task: int
}

#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct SyncConfig {
    sync_quota: Quota
}