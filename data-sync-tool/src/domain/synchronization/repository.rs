// Interfaces for entity repositories

use super::sync_management::SyncManagement;
use mockall::predicate::*;
use mockall::*;

#[automock]
pub trait SyncManagementRepository {
    fn by_id(&self, id: &str) -> Result<SyncManagement, String>;
    fn save(&self, client: SyncManagement);
    fn next_identity(&self) -> String;
    fn all(&self) -> Vec<SyncManagement>;
}
