use sea_orm::{DeriveEntityModel, ActiveModelBehavior, DeriveRelation, EnumIter};
use serde::{Deserialize, Serialize};
use sea_orm::DerivePrimaryKey;
use uuid::Uuid;
use sea_orm::PrimaryKeyTrait;
use sea_orm::EntityTrait;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "sync_task")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    // ... other fields ...
    pub plan_id: Uuid, // assuming there's a foreign key to SyncPlan
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}