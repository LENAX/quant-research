/// RateLimiter Trait
/// Defines the common interface for rate limiters
use async_trait::async_trait;

pub enum RateLimitStatus {
    Ok(usize),
    RequestPerMinuteExceeded,
    RequestPerDayExceeded,
    CountdownUnfinished(usize), 
}

#[async_trait]
pub trait RateLimiter: Sync + Send {
    async fn can_proceed(&mut self) -> RateLimitStatus;
}