// Data Schema Value Object Definition

use fake::{Dummy, Fake};
use getset::{Getters, MutGetters};
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
mod test {
    use super::*;
}
