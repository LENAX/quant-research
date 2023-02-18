// APIParams Value Object Definition

use chrono::prelude::*;
use std::collections::HashMap;
use fake::{Dummy, Fake};

// #[derive(Clone, Copy, PartialEq, Eq)]
// enum ArgType {
//     String,
//     Int,
//     Float,
//     DateTime
// }

#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
#[readonly::make]
pub struct APIParam {
    name: String,
    description: String,
    arg_type: String,
    required: bool,
    require_generation: bool,
    generate_by: String,
}

impl APIParam {
    pub fn new(name: &str, description: &str, 
               arg_type: &str, required: bool, require_generation: bool,
               generate_by: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            arg_type: arg_type.to_string(),
            required,
            require_generation,
            generate_by: generate_by.to_string(),
        }  
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
