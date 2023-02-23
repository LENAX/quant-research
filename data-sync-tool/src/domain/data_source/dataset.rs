// Dataset Domain Object Definition
// Dataset describes the schema, the api endpoint, and the parameters to request for data

use super::value_object::{api_param::APIParam, data_schema::DataSchema};
use chrono::prelude::*;
use fake::{Dummy, Fake};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
#[readonly::make]
pub struct Dataset {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub endpoint: String,                  // web endpoint of this dataset
    pub params: HashMap<String, APIParam>, // a hashmap of api arguments
    pub schema: DataSchema,                // schema of this dataset
    pub create_date: DateTime<Utc>,
    pub last_update: Option<DateTime<Utc>>,
    pub update_successful: Option<bool>,
    pub sync_on: bool,
}

impl Dataset {
    pub fn new(
        id: Uuid,
        name: &str,
        description: &str,
        endpoint: &str,
        params: HashMap<String, APIParam>,
        schema: DataSchema,
        create_date: DateTime<Utc>,
        last_update: Option<DateTime<Utc>>,
        update_successful: Option<bool>,
        sync_on: bool,
    ) -> Self {
        Self {
            id,
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
