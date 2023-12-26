use chrono::{Local, DateTime};
use getset::{Getters, Setters, MutGetters};
use uuid::Uuid;
use super::{sync_plan::SyncFrequency, value_objects::sync_config::SyncConfig, sync_task::SyncTask};

#[derive(Debug, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
// Define a query object for SyncPlan
pub struct SyncPlanQueryDTO {
    id: Option<Uuid>,
    name: Option<String>,
    description: Option<String>,
    trigger_time: Option<DateTime<Local>>,
    frequency: Option<SyncFrequency>,
    active: Option<bool>,
    datasource_id: Option<Uuid>,
    dataset_id: Option<Uuid>,
    param_template_id: Option<Uuid>,
    // You might want to include additional fields related to searching or filtering SyncPlans
}

impl SyncPlanQueryDTO {
    pub fn new() -> Self {
        Self {
            id: None,
            name: None,
            description: None,
            trigger_time: None,
            frequency: None,
            active: None,
            datasource_id: None,
            dataset_id: None,
            param_template_id: None,
        }
    }

    // Builder methods for each field
    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_trigger_time(mut self, trigger_time: DateTime<Local>) -> Self {
        self.trigger_time = Some(trigger_time);
        self
    }

    pub fn with_frequency(mut self, frequency: SyncFrequency) -> Self {
        self.frequency = Some(frequency);
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    pub fn with_datasource_id(mut self, datasource_id: Uuid) -> Self {
        self.datasource_id = Some(datasource_id);
        self
    }

    pub fn with_dataset_id(mut self, dataset_id: Uuid) -> Self {
        self.dataset_id = Some(dataset_id);
        self
    }

    pub fn with_param_template_id(mut self, param_template_id: Uuid) -> Self {
        self.param_template_id = Some(param_template_id);
        self
    }

    // ... Additional builder methods for other fields ...
}


#[derive(Debug, Default, Getters, Setters, MutGetters)]
#[getset(get = "pub", set = "pub", get_mut = "pub")]
pub struct SyncPlanUpdateDTO {
    id: Option<Uuid>, // Typically required to identify the plan to update
    name: Option<String>,
    description: Option<String>,
    trigger_time: Option<DateTime<Local>>,
    frequency: Option<SyncFrequency>,
    active: Option<bool>,
    sync_config: Option<SyncConfig>,
    tasks: Option<Vec<SyncTask>>,
    datasource_id: Option<Uuid>,
    datasource_name: Option<String>,
    dataset_id: Option<Uuid>,
    dataset_name: Option<String>,
    param_template_id: Option<Uuid>,
}

impl SyncPlanUpdateDTO {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_id(mut self, id: Uuid) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_trigger_time(mut self, trigger_time: DateTime<Local>) -> Self {
        self.trigger_time = Some(trigger_time);
        self
    }

    pub fn with_frequency(mut self, frequency: SyncFrequency) -> Self {
        self.frequency = Some(frequency);
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    pub fn with_sync_config(mut self, sync_config: SyncConfig) -> Self {
        self.sync_config = Some(sync_config);
        self
    }

    pub fn with_tasks(mut self, tasks: Vec<SyncTask>) -> Self {
        self.tasks = Some(tasks);
        self
    }

    pub fn with_datasource_id(mut self, datasource_id: Uuid) -> Self {
        self.datasource_id = Some(datasource_id);
        self
    }

    pub fn with_datasource_name(mut self, datasource_name: String) -> Self {
        self.datasource_name = Some(datasource_name);
        self
    }

    pub fn with_dataset_id(mut self, dataset_id: Uuid) -> Self {
        self.dataset_id = Some(dataset_id);
        self
    }

    pub fn with_dataset_name(mut self, dataset_name: String) -> Self {
        self.dataset_name = Some(dataset_name);
        self
    }

    pub fn with_param_template_id(mut self, param_template_id: Uuid) -> Self {
        self.param_template_id = Some(param_template_id);
        self
    }

    // Add more builder methods as necessary for other fields
}