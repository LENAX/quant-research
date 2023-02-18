// Interfaces for entity repositories

use super::arg_gen::ArgGeneration;
use mockall::predicate::*;
use mockall::*;

#[automock]
pub trait ArgGenRepository {
    fn by_id(&self, id: &str) -> Result<ArgGeneration, String>;
    fn save(&self, client: ArgGeneration);
    fn next_identity(&self) -> String;
    fn all(&self) -> Vec<ArgGeneration>;
}
