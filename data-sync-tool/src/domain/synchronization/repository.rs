// Interfaces for entity repositories

use crate::domain::repository::Repository;

use super::{sync_plan::{SyncPlan}, dtos::{SyncPlanQueryDTO, SyncPlanUpdateDTO}};
use async_trait::async_trait;


#[async_trait]
pub trait SyncPlanRepository: Repository<SyncPlan, SyncPlanQueryDTO, SyncPlanUpdateDTO> {}
