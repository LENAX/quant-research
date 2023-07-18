//! Sync Rate Limiter Implementation
//!

use async_trait::async_trait;
use chrono::{Duration, Local};
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    sync::Arc,
};
use tokio::{
    sync::{Mutex, RwLock},
    time::Instant, task::JoinHandle,
};
use uuid::Uuid;

use crate::domain::synchronization::{rate_limiter::{RateLimitStatus, RateLimiter}, custom_errors::TimerError};
use derivative::Derivative;
use getset::{Getters, Setters};
use log::error;

#[derive(Debug)]
pub struct InvalidLimitError;

impl Display for InvalidLimitError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Limit must be a non-negative integer!")
    }
}

impl Error for InvalidLimitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

#[derive(Derivative, Debug, Clone, Getters, Setters)]
#[derivative(Default)]
#[getset(get = "pub")]
pub struct WebRequestRateLimiter {
    id: Uuid,
    max_minute_request: Arc<RwLock<i64>>,
    remaining_minute_requests: Arc<Mutex<i64>>,
    remaining_daily_requests: Arc<Mutex<Option<i64>>>,
    cool_down_seconds: Arc<RwLock<i64>>,
    count_down: Arc<Mutex<Option<Duration>>>,
    last_request_time: Arc<Mutex<Option<chrono::DateTime<chrono::Local>>>>,
}

#[async_trait]
impl RateLimiter for WebRequestRateLimiter {
    async fn start_countdown(&mut self, reset_timer: bool) -> Result<JoinHandle<()>, TimerError> {
        // start a timer to count the time before allowing the next request
        // count down is started on these conditions, assuming the timer is not already running:
        // 1. if the last request makes minute limit goes to zero
        // 2. the limit is not reached, but the remote reports an limit exceed error
        {
            let mut count_down_lock = self.count_down.lock().await;
            if *count_down_lock != None {
                println!("RateLimiter {}'s timer has already been started!", self.id);
            }

            if reset_timer {
                // if the timer is not activated, or the caller explicitly asks to reset the timer
                let cool_down_second_lock = self.cool_down_seconds.write().await;
                *count_down_lock = Some(Duration::seconds(*cool_down_second_lock));
                println!("In RateLimiter {}, countdown: {:?}", self.id, count_down_lock);
            }
        }
        let count_down_clone = Arc::clone(&self.count_down);
        let max_minute_request_clone = self.max_minute_request.clone();
        let remaining_minute_request_clone = self.remaining_minute_requests.clone();
        let limiter_id = self.id.clone();

        // start a timer in background to count the time before allowing the next request
        let task = tokio::spawn(async move {
            loop {
                println!("In RateLimiter {}, the clock is ticking...", limiter_id);
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                let mut count_down_lock = count_down_clone.lock().await;
                if let Some(count_down) = count_down_lock.as_mut() {
                    let updated_count_down = count_down.checked_sub(&Duration::seconds(1));

                    if let Some(updated_count_down) = updated_count_down {
                        *count_down = updated_count_down;
                        println!("In RateLimiter {}, Time left: {}", limiter_id, *count_down);
                        if *count_down <= Duration::seconds(0) {
                            println!("In RateLimiter {}, time is up! Counting down: {}", limiter_id, *count_down);
                            drop(count_down_lock); // break out of the loop when the count down is reached.
                            
                            // Then recover the limit
                            println!("try to recover the limit...");
                            let max_minute_request_lock = max_minute_request_clone.read().await;
                            println!("max_minute_request_lock acquired...");
                            let mut remaining_minute_requests_lock = remaining_minute_request_clone.lock().await;
                            println!("remaining_minute_requests_lock acquired...");
                            *remaining_minute_requests_lock = *max_minute_request_lock;
                            println!("remaining_minute resetted...");
                            // stop counting time
                            break;
                        }
                    } else {
                        error!("In RateLimiter {}, Failed to update count down!", limiter_id);
                        return;
                    }
                }
            }
            println!("Timer stopped...");
        });

        return Ok(task);
    }

