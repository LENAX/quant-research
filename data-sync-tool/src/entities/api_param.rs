// APIParams Value Object Definition

use chrono::prelude::*;
use std::collections::HashMap;
use fake::{Dummy, Fake};


enum ArgType {
    Str,
    Int,
    Float,
    DateTime
}

#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
#[readonly::make]
struct APIParam {
    name: str,
    description: str,
    arg_type: ArgType,
    required: bool,
    require_generation: bool,
    generate_by: String,
}

impl APIParam {
    pub fn new(name: &str, description: &str, 
               arg_type: ArgType, required: bool, require_generation: bool,
               generate_by: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            arg_type,
            required,
            require_generation,
            generate_by,
        }  
    }
}