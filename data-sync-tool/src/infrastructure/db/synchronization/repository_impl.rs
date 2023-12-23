use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use std::{sync::Arc, error::Error};

use crate::domain::{synchronization::{repository::SyncPlanRepository, sync_plan::SyncPlan}, repository::Repository};
use crate::domain::synchronization::dtos::SyncPlanUpdateDTO;
use data_sync_tool::domain::synchronization::dtos::SyncPlanQueryDTO;

pub struct SyncPlanRepositoryImpl {
    db: Arc<DatabaseConnection>, // shared database connection
}


#[async_trait]
impl Repository<SyncPlan, SyncPlanQueryDTO, SyncPlanUpdateDTO> for SyncPlanRepositoryImpl {
    async fn add_one(&self, entity: SyncPlan) -> Result<Uuid, Box<dyn Error>> {
        todo!()
    }
    async fn add_all(&self, entities: Vec<SyncPlan>) -> Result<Uuid, Box<dyn Error>> {
        todo!()
    }

    // Read
    async fn find_one_or_none(&self, query: SyncPlanQueryDTO) -> Result<Option<SyncPlan>, Box<dyn Error>> {
        todo!()
    }

    async fn find_one(&self, query: SyncPlanQueryDTO) -> Result<SyncPlan, Box<dyn Error>> {
        todo!()
    }

    async fn find_many(&self, query: SyncPlanQueryDTO, page_size: usize, page_number: usize) -> Result<Vec<SyncPlan>, Box<dyn Error>> {
        todo!()
    }

    // Update
    async fn update_one(&self, entity: SyncPlanUpdateDTO) -> Result<(), Box<dyn Error>> {
        todo!()
    }
    
    async fn update_many(&self, entities: Vec<SyncPlanUpdateDTO>) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    // Delete
    async fn delete(&self, query: SyncPlanQueryDTO) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    // Implement other methods from Repository trait
}
// You may also need to implement converters between your SyncPlan, SyncPlanQueryDTO, SyncPlanUpdateDTO and the SeaORM DOs.
// #[async_trait]
// impl SyncPlanRepository for SyncPlanRepositoryImpl {
//     // Implement any additional methods specific to SyncPlanRepository or override Repository methods if necessary
// }