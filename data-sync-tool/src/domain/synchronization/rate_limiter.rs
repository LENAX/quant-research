/// RateLimiter Trait
/// Defines the common interface for rate limiters
use async_trait::async_trait;
use tokio::task::JoinHandle;

use super::custom_errors::TimerError;

pub enum RateLimitStatus {
    Ok(u64),
    // contains the remaining second of recovering the limit
    // field 1: should reset timer, field 2: how many seconds are left
    RequestPerMinuteExceeded(bool, i64),
    RequestPerDayExceeded,
}

#[async_trait]
pub trait RateLimiter: Sync + Send {
    async fn can_proceed(&mut self) -> RateLimitStatus;
    async fn start_countdown(&mut self, reset_timer: bool) -> Result<JoinHandle<()>, TimerError>;
}
