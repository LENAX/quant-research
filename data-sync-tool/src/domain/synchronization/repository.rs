// Interfaces for entity repositories

use super::{custom_errors::RepositoryError, sync_plan::SyncPlan, sync_task::SyncTask};
use async_trait::async_trait;
use tokio::sync::RwLock;
use uuid::Uuid;

#[async_trait]
pub trait SyncPlanRepository: Send + Sync {
    // Read
    // Plan
    async fn get_plan_by_id<'a>(&self, id: RwLock<&Uuid>) -> Result<SyncPlan, RepositoryError>;
    async fn get_plan_by_dataset_id<'a>(
        &self,
        dataset_id: &Uuid,
    ) -> Result<SyncPlan, RepositoryError>;
    async fn get_plan_by_dataset_name<'a>(
        &self,
        dataset_name: &str,
    ) -> Result<SyncPlan, RepositoryError>;
    async fn get_plans_by_datasource_id<'a>(
        &self,
        datasource_id: &Uuid,
    ) -> Result<Vec<SyncPlan>, RepositoryError>;
    async fn get_plans_by_datasource_name<'a>(
        &self,
        datasource_name: &str,
    ) -> Result<Vec<SyncPlan>, RepositoryError>;
    async fn get_plan_by_name<'a>(&self, name: &str) -> Result<SyncPlan, RepositoryError>;
    async fn get_plans_by_activation_status<'a>(
        &self,
        is_active: bool,
    ) -> Result<Vec<SyncPlan>, RepositoryError>;
    async fn get_plans_by_frequency<'a>(
        &self,
        sync_frequency: &str,
    ) -> Result<Vec<SyncPlan>, RepositoryError>;
    async fn get_plans_pass_due<'a>(&self) -> Result<Vec<SyncPlan>, RepositoryError>;
    async fn list_plans<'a>(
        &self,
        page_size: Option<usize>,
        page_number: Option<usize>,
    ) -> Result<Vec<SyncPlan>, RepositoryError>;

    // Task
    async fn get_task_by_id<'a>(&self, id: &Uuid) -> Result<SyncTask, RepositoryError>;
    async fn get_tasks_by_plan_id<'a>(
        &self,
        plan_id: &Uuid,
    ) -> Result<Vec<SyncTask>, RepositoryError>;
    async fn get_tasks_by_datasource_id<'a>(
        &self,
        datasource_ids: &[&Uuid],
    ) -> Result<Vec<SyncTask>, RepositoryError>;
    async fn get_tasks_by_datasource_name<'a>(
        &self,
        datasource_name: &str,
    ) -> Result<Vec<SyncTask>, RepositoryError>;
    async fn get_tasks_by_dataset_id<'a>(
        &self,
        dataset_ids: &[&Uuid],
    ) -> Result<Vec<SyncTask>, RepositoryError>;
    async fn get_tasks_by_dataset_name<'a>(
        &self,
        dataset_name: &str,
    ) -> Result<Vec<SyncTask>, RepositoryError>;

    // Create
    async fn save_plan<'a>(
        &self,
        plan: &SyncPlan,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn save_plans<'a>(
        &self,
        plans: &[&SyncPlan],
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;

    // Update
    async fn add_tasks_to_plans<'a>(
        &self,
        tasks: &[&SyncTask],
        plan_id: Uuid,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn create_plans_for_datasource<'a>(
        &self,
        plans: &[&SyncPlan],
        datasource_id: &Uuid,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn create_plan_for_dataset<'a>(
        &self,
        plan: &SyncPlan,
        dataset_id: &Uuid,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn update_plan_activation_status<'a>(
        &self,
        plan_id: &Uuid,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn update_activation_status_for_datasource<'a>(
        &self,
        active: bool,
        datasource_id: &Uuid,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn update_sync_frequency<'a>(
        &self,
        sync_frequency: &str,
        plan_id: &Uuid,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn update_plans<'a>(
        &self,
        plans: &[&SyncPlan],
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;

    // Delete
    async fn delete_plan_by_id<'a>(
        &self,
        plan_id: &Uuid,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn delete_plans<'a>(
        &self,
        plan_ids: &[Uuid],
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn delete_plan_for_dataset<'a>(
        &self,
        dataset_id: &Uuid,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn delete_plans_for_datasource<'a>(
        &self,
        datasource_id: &Uuid,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn delete_deactivated_plans_for_datasource<'a>(
        &self,
        datasource_id: &Uuid,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
    async fn delete_tasks_for_plan<'a>(
        &self,
        task_ids: &[&Uuid],
        plan_id: Uuid,
    ) -> Result<Box<dyn SyncPlanRepository>, RepositoryError>;
}
