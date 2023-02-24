// Data Schema Value Object Definition

use std::{cell::RefCell, collections::HashMap, rc::Rc};
use std::{error, fmt};
use lazy_static::lazy_static;
use fake::{Dummy, Fake};
use getset::{Getters, MutGetters};
use regex::Regex;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

lazy_static! {
    static ref STRING_ARG_TYPE_PATTERN: Regex = Regex::new("^str|string|String$").unwrap();
    static ref INT_ARG_TYPE_PATTERN: Regex = Regex::new("^int|Integer$").unwrap();
    static ref FLOAT_ARG_TYPE_PATTERN: Regex = Regex::new("^int|Integer$").unwrap();
}

#[derive(Debug, Clone)]
pub struct InvalidColumnType;
impl fmt::Display for InvalidColumnType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Column can only be one of String, Int, and Float!")
    }
}
impl error::Error for InvalidColumnType {}

#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
pub enum ColumnType {
    String,
    Int,
    Float,
}

impl TryFrom<String> for ColumnType {
    type Error = InvalidColumnType;

    fn try_from(value: String) -> std::result::Result<ColumnType, InvalidColumnType> {
        if STRING_ARG_TYPE_PATTERN.is_match(&value) {
            Ok(ColumnType::String)
        } else if INT_ARG_TYPE_PATTERN.is_match(&value) {
            Ok(ColumnType::Int)
        } else if FLOAT_ARG_TYPE_PATTERN.is_match(&value) {
            Ok(ColumnType::Float)
        } else {
            Err(InvalidColumnType)
        }
    }
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

impl Column {
    pub fn new(name: &str, col_type: &str, description: &str) -> Result<Self> {
        let column_type = ColumnType::try_from(col_type.to_string())?;
        Ok(Self {
            name: name.to_string(),
            col_type: column_type,
            description: description.to_string()
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
