// Data Schema Value Object Definition

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use fake::{Dummy, Fake};
use getset::{Getters, MutGetters};

#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
pub enum ColumnType {
    String,
    Int,
    Float,
    Double
}

#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, MutGetters)]
pub struct Column {
    #[getset(get = "pub")]
    name: String,
    #[getset(get = "pub")]
    col_type: ColumnType,
    #[getset(get = "pub")]
    description: String,
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
