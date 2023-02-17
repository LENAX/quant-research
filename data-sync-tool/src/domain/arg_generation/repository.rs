// Interfaces for entity repositories

use crate::domain::entities::ArgGeneration;
use mockall::predicate::*;
use mockall::*;

#[automock]
pub trait ArgGenRepository {
    fn by_id(&self, id: &str) -> Result<Client, String>;
    fn save(&self, client: Client);
    fn next_identity(&self) -> String;
    fn all(&self) -> Vec<Client>;
}
