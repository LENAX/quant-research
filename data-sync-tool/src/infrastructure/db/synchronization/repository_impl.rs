use anyhow::Ok;
use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DeleteResult, Set, TransactionTrait};
use std::{error::Error, sync::Arc};
use uuid::Uuid;

use crate::domain::synchronization::dtos::SyncPlanUpdateDTO;
use crate::domain::{repository::Repository, synchronization::sync_plan::SyncPlan};
use data_sync_tool::domain::synchronization::dtos::SyncPlanQueryDTO;

use super::sync_plan_do::{self, ActiveModel, Column};
use anyhow::Result;
use sea_orm::prelude::*;

pub struct SyncPlanRepositoryImpl {
    db: Arc<DatabaseConnection>, // shared database connection
}

#[async_trait]
impl Repository<SyncPlan, SyncPlanQueryDTO, SyncPlanUpdateDTO> for SyncPlanRepositoryImpl {
    async fn add_one(&self, entity: SyncPlan) -> Result<Uuid> {
        // Convert SyncPlan to SeaORM ActiveModel
        let model: ActiveModel = entity.into(); // Assuming you have a conversion method
        let res = model.insert(&*self.db).await?;
        println!("{:?}", res);
        Ok(res.id)
    }

    async fn add_all(&self, entities: Vec<SyncPlan>) -> Result<Uuid> {
        // Convert all SyncPlans to SeaORM ActiveModels
        let models: Vec<ActiveModel> = entities.into_iter().map(|entity| entity.into()).collect();
        let res = sync_plan_do::Entity::insert_many(models)
            .exec(&*self.db)
            .await?;
        Ok(res.last_insert_id) // Or handle multiple inserts appropriately
    }

    async fn find_one_or_none(&self, query: SyncPlanQueryDTO) -> Result<Option<SyncPlan>> {
        let mut select = sync_plan_do::Entity::find();

        if let Some(id) = query.id() {
            select = select.filter(sync_plan_do::Column::Id.eq(*id));
        }

        if let Some(name) = query.name() {
            select = select.filter(sync_plan_do::Column::Name.eq(name));
        }

        if let Some(description) = query.description() {
            select = select.filter(sync_plan_do::Column::Description.eq(description));
        }

        if let Some(trigger_time) = query.trigger_time() {
            select = select.filter(sync_plan_do::Column::TriggerTime.eq(*trigger_time));
        }

        if let Some(frequency) = query.frequency() {
            select = select.filter(sync_plan_do::Column::Frequency.eq(frequency.to_string()));
        }

        if let Some(active) = query.active() {
            select = select.filter(sync_plan_do::Column::Active.eq(*active));
        }

        if let Some(datasource_id) = query.datasource_id() {
            select = select.filter(sync_plan_do::Column::DatasourceId.eq(*datasource_id));
        }

        if let Some(dataset_id) = query.dataset_id() {
            select = select.filter(sync_plan_do::Column::DatasetId.eq(*dataset_id));
        }

        if let Some(param_template_id) = query.param_template_id() {
            select = select.filter(sync_plan_do::Column::ParamTemplateId.eq(*param_template_id));
        }

        // ... Add more filters based on other fields of SyncPlanQueryDTO if necessary ...

        let sync_plan = select.one(&*self.db).await?;
        Ok(sync_plan.map(Into::into)) // Convert the database entity to domain entity, assuming you have appropriate conversion implemented
    }

    async fn find_one(&self, query: SyncPlanQueryDTO) -> Result<SyncPlan> {
        // Similar to find_one_or_none, but expecting one result only
        let sync_plan = self.find_one_or_none(query).await?;

        match sync_plan {
            Some(sync_plan) => Ok(sync_plan),
            None => Err(anyhow::anyhow!("Plan not found")),
        }
    }

    async fn find_many(
        &self,
        query: SyncPlanQueryDTO,
        page_size: usize,
        page_number: usize,
    ) -> Result<Vec<SyncPlan>> {
        let mut select = sync_plan_do::Entity::find();

        // Apply filters based on SyncPlanQueryDTO fields
        if let Some(id) = query.id() {
            select = select.filter(sync_plan_do::Column::Id.eq(*id));
        }
        if let Some(name) = query.name() {
            select = select.filter(sync_plan_do::Column::Name.eq(name));
        }
        // ... continue for other fields ...

        // Apply pagination
        let paginator = select.paginate(&*self.db, page_size.try_into().unwrap());
        let result_page = paginator
            .fetch_page((page_number - 1).try_into().unwrap())
            .await?; // Page number is usually 1-indexed in user interfaces

        // Map database entities (Model) to domain entities (SyncPlan)
        let sync_plans: Vec<SyncPlan> = result_page
            .into_iter()
            .map(|model| model.into()) // Ensure you have implemented From<Model> for SyncPlan or a similar conversion
            .collect();

        Ok(sync_plans)
    }

