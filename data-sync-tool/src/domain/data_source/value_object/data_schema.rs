// Data Schema Value Object Definition

use getset::{Getters, MutGetters};
use std::{collections::HashMap, error::Error, str::FromStr};

use super::field_type::FieldType;

#[derive(Debug, PartialEq, Eq, Clone, Getters, MutGetters)]
#[getset(get = "pub")]
pub struct Column {
    name: String,
    col_type: FieldType,
    description: String,
}

impl Column {
    pub fn new(name: &str, col_type: &str, description: &str) -> Result<Self, Box<dyn Error>> {
        let column_type = FieldType::from_str(col_type)?;
        Ok(Self {
            name: name.to_string(),
            col_type: column_type,
            description: description.to_string(),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Getters)]
pub struct DataSchema {
    #[getset(get = "pub", get_mut = "pub")]
    columns: HashMap<String, Column>,
}

impl DataSchema {
    pub fn new(columns: &Vec<Column>) -> Self {
        let mut column_map: HashMap<String, Column> = HashMap::new();
        for column in columns {
            column_map.insert(column.name().to_string(), column.clone());
        }

        Self {
            columns: column_map,
        }
    }

    pub fn insert_columns(&mut self, columns: &Vec<Column>) -> &mut Self {
        for col in columns {
            let key = col.name().to_string();
            self.columns.insert(key, col.clone());
        }
        return self;
    }

    pub fn remove_columns_by_name(&mut self, names: &Vec<String>) -> &mut Self {
        for name in names {
            self.columns.remove(name);
        }
        return self;
    }

    pub fn remove_all_columns(&mut self) -> &mut Self {
        self.columns.clear();
        return self;
    }
}

impl Default for DataSchema {
    fn default() -> Self {
        Self {
            columns: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod test {}
