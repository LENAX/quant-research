// Interfaces for entity repositories

use super::param_template::ParameterTemplate;
use mockall::predicate::*;
use mockall::*;

#[automock]
pub trait ParameterTemplateRepository {
    fn by_id(&self, id: &str) -> Result<ParameterTemplate, String>;
    fn save(&self, client: ParameterTemplate);
    fn next_identity(&self) -> String;
    fn all(&self) -> Vec<ParameterTemplate>;
}
