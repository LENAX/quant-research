use sea_orm::entity::prelude::*;
use uuid::Uuid;
use chrono::{DateTime, Local};

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