    async fn can_proceed(&mut self) -> RateLimitStatus {
        // checks for daily limit
        {
            let remaining_daily_requests_lock = self.remaining_daily_requests.lock().await;

            // Max daily limit reached
            if let Some(remaining_daily_requests) = *remaining_daily_requests_lock {
                if remaining_daily_requests <= 0 {
                    println!("Maximum daily limit reached!");
                    return RateLimitStatus::RequestPerDayExceeded;
                }
            }
        }
        
        // Check whether countdown is activated
        let mut count_down_activated = true;

        // Checks whether the time is up
        // try not to block the timer for too long
        {
            let mut count_down_lock = self.count_down.lock().await;
            if let Some(countdown) = *count_down_lock {
                // count down is activated
                if countdown.num_seconds() > 0 {
                    return RateLimitStatus::RequestPerMinuteExceeded(false, countdown.num_seconds());
                }
                
                if countdown.num_seconds() <= 0{
                    println!("Countdown is over! second: {}", countdown.num_seconds());
                    // reset the count down to none and allow to send request
                    *count_down_lock = None;
                    count_down_activated = false;
                }    
            }
        }
        
        if !count_down_activated {
            println!("Count down is deactivated. Immediately allow one request!");
            // reset the remaining_minute_requests
            let mut remaining_minute_requests_lock =
                self.remaining_minute_requests.lock().await;
            let max_minute_request_lock = self.max_minute_request.read().await;
            *remaining_minute_requests_lock = *max_minute_request_lock - 1; // immediately allow one request
            
            // set last request time
            let mut last_requst_time_lock = self.last_request_time.lock().await;
            *last_requst_time_lock = Some(Local::now()); // update last_request_time
            
            // allow the request
            return RateLimitStatus::Ok(*remaining_minute_requests_lock as u64);
        }
        
        {
            println!("Countdown is not activated. Proceed as normal");
            let mut remaining_minute_requests_lock = self.remaining_minute_requests.lock().await;
            if *remaining_minute_requests_lock <= 0 {
                // no more requests are allowed. Should start waiting immediately
                let cooldown_seconds_lock = self.cool_down_seconds.read().await;
                return RateLimitStatus::RequestPerMinuteExceeded(true, *cooldown_seconds_lock);
            }

            *remaining_minute_requests_lock -= 1;
            let mut last_requst_time_lock = self.last_request_time.lock().await;
            *last_requst_time_lock = Some(Local::now()); // update last_request_time
            return RateLimitStatus::Ok(*remaining_minute_requests_lock as u64);
        }
    }
}

impl WebRequestRateLimiter {
    pub fn new(
        max_minute_request: i64,
        max_daily_request: Option<i64>,
        cool_down_seconds: Option<i64>,
    ) -> Result<WebRequestRateLimiter, InvalidLimitError> {
        if max_minute_request < 0 {
            return Err(InvalidLimitError);
        }

        if let Some(daily_limit) = max_daily_request {
            if daily_limit < 0 {
                return Err(InvalidLimitError);
            }
        }

        let mut default_cool_down_seconds: i64 = 60;

        if let Some(cooldown_sec) = cool_down_seconds {
            if cooldown_sec < 0 {
                return Err(InvalidLimitError);
            }
            default_cool_down_seconds = cooldown_sec;
        }

        Ok(WebRequestRateLimiter {
            id: uuid::Uuid::new_v4(),
            max_minute_request: Arc::new(RwLock::new(max_minute_request.try_into().unwrap())),
            remaining_minute_requests: Arc::new(Mutex::new(max_minute_request)),
            remaining_daily_requests: Arc::new(Mutex::new(max_daily_request)),
            cool_down_seconds: Arc::new(RwLock::new(default_cool_down_seconds)),
            count_down: Arc::new(Mutex::new(None)),
            last_request_time: Arc::new(Mutex::new(None)),
        })
    }
}


#[cfg(test)]
mod test{
    use std::{error::Error, sync::Arc, cell::RefCell};
    use futures::future::join_all;

    // use log::println;

    use tokio::{sync::Mutex, join, task::{JoinHandle}};

