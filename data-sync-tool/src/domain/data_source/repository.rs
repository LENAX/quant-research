// Interfaces for entity repositories

use super::data_source::DataSource;
use mockall::predicate::*;
use mockall::*;

#[automock]
pub trait DataSourceRepository {
    fn by_id(&self, id: &str) -> Result<DataSource, String>;
    fn save(&self, client: DataSource);
    fn next_identity(&self) -> String;
    fn all(&self) -> Vec<DataSource>;
}
