use sea_orm::{entity::prelude::*};
use uuid::Uuid;
use chrono::{DateTime, Local};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "sync_plan")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub trigger_time: Option<DateTime<Local>>,
    // Frequency and other complex types might need to be stored as a serialized format or as a reference to another table
    pub frequency: String, // For simplicity, assuming frequency is a string here. Adjust as needed.
    pub active: bool,
    // `sync_config` and `tasks` might represent relations to other tables or serialized JSON/YAML data
    pub sync_config: String, // Consider creating a separate table or storing as a serialized string
    // ... other fields ...
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}