    use crate::{infrastructure::sync::sync_rate_limiter::WebRequestRateLimiter, domain::synchronization::rate_limiter::{RateLimiter, RateLimitStatus}};

    #[tokio::test]
    async fn it_should_count_remaining_time_as_expected() {
        println!("Start timer test");
        let mut web_rate_limiter = Arc::new(Mutex::new(WebRequestRateLimiter::new(
            10, None, Some(3)
        ).unwrap()));
        let limiter_clone = web_rate_limiter.clone();
        let rate_limiter_task = tokio::spawn(async move {
            for i in 0..100 {
                // tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                println!("Counting remaining time: {}", i);
                let apply_response = limiter_clone.lock().await.can_proceed().await;
                let limiter_id = limiter_clone.lock().await.id().clone().to_string();
                match apply_response {
                    RateLimitStatus::Ok(remaining_count) => {
                        println!("Rate limiter {} permits this request, and there are {} available requests left", limiter_id, remaining_count);
                    },
                    RateLimitStatus::RequestPerDayExceeded => {
                        println!("Oh no! Rate limiter {} rejects this request because daily limit is reached!", limiter_id);
                    },
                    RateLimitStatus::RequestPerMinuteExceeded(should_start_timer, remaining_seconds) => {
                        if should_start_timer {
                            let mut limiter_lock = limiter_clone.lock().await;
                            let task = limiter_lock.start_countdown(true).await;

                            if let Ok(join_handle) = task {
                                
                                // let mut handles_lock = handle_clone.lock().await;
                                let _ = join!(join_handle);
                            }
                        }
                        println!("Rate limiter {} rejects this request because minute limit is reached! Time remaining: {}", limiter_id, remaining_seconds);
                    }
                }
            }
            
        });
        // let mut handles_lock = handles.lock().await;
        let _ = join!(rate_limiter_task);
        
    }

    #[tokio::test]
    async fn it_should_run_several_limiters_concurrenly() {
        let limiters = vec![
            Arc::new(Mutex::new(WebRequestRateLimiter::new(5, None, Some(1)).unwrap())),
            Arc::new(Mutex::new(WebRequestRateLimiter::new(10, None, Some(3)).unwrap())),
            Arc::new(Mutex::new(WebRequestRateLimiter::new(15, None, Some(5)).unwrap())),
            Arc::new(Mutex::new(WebRequestRateLimiter::new(20, None, Some(7)).unwrap())),
            Arc::new(Mutex::new(WebRequestRateLimiter::new(25, None, Some(9)).unwrap())),
            Arc::new(Mutex::new(WebRequestRateLimiter::new(30, None, Some(11)).unwrap())),
        ];
        let tasks = limiters.into_iter().map(|limiter| {
            let limiter_clone = limiter.clone();
            let rate_limiter_task = tokio::spawn(async move {
                for i in 0..100 {
                    // tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    println!("Counting remaining time: {}", i);
                    let apply_response = limiter_clone.lock().await.can_proceed().await;
                    let limiter_id = limiter_clone.lock().await.id().clone().to_string();
                    match apply_response {
                        RateLimitStatus::Ok(remaining_count) => {
                            println!("Rate limiter {} permits this request, and there are {} available requests left", limiter_id, remaining_count);
                        },
                        RateLimitStatus::RequestPerDayExceeded => {
                            println!("Oh no! Rate limiter {} rejects this request because daily limit is reached!", limiter_id);
                        },
                        RateLimitStatus::RequestPerMinuteExceeded(should_start_timer, remaining_seconds) => {
                            if should_start_timer {
                                let mut limiter_lock = limiter_clone.lock().await;

                                let task = limiter_lock.start_countdown(true).await;
    
                                if let Ok(join_handle) = task {
                                    
                                    // let mut handles_lock = handle_clone.lock().await;
                                    let _ = join!(join_handle);
                                }
                            }
                            println!("Rate limiter {} rejects this request because the maximum number of request per minute is reached! Time remaining: {}", limiter_id, remaining_seconds);
                        }
                    }
                }
                
            });
            rate_limiter_task
        }).collect::<Vec<JoinHandle<_>>>();

        let _ = join_all(tasks).await;
    }
}
