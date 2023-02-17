// Interfaces for entity repositories

use super::data_source::DataSource;
use mockall::predicate::*;
use mockall::*;

#[automock]
pub trait DataSourceRepository {
    fn by_id(&self, id: &str) -> Result<Client, String>;
    fn save(&self, client: Client);
    fn next_identity(&self) -> String;
    fn all(&self) -> Vec<Client>;
}
