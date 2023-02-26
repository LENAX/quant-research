// Data Schema Value Object Definition

use fake::{Dummy, Fake};
use getset::{Getters, MutGetters};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use super::field_type::FieldType;
use crate::common::errors::Result;

#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, MutGetters)]
pub struct Column {
    #[getset(get = "pub")]
    name: String,
    #[getset(get = "pub")]
    col_type: FieldType,
    #[getset(get = "pub")]
    description: String,
}

impl Column {
    pub fn new(name: &str, col_type: &str, description: &str) -> Result<Self> {
        let column_type = FieldType::try_from(col_type.to_string())?;
        Ok(Self {
            name: name.to_string(),
            col_type: column_type,
            description: description.to_string(),
        })
    }
}

#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters)]
pub struct DataSchema {
    #[getset(get = "pub", get_mut = "pub")]
    columns: HashMap<String, Rc<RefCell<Column>>>,
}

impl DataSchema {
    pub fn new(columns: &Vec<Rc<RefCell<Column>>>) -> Self {
        let mut column_map: HashMap<String, Rc<RefCell<Column>>> = HashMap::new();
        for column in columns {
            column_map.insert(column.as_ref().borrow().name.clone(), column.clone());
        }

        Self {
            columns: column_map,
        }
    }

    pub fn insert_columns(&mut self, columns: &Vec<Rc<RefCell<Column>>>) -> &mut Self {
        for col in columns {
            let key = col.as_ref().borrow().name.clone();
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
