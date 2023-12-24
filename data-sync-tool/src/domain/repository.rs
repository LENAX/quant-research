use async_trait::async_trait;
use uuid::Uuid;
use std::error::Error;

#[async_trait]
pub trait Repository<E, Q, U>: Send + Sync {
    // Create
    async fn add_one(&self, entity: E) -> Result<Uuid, Box<dyn Error>>;
    async fn add_all(&self, entities: Vec<E>) -> Result<Uuid, Box<dyn Error>>;

    // Read
    async fn find_one_or_none(&self, query: Q) -> Result<Option<E>, Box<dyn Error>>;
    async fn find_one(&self, query: Q) -> Result<E, Box<dyn Error>>;
    async fn find_many(&self, query: Q, page_size: usize, page_number: usize) -> Result<Vec<E>, Box<dyn Error>>;

    // Update
    async fn update_one(&self, entity: U) -> Result<(), Box<dyn Error>>;
    async fn update_many(&self, entities: Vec<U>) -> Result<(), Box<dyn Error>>;

    // Delete
    async fn delete(&self, query: Q) -> Result<(), Box<dyn Error>>;
}