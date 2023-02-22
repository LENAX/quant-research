// Data Schema Value Object Definition

use std::collections::HashMap;

use fake::{Dummy, Fake};

#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
#[readonly::make]
pub struct Column {
    pub name: String,
    pub col_type: String,
    pub description: String,
}

#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
#[readonly::make]
pub struct DataSchema {
    pub columns: HashMap<String, Column>,
}

impl DataSchema {
    pub fn new(columns: Vec<Column>) -> Self {
        let mut column_map: HashMap<String, Column> = HashMap::new();
        for column in columns {
            column_map.insert(column.name.clone(), column);
        }

        Self {
            columns: column_map,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
