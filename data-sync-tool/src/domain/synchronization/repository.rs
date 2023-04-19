// Interfaces for entity repositories

use super::sync_plan::SyncPlan;
use mockall::predicate::*;
use mockall::*;

#[automock]
pub trait SyncManagementRepository {
    fn by_id(&self, id: &str) -> Result<SyncPlan, String>;
    fn save(&self, client: SyncPlan);
    fn next_identity(&self) -> String;
    fn all(&self) -> Vec<SyncPlan>;
}
