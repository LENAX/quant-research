// Synchronization Plan Definition
// Defines when synchronization of a dataset should happend

use std::{collections::HashMap, sync::Arc};

use super::{
    custom_errors::TaskCreationError, sync_task::SyncTask, value_objects::sync_config::{SyncConfig, SyncMode},
    value_objects::task_spec::RequestMethod,
};
use chrono::prelude::*;
use derivative::Derivative;

use getset::{Getters, Setters};

use serde_json::Value;
use url::Url;
use uuid::Uuid;

#[derive(Derivative)]
#[derivative(Default(bound = ""))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SyncFrequency {
    Continuous,
    PerMinute,
    PerHour,
    #[derivative(Default)]
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

#[derive(Derivative, Debug, PartialEq, Eq, Clone, Getters, Setters)]
#[getset(get = "pub", set = "pub")]
pub struct CreateTaskRequest {
    url: Url,
    request_method: RequestMethod,
    request_header: HashMap<String, String>,
    payload: Option<Value>,
}

// Synchronization Plan
#[derive(Derivative, Debug, Clone, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub")]
pub struct SyncPlan {
    id: Uuid,
    #[derivative(Default(value = "New Plan"))]
    name: String,
    #[derivative(Default(value = "Please add a description"))]
    description: String,
    trigger_time: Option<DateTime<Local>>,
    frequency: SyncFrequency,
    #[derivative(Default(value = "false"))]
    active: bool,
    sync_config: SyncConfig,
    tasks: Vec<SyncTask>,
    datasource_id: Option<Uuid>,
    datasource_name: Option<String>,
    dataset_id: Option<Uuid>,
    dataset_name: Option<String>,
    param_template_id: Option<Uuid>,
}

impl SyncPlan {
    pub fn new(
        name: &str,
        description: &str,
        trigger_time: Option<DateTime<Local>>,
        frequency: SyncFrequency,
        active: bool,
        tasks: Vec<SyncTask>,
        datasource_id: Option<Uuid>,
        datasource_name: &str,
        dataset_id: Option<Uuid>,
        dataset_name: &str,
        param_template_id: Option<Uuid>,
        sync_config: SyncConfig,
    ) -> SyncPlan {
        SyncPlan {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: description.to_string(),
            trigger_time,
            frequency,
            active,
            tasks: tasks,
            datasource_id,
            datasource_name: Some(datasource_name.to_string()),
            dataset_name: Some(dataset_name.to_string()),
            dataset_id,
            param_template_id,
            sync_config,
        }
    }

    pub fn set_plan_for(
        &mut self,
        datasource_id: Uuid,
        datasource_name: &str,
        dataset_id: Uuid,
        dataset_name: &str,
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
                let now = Local::now();
                return now >= trigger_time;
            }
            None => return false,
        }
    }

    pub fn create_tasks(
        &mut self,
        requests: &[CreateTaskRequest],
    ) -> Result<&mut Self, TaskCreationError> {
        for request in requests {
            let mut new_task = SyncTask::from(request);

            new_task
                .set_start_time(Local::now())
                .set_dataset_id(self.dataset_id)
                .set_dataset_name(self.dataset_name.clone())
                .set_datasource_id(self.datasource_id)
                .set_datasource_name(self.dataset_name.clone())
                .set_sync_plan_id(Some(self.id));
            self.tasks.push(new_task);
        }

        return Ok(self);
    }

    pub fn sync_mode(&self) -> &SyncMode {
        return self.sync_config.sync_mode()
    }
}
