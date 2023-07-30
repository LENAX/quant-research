
/// DTOs related to the task management module
/// 
use getset::{Getters, Setters};

use crate::infrastructure::{mq::factory::{SupportedMQImpl, MQType}, sync::sync_rate_limiter::RateLimiterImpls};

#[derive(Getters, Setters, Default, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct CreateRateLimiterRequest {
    max_request: i64,
    max_daily_request: Option<i64>,
    cooldown: Option<i64>
}

#[derive(Getters, Setters, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct CreateSyncTaskQueueRequest {
    rate_limiter_impl: RateLimiterImpls,
    rate_limiter_param: Option<CreateRateLimiterRequest>,
    max_retry: Option<u32>
}

#[derive(Getters, Setters, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct CreateTaskManagerRequest {
    create_task_queue_requests: Vec<CreateSyncTaskQueueRequest>,
    use_mq_impl: SupportedMQImpl,
    task_channel_mq_type: MQType,
    task_channel_capacity: usize,
    error_channel_capcity: usize,
    error_channel_mq_type: MQType,
    failed_task_channel_capacity: usize,
    failed_task_mq_type: MQType,
}