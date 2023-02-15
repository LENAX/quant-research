// Dataset Domain Object Definition

use chrono::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Dummy, PartialEq, Eq)]
pub struct Dataset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub create_date: DateTime<Local>,
    pub last_update: DateTime<Local>,
    pub update_successful: bool,
    pub api_name: String,
    pub api_params: HashMap<String, APIParam>,
    pub schema: DataSchema,
    pub values: DataValues
}