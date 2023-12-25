use async_trait;
use chrono::prelude::*;
use sea_orm::{DeriveIden, EnumIter, Iden};
use sea_orm_migration::prelude::*;

#[derive(DeriveIden, EnumIter)]
pub enum SyncPlan {
    Table,
    Id,
    Name,
    Description,
    TriggerTime,
    Frequency,
    Active,
    SyncConfig,
    // Add other columns as needed
    DatasourceId,
    DatasourceName,
    DatasetId,
    DatasetName,
    ParamTemplateId,
    // More fields here as per your model
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SyncPlan::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(SyncPlan::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(SyncPlan::Name).string().not_null())
                    .col(ColumnDef::new(SyncPlan::Description).string().not_null())
                    .col(ColumnDef::new(SyncPlan::TriggerTime).date_time().null())
                    .col(ColumnDef::new(SyncPlan::Frequency).string().not_null())
                    .col(ColumnDef::new(SyncPlan::Active).boolean().not_null())
                    .col(ColumnDef::new(SyncPlan::SyncConfig).string().not_null())
                    // ... other fields ...
                    .col(ColumnDef::new(SyncPlan::DatasourceId).uuid().null())
                    .col(ColumnDef::new(SyncPlan::DatasourceName).string().null())
                    .col(ColumnDef::new(SyncPlan::DatasetId).uuid().null())
                    .col(ColumnDef::new(SyncPlan::DatasetName).string().null())
                    .col(ColumnDef::new(SyncPlan::ParamTemplateId).uuid().null())
                    // Define relationships or foreign keys if necessary
                    // For example, if you have a relation to another table
                    // .foreign_key(
                    //     ForeignKey::create()
                    //         .from(SyncPlan::Table, SyncPlan::SomeForeignKey)
                    //         .to(OtherTable, OtherColumn::Id)
                    //         .on_delete(ForeignKeyAction::Cascade)
                    //         .on_update(ForeignKeyAction::Cascade),
                    // )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(SyncPlan::Table).to_owned()).await
    }
}
