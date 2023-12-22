// Interfaces for entity repositories

use crate::domain::repository::Repository;

use super::{custom_errors::RepositoryError, sync_plan::SyncPlan, sync_task::SyncTask};
use async_trait::async_trait;
use getset::{Getters, Setters, MutGetters};
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Getters, Setters, MutGetters)]
#[getset(get, set, get_mut)]
// Define a query object for SyncPlan
pub struct SyncPlanQuery {
    // Fields to query SyncPlan, like name, date range, etc.
    name: Option<String>,
    // Other query fields...
}

#[async_trait]
pub trait SyncPlanRepository: Repository<SyncPlan, SyncPlanQuery> {
    // Additional methods specific to SyncPlan
    fn find_by_name(&self, name: &str) -> Result<Vec<SyncPlan>, Box<dyn std::error::Error>>;
    fn find_active_plans(&self) -> Result<Vec<SyncPlan>, Box<dyn std::error::Error>>;
    // More methods as needed...
}
