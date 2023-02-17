// Data Schema Value Object Definition

use std::collection::HashMap;

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
    pub columns: HashMap<String, Column>
}

impl DataSchema {
    pub fn new() -> Self {
         
    }
}

#[cfg(test)]
mod test {
    use super::*;
}