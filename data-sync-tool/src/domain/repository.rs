use async_trait::async_trait;
use uuid::Uuid;
use std::error::Error;
use anyhow::Result;

#[async_trait]
pub trait Repository<E, Q, U>: Send + Sync {
    // Create
    async fn add_one(&self, entity: E) -> Result<Uuid>;
    async fn add_all(&self, entities: Vec<E>) -> Result<Uuid>;

    // Read
    async fn find_one_or_none(&self, query: Q) -> Result<Option<E>>;
    async fn find_one(&self, query: Q) -> Result<E>;
    async fn find_many(&self, query: Q, page_size: usize, page_number: usize) -> Result<Vec<E>>;

    // Update
    async fn update_one(&self, entity: U) -> Result<()>;
    async fn update_many(&self, entities: Vec<U>) -> Result<()>;

    // Delete
    async fn delete_one(&self, query: Q) -> Result<()>;
    async fn delete_many(&self, queries: Vec<Q>) -> Result<()>;
}
