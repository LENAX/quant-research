// Dataset Domain Object Definition
// Dataset describes the schema, the api endpoint, and the parameters to request for data

use chrono::prelude::*;
use fake::{Dummy, Fake};
use std::collections::HashMap;
use super:: value_object::{api_param::APIParam, data_schema::DataSchema};
use getset::{Getters, Setters};


#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, Setters, Default)]
#[getset(get, set, get_mut)]
pub struct Dataset {
    id: String,
    name: String,
    description: String,
    endpoint: String, // web endpoint of this dataset
    params: HashMap<String, APIParam>, // a hashmap of api arguments
    schema: DataSchema,  // schema of this dataset
    create_date: DateTime<Utc>,
    last_update: Option<DateTime<Utc>>,
    update_successful: Option<bool>,
    sync_on: bool,
}

impl Dataset {
    pub fn new(id: &str, name: &str, description: &str, 
               endpoint: &str, params: HashMap<String, APIParam>, 
               schema: DataSchema, create_date: DateTime<Utc>,
               last_update: Option<DateTime<Utc>>, update_successful: Option<bool>, sync_on: bool) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            endpoint: endpoint.to_string(),
            params,
            schema,
            create_date,
            last_update,
            update_successful,
            sync_on,
        }  
    }
}


#[cfg(test)]
mod test {
    use super::*;
}
