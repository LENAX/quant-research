// Interfaces for entity repositories

use super::data_source::DataSource;
use async_trait::async_trait;

#[async_trait]
pub trait DataSourceRepository: Send + Sync {
    fn by_id(&self, id: &str) -> Result<DataSource, String>;
    fn save(&self, client: DataSource);
    fn next_identity(&self) -> String;
    fn all(&self) -> Vec<DataSource>;
}
