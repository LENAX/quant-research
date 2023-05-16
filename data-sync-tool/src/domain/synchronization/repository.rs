// Interfaces for entity repositories

use super::sync_plan::SyncPlan;
use mockall::predicate::*;
use mockall::*;


pub trait SyncPlanRepository {
    fn by_id<'a>(&self, id: &str) -> Result<SyncPlan<'a>, String>;
    fn save<'a>(&self, client: SyncPlan<'a>);
    fn next_identity<'a>(&self) -> String;
    fn all<'a>(&self) -> Vec<SyncPlan<'a>>;
}
