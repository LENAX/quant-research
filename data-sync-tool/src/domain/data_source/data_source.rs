//! Data Source Domain Object Definition

use chrono::prelude::*;
use fake::{ Fake};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use std::{cell::RefCell, collections::HashMap, error, fmt, sync::Arc};
use uuid::Uuid;

use super::{dataset::Dataset, value_object::local_storage::LocalStorage};

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

// Errors
#[derive(Debug, Clone)]
pub struct UpdateTimeEarlierThanCreationError;
impl fmt::Display for UpdateTimeEarlierThanCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Update time should be later than creation time!")
    }
}

#[derive(Debug, Clone)]
pub struct UpdateStatusShouldCoexistWithItsDate;
impl fmt::Display for UpdateStatusShouldCoexistWithItsDate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "If the entity is updated, then it should has an update status."
        )
    }
}

impl error::Error for UpdateStatusShouldCoexistWithItsDate {}
impl error::Error for UpdateTimeEarlierThanCreationError {}

#[derive(Debug, PartialEq, Eq, Clone, Getters, Setters, MutGetters, CopyGetters)]
pub struct DataSource {
    #[getset(get = "pub", set = "pub")]
    id: Uuid,

    #[getset(get = "pub", set = "pub")]
    name: String,

    #[getset(get = "pub", set = "pub")]
    description: String,

    #[getset(get = "pub", set = "pub")]
    api_key: String,

    #[getset(get = "pub", set = "pub")]
    create_date: DateTime<Local>,

    #[getset(get = "pub")]
    last_update_time: Option<DateTime<Local>>,

    #[getset(get = "pub", set = "pub")]
    update_successful: Option<bool>, // defaults to true if last_update_time is provided

    #[getset(get = "pub")]
    datasets: HashMap<String, Dataset>,

    #[getset(get = "pub", set = "pub")]
    local_storage: LocalStorage,
}

impl DataSource {
    pub fn new(
        id: Uuid,
        name: &str,
        description: &str,
        api_key: &str,
        create_date: DateTime<Local>,
        last_update_time: Option<DateTime<Local>>,
        update_successful: Option<bool>,
        datasets: &[Dataset],
    ) -> Result<Self> {
        let mut id_mapped_datasets: HashMap<String, Dataset> = HashMap::new();

        datasets.into_iter().for_each(|v| {
            let dataset_id = v.id().to_string();
            id_mapped_datasets.insert(dataset_id, v.clone());
        });

        match last_update_time {
            None => {
                if let Some(_) = update_successful {
                    return Err(Box::new(UpdateStatusShouldCoexistWithItsDate));
                } else {
                    return Ok(Self {
                        id,
                        name: name.to_string(),
                        description: description.to_string(),
                        api_key: api_key.to_string(),
                        create_date,
                        last_update_time: None,
                        update_successful: None,
                        datasets: id_mapped_datasets,
                        local_storage: LocalStorage::default(),
                    });
                }
            }
            Some(update_dt) => {
                if let Some(update_ok) = update_successful {
                    return Ok(Self {
                        id,
                        name: name.to_string(),
                        description: description.to_string(),
                        api_key: api_key.to_string(),
                        create_date,
                        last_update_time: Some(update_dt),
                        update_successful: Some(update_ok),
                        datasets: id_mapped_datasets,
                        local_storage: LocalStorage::default()
                    });
                } else {
                    return Ok(Self {
                        id,
                        name: name.to_string(),
                        description: description.to_string(),
                        api_key: api_key.to_string(),
                        create_date,
                        last_update_time: Some(update_dt),
                        update_successful: Some(false),
                        datasets: id_mapped_datasets,
                        local_storage: LocalStorage::default()
                    });
                }
            }
        }
    }

    pub fn set_last_update_time(&mut self, update_dt: DateTime<Local>) -> Result<&mut Self> {
        if self.create_date > update_dt {
            Err(Box::new(UpdateTimeEarlierThanCreationError))
        } else {
            self.last_update_time = Some(update_dt);
            Ok(self)
        }
    }

    pub fn add_datasets(&mut self, datasets: &Vec<Dataset>) -> Result<&mut Self> {
        for dataset in datasets {
            self.datasets
                .insert(dataset.id().to_string(), dataset.clone());
        }
        return Ok(self);
    }

    pub fn get_datasets_by_ids(
        &self,
        dataset_ids: &Vec<&str>,
    ) -> HashMap<String, Dataset> {
        let mut result_map: HashMap<String, Dataset> = HashMap::new();
        for id in dataset_ids {
            if let Some(matched_ds) = self.datasets.get(*id) {
                result_map.insert(String::from(*id), matched_ds.clone());
            }
        }
        result_map
    }

    pub fn get_datasets_requires_sync(&self) -> HashMap<String, Dataset> {
        let mut datasets_require_sync: HashMap<String, Dataset> = HashMap::new();
        self.datasets
            .iter()
            .filter(|kv| {
                let ds = kv.1.clone();
                let ds_ref = ds;
                let need_sync = *ds_ref.sync_enabled();
                need_sync
            })
            .for_each(|kv| {
                datasets_require_sync.insert(kv.0.to_string(), kv.1.clone());
            });
        return datasets_require_sync;
    }

    pub fn remove_dataset_by_ids(&mut self, dataset_ids: &Vec<String>) -> &mut Self {
        for dataset_id in dataset_ids {
            self.datasets.remove(dataset_id);
        }
        return self;
    }

    pub fn remove_all_datasets(&mut self) -> &mut Self {
        self.datasets.clear();
        return self;
    }

    // TODO: Advanced Features: Automatic Schema Import and Parsing

    // TODO: Advanced Features: Automatic DB Schema Creation

    // TODO: Advanced Features: Automatic Schema Migration
}

impl std::fmt::Display for DataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,
               "DataSource(id: {},  name: {}, description: {}, api_key: {}, create_date: {}, last_update_time: {:?}, update_successful: {:?}, datasets: {:?})",
               self.id, self.name, self.description, self.api_key, self.create_date.with_timezone(&Local), Some(self.last_update_time),
               Some(self.update_successful), self.datasets)
    }
}

impl Default for DataSource {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::from("New DataSource"),
            description: String::from("Please write a description."),
            api_key: String::from(""),
            create_date: chrono::offset::Local::now(),
            last_update_time: None,
            update_successful: None,
            datasets: HashMap::new(),
            local_storage: LocalStorage::default()
        }
    }
}