    async fn update_one(&self, entity: SyncPlanUpdateDTO) -> Result<()> {
        // Ensure the ID is present
        let plan_id = match entity.id() {
            Some(id) => id,
            None => return Err(anyhow::anyhow!("SyncPlan ID is required for update")),
        };

        // Start building the update statement
        let mut update = sync_plan_do::Entity::update_many();

        // Apply changes for each field in the DTO
        if let Some(name) = entity.name() {
            update = update.col_expr(sync_plan_do::Column::Name, Expr::value(name));
        }
        if let Some(description) = entity.description() {
            update = update.col_expr(sync_plan_do::Column::Description, Expr::value(description));
        }
        // Continue for other fields...
        if let Some(trigger_time) = entity.trigger_time() {
            update = update.col_expr(
                sync_plan_do::Column::TriggerTime,
                Expr::value(*trigger_time),
            );
        }
        if let Some(frequency) = entity.frequency() {
            // Assuming frequency is converted to a string for the database
            update = update.col_expr(
                sync_plan_do::Column::Frequency,
                Expr::value(frequency.to_string()),
            );
        }
        if let Some(active) = entity.active() {
            update = update.col_expr(sync_plan_do::Column::Active, Expr::value(*active));
        }
        if let Some(sync_config) = entity.sync_config() {
            // Assuming sync_config is serialized to a string for the database
            update = update.col_expr(
                sync_plan_do::Column::SyncConfig,
                Expr::value(serde_json::to_string(&sync_config)?),
            );
        }
        // ... other fields ...

        // Apply the update statement to the database
        update
            .filter(sync_plan_do::Column::Id.eq(*plan_id))
            .exec(&*self.db)
            .await?;

        Ok(())
    }

    async fn update_many(&self, entities: Vec<SyncPlanUpdateDTO>) -> Result<()> {
        // Start a transaction
        let txn = self.db.begin().await.map_err(|err| Box::new(err))?;

        for entity in entities.iter() {
            let mut active_model: sync_plan_do::ActiveModel =
                <sync_plan_do::ActiveModel as sea_orm::ActiveModelTrait>::default();

            // Assuming SyncPlanUpdateDTO's fields are all Option types
            if let Some(id) = entity.id() {
                active_model.id = Set(*id);
            }
            if let Some(name) = &entity.name() {
                active_model.name = Set(name.clone());
            }
            if let Some(description) = entity.description() {
                active_model.description = Set(description.to_string());
            }
            // Continue for other fields...
            if let Some(trigger_time) = entity.trigger_time() {
                active_model.trigger_time = Set(Some(*trigger_time));
            }
            if let Some(frequency) = entity.frequency() {
                // Assuming frequency is converted to a string for the database
                active_model.frequency = Set(frequency.to_string());
            }
            if let Some(active) = entity.active() {
                active_model.active = Set(*active);
            }
            if let Some(sync_config) = entity.sync_config() {
                // Assuming sync_config is serialized to a string for the database
                let config_string = serde_json::to_string(sync_config)?;
                active_model.sync_config = Set(config_string);
            }
            // ... continue setting other fields ...

            // Apply changes within the transaction
            let res = active_model.update(&txn).await;
            if res.is_err() {
                // If any update fails, rollback the transaction and return an error
                let _ = txn
                    .rollback()
                    .await
                    .map_err(|err| Box::new(anyhow::anyhow!(err)));
                return Err(Box::new(res.err().unwrap()).into());
            }
        }

        // If all updates succeed, commit the transaction
        let _ = txn
            .commit()
            .await
            .map_err(|err| Box::new(anyhow::anyhow!(err)));
        Ok(())
    }

