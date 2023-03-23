// Data Schema Value Object Definition

use std::collections::HashMap;

use fake::{Dummy, Fake};
use getset::{Getters, Setters};


#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, Setters, Default)]
#[getset(get, set, get_mut)]
pub struct Column {
    name: String,
    col_type: String,
    description: String,
}

#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, Setters, Default)]
#[getset(get, set, get_mut)]
pub struct DataSchema {
    columns: HashMap<String, Column>
}

impl DataSchema {
    pub fn new(columns: Vec<Column>) -> Self {
        let mut column_map: HashMap<String, Column> = HashMap::new();
        for column in columns {
            column_map.insert(column.name.clone(), column);
        }

        Self { columns: column_map }
    }
}

#[cfg(test)]
mod test {
    use super::*;
}