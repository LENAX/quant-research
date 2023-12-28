use derivative::Derivative;
use getset::{Getters, Setters};
use serde::{Serialize, Deserialize};

#[derive(Derivative, Debug, Clone, Getters, Setters, Serialize, Deserialize)]
#[getset(get = "pub", set = "pub")]
pub struct RateQuota {
    max_line_per_request: u32,
    max_request_per_minute: u32,
    daily_limit: u32,
    cooldown_seconds: u32,
    max_concurrent_task: u32,
    max_retry: u32,
}


#[derive(Derivative)]
#[derivative(Default(bound = ""))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SyncMode {
    #[derivative(Default)]
    HttpAPI,
    WebsocketStreaming
}

#[derive(Derivative, Debug, Clone, Getters, Setters, Default, Serialize, Deserialize)]
#[getset(get = "pub", set = "pub")]
pub struct SyncConfig {
    pub(crate) sync_rate_quota: Option<RateQuota>,
    pub(crate) sync_mode: SyncMode,
}
