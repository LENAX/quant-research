// Dataset Domain Object Definition
// Dataset describes the schema, the api endpoint, and the parameters to request for data

use super::{
    data_source::{UpdateStatusShouldCoexistWithItsDate, UpdateTimeEarlierThanCreationError},
    value_object::{
        api_param::APIParam,
        data_schema::{Column, DataSchema},
    },
};
use chrono::prelude::*;
use fake::{ Fake};
use getset::{Getters, MutGetters, Setters};
use lazy_static::lazy_static;
use regex::Regex;
use std::{cell::RefCell, collections::HashMap, error, fmt, sync::Arc};
use uuid::Uuid;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug, Clone)]
pub struct InvalidAPIEndpointFormat;
impl fmt::Display for InvalidAPIEndpointFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "API endpoint format is invalid!")
    }
}
impl error::Error for InvalidAPIEndpointFormat {}

#[derive(Debug,  PartialEq, Eq, Clone, Getters, MutGetters, Setters)]
pub struct Dataset {
    #[getset(get = "pub", set = "pub")]
    id: Uuid,
    #[getset(get = "pub", set = "pub")]
    name: String,
    #[getset(get = "pub", set = "pub")]
    description: String,
    #[getset(get = "pub", set = "pub")]
    endpoint: String, // web endpoint of this api_param
    #[getset(get = "pub")]
    api_params: HashMap<String, APIParam>, // a hashmap of api arguments
    #[getset(get = "pub", get_mut = "pub")]
    schema: DataSchema, // schema of this api_param
    #[getset(get = "pub", set = "pub")]
    create_date: DateTime<Local>,
    #[getset(get = "pub")]
    last_update_time: Option<DateTime<Local>>,
    #[getset(get = "pub", set = "pub")]
    update_successful: Option<bool>,
    #[getset(get = "pub", set = "pub")]
    sync_enabled: bool,
}

impl Dataset {
    pub fn new(
        id: Uuid,
        name: &str,
        description: &str,
        endpoint: &str,
        api_params: &Vec<APIParam>,
        schema: DataSchema,
        create_date: DateTime<Local>,
        last_update_time: Option<DateTime<Local>>,
        update_successful: Option<bool>,
        sync_enabled: bool,
    ) -> Result<Self> {
        let mut params_map: HashMap<String, APIParam> = HashMap::new();
        api_params.into_iter().for_each(|p| {
            params_map.insert(p.name().clone(), p.clone());
        });
        lazy_static! {
            static ref RE: Regex = Regex::new("^/[a-zA-Z0-9-_]+$").unwrap();
        }
        if !RE.is_match(endpoint) {
            return Err(Box::new(InvalidAPIEndpointFormat));
        }

        match last_update_time {
            None => {
                if let Some(_) = update_successful {
                    return Err(Box::new(UpdateStatusShouldCoexistWithItsDate));
                } else {
                    return Ok(Self {
                        id,
                        name: name.to_string(),
                        description: description.to_string(),
                        endpoint: endpoint.to_string(),
                        api_params: params_map,
                        schema: schema,
                        create_date,
                        last_update_time: None,
                        update_successful: None,
                        sync_enabled,
                    });
                }
            }
            Some(update_dt) => {
                if let Some(update_ok) = update_successful {
                    return Ok(Self {
                        id,
                        name: name.to_string(),
                        description: description.to_string(),
                        endpoint: endpoint.to_string(),
                        api_params: params_map,
                        schema: schema,
                        create_date,
                        last_update_time: Some(update_dt),
                        update_successful: Some(update_ok),
                        sync_enabled,
                    });
                } else {
                    return Ok(Self {
                        id,
                        name: name.to_string(),
                        description: description.to_string(),
                        endpoint: endpoint.to_string(),
                        api_params: params_map,
                        schema: schema,
                        create_date,
                        last_update_time: Some(update_dt),
                        update_successful: Some(false),
                        sync_enabled,
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

    pub fn add_api_params(&mut self, api_params: &Vec<APIParam>) -> Result<&mut Self> {
        for api_param in api_params {
            self.api_params.insert(
                api_param.name().to_string(),
                api_param.clone(),
            );
        }
        return Ok(self);
    }

    pub fn get_api_params_by_name(
        &self,
        api_param_name: &Vec<&str>,
    ) -> HashMap<String, APIParam> {
        let mut result_map: HashMap<String, APIParam> = HashMap::new();
        for name in api_param_name {
            if let Some(matched_param) = self.api_params.get(*name) {
                result_map.insert(String::from(*name), matched_param.clone());
            }
        }
        result_map
    }

    pub fn remove_api_param_by_names(&mut self, api_param_names: &Vec<String>) -> &mut Self {
        for api_param_id in api_param_names {
            self.api_params.remove(api_param_id);
        }
        return self;
    }

    pub fn remove_all_api_params(&mut self) -> &mut Self {
        self.api_params.clear();
        return self;
    }

    pub fn add_columns_to_schema(&mut self, columns: &Vec<Column>) -> &mut Self {
        self.schema_mut().insert_columns(columns);
        // self.
        return self;
    }

    pub fn remove_columns_from_schema_by_name(&mut self, column_names: &Vec<String>) -> &mut Self {
        self.schema_mut()
            .remove_columns_by_name(column_names);
        return self;
    }

    pub fn remove_all_columns_from_schema(&mut self) -> &mut Self {
        self.schema_mut().remove_all_columns();
        return self;
    }
}

impl Default for Dataset {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::from("New APIParam"),
            description: String::from("Please write a description."),
            endpoint: String::from("/example/endpoint"),
            api_params: HashMap::new(),
            schema: DataSchema::default(),
            create_date: chrono::offset::Local::now(),
            last_update_time: None,
            update_successful: None,
            sync_enabled: false,
        }
    }
}
