pub use sea_orm_migration::prelude::*;
pub struct Migrator;

pub mod m20231225_000002_create_sync_plan_table;
pub mod m20231225_000002_create_sync_task_table;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20231225_000002_create_sync_plan_table::Migration),
            Box::new(m20231225_000002_create_sync_task_table::Migration),
        ]
    }
}
