// Synchronization Plan Definition
// Defines when synchronization of a dataset should happend

use std::{str::FromStr, collections::HashMap};

use super::{
    custom_errors::TaskCreationError,
    sync_task::SyncTask,
    value_objects::sync_config::{self, SyncConfig},
    value_objects::task_spec::{RequestMethod, TaskSpecification},
};
use chrono::prelude::*;
use derivative::Derivative;
use fake::Fake;
use getset::{Getters, Setters};
use itertools::izip;
use serde_json::Value;
use url::{ParseError, Url};
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
pub struct CreateTaskRequest<'a> {
    url: Url,
    request_method: RequestMethod,
    request_header: HashMap<&'a str, &'a str>,
    payload: Option<&'a Value>,
}

// Synchronization Plan
#[derive(Derivative, Debug, PartialEq, Eq, Clone, Getters, Setters, Default)]
#[getset(get = "pub", set = "pub")]
pub struct SyncPlan<'a> {
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
    tasks: Vec<SyncTask<'a>>,
    datasource_id: Option<Uuid>,
    datasource_name: Option<String>,
    dataset_id: Option<Uuid>,
    dataset_name: Option<String>,
    param_template_id: Option<Uuid>,
}

impl<'a> SyncPlan<'a> {
    pub fn new(
        name: &'a str,
        description: &'a str,
        trigger_time: Option<DateTime<Local>>,
        frequency: SyncFrequency,
        active: bool,
        tasks: Vec<SyncTask<'a>>,
        datasource_id: Option<Uuid>,
        datasource_name: &'a str,
        dataset_id: Option<Uuid>,
        dataset_name: &'a str,
        param_template_id: Option<Uuid>,
        sync_config: SyncConfig,
    ) -> SyncPlan<'a> {
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
        &'a mut self,
        requests: &'a [CreateTaskRequest<'a>],
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
}
