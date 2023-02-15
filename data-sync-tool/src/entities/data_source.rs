// Data Source Domain Object Definition

use chrono::prelude::*;
use std::collections::HashMap;
use fake::{Dummy, Fake};

#[derive(Debug, Dummy, PartialEq, Eq)]
pub struct DataSource {
    pub id: String,
    pub name: String,
    pub description: String,
    pub api_key: String,
    pub create_date: DateTime<Local>,
    pub last_update: DateTime<Local>,
    pub update_successful: bool,
    pub datasets: HashMap<String, Dataset>
}