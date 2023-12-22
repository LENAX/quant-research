use async_trait::async_trait;
use uuid::Uuid;
use std::error::Error;

#[async_trait]
pub trait Repository<E, Q>: Send + Sync {
    fn create(&self, entity: E) -> Result<Uuid, Box<dyn Error>>;
    fn read(&self, query: Q) -> Result<Option<E>, Box<dyn Error>>;
    fn update(&self, entity: E) -> Result<(), Box<dyn Error>>;
    fn delete(&self, query: Q) -> Result<(), Box<dyn Error>>;
}
