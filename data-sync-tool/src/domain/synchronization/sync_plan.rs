// Synchronization Plan Definition
// Defines when synchronization of a dataset should happend

use chrono::prelude::*;
use fake::{Dummy, Fake};
use uuid::Uuid;
use getset::{Getters, Setters};

// use std::collections::HashMap;
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SyncFrequency {
    Continuous,
    PerMinute,
    PerHour,
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly
}

// Synchronization Plan
#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct SyncPlan {
    id: Uuid,
    name: String,
    description: String,
    trigger_time: Option<DateTime<Utc>>,
    frequency: SyncFrequency,
    active: bool,
    datasource_id: Option<Uuid>,
    datasource_name: Option<String>,
    dataset_id: Option<Uuid>,
    dataset_name: Option<String>,
}

impl Default for SyncPlan {
    fn default() -> Self {
        return SyncPlan{
            id: Uuid::new_v4(),
            name: String::from("New plan"),
            description: String::from("Please complete the plan by adding it to a dataset"),
            frequency: SyncFrequency::Daily,
            active: false,
            datasource_id: None,
            datasource_name: None,
            dataset_id: None,
            dataset_name: None,
            trigger_time: None,
        }
    }
}

impl SyncPlan {
    pub fn new(
        name: &str, description: &str, trigger_time: Option<DateTime<Utc>>,
        frequency: SyncFrequency, active: bool, datasource_id: Option<Uuid>,
        datasource_name: &str, dataset_id: Option<Uuid>, dataset_name: &str
    ) -> SyncPlan {
        SyncPlan {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.to_string(),
            trigger_time,
            frequency,
            active,
            datasource_id,
            datasource_name: Some(datasource_name.to_string()),
            dataset_name: Some(dataset_name.to_string()),
            dataset_id,
        }
    }

    pub fn set_plan_for(
        &mut self, datasource_id: Uuid, datasource_name: &str,
        dataset_id: Uuid, dataset_name: &str
    ) -> &mut Self {
        self.datasource_id = Some(datasource_id);
        self.datasource_name = Some(datasource_name.to_string());
        self.dataset_id = Some(dataset_id);
        self.dataset_name = Some(dataset_name.to_string());

        return self;
    }

    pub fn should_trigger(self) -> bool {
        match self.trigger_time {
            Some(trigger_time) => {
                let now = Utc::now();
                return now >= trigger_time;
            },
            None => return false,
        }
    }
}