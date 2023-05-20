/// Synchronization Domain Services
use async_trait::async_trait;
use crate::domain::synchronization::sync_plan::SyncPlan;
use std::error::Error;


#[async_trait]
pub trait SyncPlanManagementService<'a> {
    async fn create_plan() -> Result<SyncPlan<'a>, Box<dyn Error>>;
    async fn find_plan() -> Result<Vec<SyncPlan<'a>>, Box<dyn Error>>;
    async fn update_plan() -> Result<SyncPlan<'a>, Box<dyn Error>>;
    async fn delete_plan() -> Result<bool, Box<dyn Error>>;
}