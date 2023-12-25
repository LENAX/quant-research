use chrono::{DateTime, Local};
use sea_orm::{entity::prelude::*, Set};
use uuid::Uuid;

use crate::domain::synchronization::sync_plan::{SyncPlan, SyncFrequency};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[cfg_attr(feature = "with-uuid", feature = "with-chrono")]
#[sea_orm(table_name = "sync_plan")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(indexed)]
    pub name: String,
    pub description: String,
    pub trigger_time: Option<DateTime<Local>>,
    // Frequency and other complex types might need to be stored as a serialized format or as a reference to another table
    pub frequency: String, // For simplicity, assuming frequency is a string here. Adjust as needed.
    pub active: bool,
    // `sync_config` and `tasks` might represent relations to other tables or serialized JSON/YAML data
    pub sync_config: String, // Consider creating a separate table or storing as a serialized string
    // ... other fields ...
    #[sea_orm(indexed, nullable)]
    pub datasource_id: Option<Uuid>,
    #[sea_orm(indexed, nullable)]
    pub datasource_name: Option<String>,
    #[sea_orm(indexed, nullable)]
    pub dataset_id: Option<Uuid>,
    #[sea_orm(indexed, nullable)]
    pub dataset_name: Option<String>,
    #[sea_orm(indexed, nullable)]
    pub param_template_id: Option<Uuid>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::sync_task_do::Entity")]
    SyncTask,
}

impl Related<super::sync_task_do::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SyncTask.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl From<SyncPlan> for self::ActiveModel {
    fn from(plan: SyncPlan) -> Self {
        let mut model = self::ActiveModel {
            id: Set(*plan.id()),
            name: Set(plan.name().to_string()),
            description: Set(plan.description().to_string()),
            trigger_time: Set(plan.trigger_time().to_owned()),
            frequency: Set(plan.frequency().to_string()), // Assuming frequency can be converted to a string
            active: Set(*plan.active()),
            sync_config: Set(serde_json::to_string(plan.sync_config()).unwrap()), // Assuming sync_config can be serialized to a JSON string
            // ...other fields...
            datasource_id: Set(*plan.datasource_id()),
            datasource_name: Set(plan.datasource_name().clone()),
            dataset_id: Set(*plan.dataset_id()),
            dataset_name: Set(plan.dataset_name().clone()),
            param_template_id: Set(*plan.param_template_id())
        };

        // Convert frequency and sync_config from domain to db representation here if they are not simple types

        model
    }
}

impl From<self::Model> for SyncPlan {
    fn from(model: self::Model) -> Self {
        SyncPlan::new(
            &model.name,
            &model.description,
            &model.trigger_time,
            SyncFrequency::from(model.frequency),
            model.active,
            &model.datasource_id,
            &model.datasource_name,
            &model.dataset_id,
            &model.dataset_name,
            &model.param_template_id,
            &model.sync_config,
        )
    }
}
