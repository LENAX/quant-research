/// DTOs related to the task management module
///
use getset::{Getters, Setters};
use uuid::Uuid;

use crate::{
    domain::synchronization::value_objects::sync_config::{RateQuota, RateLimiterImpls},
    infrastructure::mq::factory::{MQType, SupportedMQImpl},
};

#[derive(Getters, Setters, Default, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct CreateRateLimiterRequest {
    max_request: u32,
    max_daily_request: Option<u32>,
    cooldown: Option<u32>,
}

impl From<&RateQuota> for CreateRateLimiterRequest {
    fn from(item: &RateQuota) -> Self {
        CreateRateLimiterRequest {
            max_request: *item.max_request_per_minute(),
            max_daily_request: Some(*item.daily_limit()),
            cooldown: Some(*item.cooldown_seconds()),
        }
    }
}

#[derive(Getters, Setters, Clone)]
#[getset(get = "pub", set = "pub")]
pub struct CreateSyncTaskQueueRequest {
    dataset_id: Uuid,
    sync_plan_id: Uuid,
    rate_limiter_impl: RateLimiterImpls,
    rate_limiter_param: Option<CreateRateLimiterRequest>,
    max_retry: Option<u32>,
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