    async fn delete_one(&self, query: SyncPlanQueryDTO) -> Result<()> {
        // Start building the delete statement
        let mut delete_stmt = sync_plan_do::Entity::delete_many();

        // Apply filters based on the fields in SyncPlanQueryDTO
        if let Some(id) = query.id() {
            delete_stmt = delete_stmt.filter(sync_plan_do::Column::Id.eq(*id));
        }

        if let Some(name) = query.name() {
            delete_stmt = delete_stmt.filter(sync_plan_do::Column::Name.contains(name));
        }

        if let Some(active) = query.active() {
            delete_stmt = delete_stmt.filter(sync_plan_do::Column::Active.eq(*active));
        }
        // ... apply more filters for other fields in SyncPlanQueryDTO ...

        // Execute the delete statement
        let _ = delete_stmt
            .exec(&*self.db)
            .await
            .map_err(|err| Box::new(anyhow::anyhow!(err)));
        Ok(())
    }

    async fn delete_many(&self, queries: Vec<SyncPlanQueryDTO>) -> Result<()> {
        let txn = self.db.begin().await.map_err(|err| Box::new(err))?;

        for query in queries {
            let mut delete_stmt = sync_plan_do::Entity::delete_many();

            // Apply filters based on the fields in SyncPlanQueryDTO
            if let Some(id) = query.id() {
                delete_stmt = delete_stmt.filter(sync_plan_do::Column::Id.eq(*id));
            }
            if let Some(name) = query.name() {
                delete_stmt = delete_stmt.filter(sync_plan_do::Column::Name.contains(name));
            }
            // ... apply more filters for other fields in SyncPlanQueryDTO ...

            // Execute the delete statement for each query
            let _ = delete_stmt
                .exec(&*self.db)
                .await
                .map_err(|e| Box::new(e) as Box<dyn Error>);
        }

        let _ = txn
            .commit()
            .await
            .map_err(|err| Box::new(anyhow::anyhow!(err)));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::synchronization::{
        sync_plan::SyncFrequency,
        value_objects::sync_config::{SyncConfig, SyncMode},
    };

    use super::*;
    use chrono::Local;
    use lazy_static::lazy::Lazy;
    use lazy_static::lazy_static;
    use log::info;
    use sea_orm::{Database, DbBackend, EntityTrait, MockDatabase, MockExecResult, Set, Schema, sea_query::TableCreateStatement};
    use uuid::Uuid;

    // static insert_id: uuid::Uuid = Lazy::new(|| { Uuid::parse_str("a1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8").unwrap()});

    // Function to set up an in-memory database or mock
    async fn setup_database() -> Result<Arc<DatabaseConnection>> {
        // Here you would set up your in-memory database or configure the mock.
        let db = Database::connect("sqlite::memory:").await?;
        Ok(Arc::new(db))
    }

    async fn setup_schema(db: &DbConn) {
        // Setup Schema helper
        let schema = Schema::new(DbBackend::Sqlite);

        // Derive from Entity
        let stmt: TableCreateStatement = schema.create_table_from_entity(sync_plan_do::Entity);
        // Execute create table statement
        let result = db.execute(db.get_database_backend().build(&stmt)).await;
        println!("{:?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_one_sync_plan() {
        
        let db = setup_database().await.unwrap();
        setup_schema(&*db).await;
        let repository = SyncPlanRepositoryImpl { db: db.clone() };

        let new_sync_plan = SyncPlan::default();
        let result = repository.add_one(new_sync_plan).await;
        println!("{:?}", result);
        // assert!(result.is_ok());
        // Add more specific assertions here
    }

    #[tokio::test]
    async fn test_find_one_or_none_sync_plan() {
        let db = setup_database().await.unwrap();
        setup_schema(&*db).await;
        let repository = SyncPlanRepositoryImpl { db: db.clone() };

        let query_dto = SyncPlanQueryDTO::new().with_id(Uuid::new_v4());
        let result = repository.find_one_or_none(query_dto).await;
        assert!(result.is_ok());
        // Add more specific assertions here
    }

    #[tokio::test]
    async fn test_update_one_sync_plan() {
        let db = setup_database().await.unwrap();
        let repository = SyncPlanRepositoryImpl { db: db.clone() };

        let update_dto = SyncPlanUpdateDTO::new().with_id(Uuid::new_v4());
        let result = repository.update_one(update_dto).await;
        assert!(result.is_ok());
        // Add more specific assertions here
    }

    // ... Additional tests ...

    // Make sure to use the correct attribute for async tests
}
