// Interfaces for entity repositories

use mockall::predicate::*;
use mockall::*;

use super::arg_template::ArgTemplate;

pub trait ArgGenRepository {
    fn by_id(&self, id: &str) -> Result<ArgTemplate, String>;
    fn save(&self, client: ArgTemplate);
    fn next_identity(&self) -> String;
    fn all(&self) -> Vec<ArgTemplate>;
}
