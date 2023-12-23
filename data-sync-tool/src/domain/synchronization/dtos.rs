use chrono::{Local, DateTime};
use getset::{Getters, Setters, MutGetters};
use uuid::Uuid;
use super::sync_plan::SyncFrequency;

#[derive(Debug, Getters, Setters, MutGetters)]
#[getset(get, set, get_mut)]
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


#[derive(Debug, Getters, Setters, MutGetters)]
#[getset(get, set, get_mut)]
pub struct SyncPlanUpdateDTO {
    id: Uuid, // Typically required to identify the plan to update
    name: Option<String>,
    description: Option<String>,
    trigger_time: Option<DateTime<Local>>,
    frequency: Option<SyncFrequency>,
    active: Option<bool>,
    // ... other fields that might be updatable
}

impl SyncPlanUpdateDTO {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            name: None,
            description: None,
            trigger_time: None,
            frequency: None,
            active: None,
            // ... Initialize other fields as None
        }
    }

    // Builder methods for each updatable field
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

    // ... Additional builder methods for other updatable fields

    // Getters or setters for accessing or modifying the fields
    // Depending on your framework or application structure, you might need to add getter methods
    // For simplicity, I'll add one example getter method:
    // ... additional getters or utility methods as necessary
}