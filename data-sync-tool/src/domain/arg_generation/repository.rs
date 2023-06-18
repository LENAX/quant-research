// Interfaces for entity repositories

use async_trait::async_trait;
use super::template::Template;

#[async_trait]
pub trait TemplateRepository: Send + Sync {
    fn by_id(&self, id: &str) -> Result<Template, String>;
    fn save(&self, client: Template);
    fn next_identity(&self) -> String;
    fn all(&self) -> Vec<Template>;
}