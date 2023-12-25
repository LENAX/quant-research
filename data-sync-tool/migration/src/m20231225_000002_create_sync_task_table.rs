use async_trait;
use sea_orm::{DeriveIden, EnumIter, Iden, Iterable};
use sea_orm_migration::prelude::*;
use sea_orm_migration::DbErr;

#[derive(DeriveIden, EnumIter)]
pub enum SyncTask {
    Table,
    Id,
    SyncPlanId,
    Status,
    StartTime,
    EndTime,
    CreateTime,
    Spec,
    ResultMessage,
}

#[derive(Iden, EnumIter)]
pub enum TaskStatus {
    Table,
    Pending,
    Running,
    Completed,
    Failed,
    // ... any other statuses ...
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SyncTask::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(SyncTask::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(SyncTask::SyncPlanId).uuid().not_null())
                    .col(
                        ColumnDef::new(SyncTask::Status)
                        .enumeration(TaskStatus::Table, TaskStatus::iter().skip(1))
                        .string()
                        .not_null(),
                    )
                    .col(ColumnDef::new(SyncTask::StartTime).date_time().not_null())
                    .col(ColumnDef::new(SyncTask::EndTime).date_time().null())
                    .col(ColumnDef::new(SyncTask::CreateTime).date_time().not_null())
                    .col(ColumnDef::new(SyncTask::Spec).json().not_null())
                    .col(ColumnDef::new(SyncTask::ResultMessage).string().null())
                    // .foreign_key(
                    //     ForeignKey::create()
                    //         .from(SyncTask::Entity, SyncTask::SyncPlanId)
                    //         .to(SyncPlan::Entity, SyncPlan::Column::Id)
                    //         .on_delete(ForeignKeyAction::Cascade)
                    //         .on_update(ForeignKeyAction::Cascade),
                    // )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SyncTask::Table).to_owned())
            .await
    }
}
