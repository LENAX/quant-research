//! Errors returned by task manager

use std::error::{self, Error};
use std::fmt;
use std::sync::Arc;
use tokio::task::JoinHandle;

use crate::domain::synchronization::custom_errors::TimerError;

pub type CooldownTimerTask = JoinHandle<()>;
pub type TimeSecondLeft = i64;

#[derive(Debug, Clone)]
pub enum TaskManagerError {
    RateLimited(Option<Arc<CooldownTimerTask>>, TimeSecondLeft), // timer task, seco
    DailyLimitExceeded,
    RateLimiterInitializationError(TimerError),
    BatchPlanInsertionFailed(String),
    QueueNotFound,
    MissingRateLimitParam,
    RateLimiterNotSet(String),
    OtherError(String),
}

impl fmt::Display for TaskManagerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl error::Error for TaskManagerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

// May need error handling
#[derive(Debug)]
pub enum QueueError {
    NothingToSend,
    SendingNotStarted(String),
    RateLimited(Option<CooldownTimerTask>, TimeSecondLeft),
    DailyLimitExceeded,
    RateLimiterError(TimerError),
    QueuePaused(String),
    QueueStopped(String),
    QueueFinished(String),
    EmptyRequestReceived(String),
    UnmatchedSyncPlanId
}
