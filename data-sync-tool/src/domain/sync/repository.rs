// Interfaces for entity repositories

use mockall::predicate::*;
use mockall::*;

use super::sync_plan::SyncPlan;

pub trait SyncPlanRepository {
    fn by_id(&self, id: &str) -> Result<SyncPlan, String>;
    fn save(&self, client: SyncPlan);
    fn next_identity(&self) -> String;
    fn all(&self) -> Vec<SyncPlan>;
}
