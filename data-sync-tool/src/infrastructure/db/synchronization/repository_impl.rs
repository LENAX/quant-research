use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use uuid::Uuid;
use std::{sync::Arc, error::Error};

use crate::domain::{synchronization::sync_plan::SyncPlan, repository::Repository};
use crate::domain::synchronization::dtos::SyncPlanUpdateDTO;
use data_sync_tool::domain::synchronization::dtos::SyncPlanQueryDTO;

pub struct SyncPlanRepositoryImpl {
    db: Arc<DatabaseConnection>, // shared database connection
}


#[async_trait]
impl Repository<SyncPlan, SyncPlanQueryDTO, SyncPlanUpdateDTO> for SyncPlanRepositoryImpl {
    async fn add_one(&self, entity: SyncPlan) -> Result<Uuid, Box<dyn Error>> {
        // Convert SyncPlan to SeaORM ActiveModel
        let model = entity.into_active_model(); // Assuming you have a conversion method
        let res = SyncPlanEntity::insert(model).exec(&self.db).await?;
        Ok(res.last_insert_id)
    }

    async fn add_all(&self, entities: Vec<SyncPlan>) -> Result<Uuid, Box<dyn Error>> {
        // Convert all SyncPlans to SeaORM ActiveModels
        let models: Vec<_> = entities.into_iter().map(|entity| entity.into_active_model()).collect();
        let res = SyncPlanEntity::insert_many(models).exec(&self.db).await?;
        Ok(res.last_insert_id) // Or handle multiple inserts appropriately
    }

    async fn find_one_or_none(&self, query: SyncPlanQueryDTO) -> Result<Option<SyncPlan>, Box<dyn Error>> {
        // Build a query based on SyncPlanQueryDTO
        let mut select = SyncPlanEntity::find();
        
        if let Some(id) = query.id {
            select = select.filter(Column::Id.eq(id));
        }
        
        // Add more filters based on other fields of SyncPlanQueryDTO

        let sync_plan = select.one(&self.db).await?;
        Ok(sync_plan.map(Into::into)) // Convert the database entity to domain entity
    }

    async fn find_one(&self, query: SyncPlanQueryDTO) -> Result<SyncPlan, Box<dyn Error>> {
        // Similar to find_one_or_none, but expecting one result only
        let sync_plan = self.find_one_or_none(query).await?.ok_or("SyncPlan not found")?;
        Ok(sync_plan)
    }

    async fn find_many(&self, query: SyncPlanQueryDTO, page_size: usize, page_number: usize) -> Result<Vec<SyncPlan>, Box<dyn Error>> {
        // Implement pagination and query building
        todo!()
    }

    async fn update_one(&self, entity: SyncPlanUpdateDTO) -> Result<(), Box<dyn Error>> {
        // Convert SyncPlanUpdateDTO to a set of changes and apply them to the database
        let mut update = SyncPlanEntity::update_many();

        // Apply changes from SyncPlanUpdateDTO
        if let Some(name) = entity.name {
            update = update.col(Column::Name.set(name));
        }

        // ... other field updates ...

        update.filter(Column::Id.eq(entity.id.unwrap())).exec(&self.db).await?;
        Ok(())
    }

    async fn update_many(&self, entities: Vec<SyncPlanUpdateDTO>) -> Result<(), Box<dyn Error>> {
        // Update multiple records based on each SyncPlanUpdateDTO
        for entity in entities {
            self.update_one(entity).await?;
        }
        Ok(())
    }

    async fn delete(&self, query: SyncPlanQueryDTO) -> Result<(), Box<dyn Error>> {
        // Build deletion query from SyncPlanQueryDTO
        let mut delete = SyncPlanEntity::delete_many();

        if let Some(id) = query.id {
            delete = delete.filter(Column::Id.eq(id));
        }

        // ... other conditions ...

        delete.exec(&self.db).await?;
        Ok(())
    }
}
// You may also need to implement converters between your SyncPlan, SyncPlanQueryDTO, SyncPlanUpdateDTO and the SeaORM DOs.
// #[async_trait]
// impl SyncPlanRepository for SyncPlanRepositoryImpl {
//     // Implement any additional methods specific to SyncPlanRepository or override Repository methods if necessary
// }