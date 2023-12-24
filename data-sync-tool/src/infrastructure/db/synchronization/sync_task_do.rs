use chrono::{DateTime, Local};
use sea_orm::{Related, RelationDef, RelationTrait};
use sea_orm::{DeriveEntityModel, ActiveModelBehavior, DeriveRelation, EnumIter};
use serde::{Deserialize, Serialize};
use sea_orm::DerivePrimaryKey;
use serde_json::Value;
use uuid::Uuid;
use sea_orm::PrimaryKeyTrait;
use sea_orm::EntityTrait;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sync_task")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    #[sea_orm(indexed)]
    pub sync_plan_id: Uuid, // assuming there's a foreign key to SyncPlan
    pub status: String,
    #[sea_orm(indexed)]
    pub start_time: DateTime<Local>,
    pub end_time: Option<DateTime<Local>>,
    #[sea_orm(indexed)]
    pub create_time: DateTime<Local>,
    pub spec: Value, // data payload and specification of the task
    pub result_message: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::sync_plan_do::Entity",
        from = "Column::SyncPlanId",
        to = "super::sync_plan_do::Column::Id"
    )]
    SyncPlan,
}

impl Related<super::sync_plan_do::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SyncPlan.def()
    }
}


impl ActiveModelBehavior for ActiveModel {}