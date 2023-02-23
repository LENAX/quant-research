// APIParams Value Object Definition

use chrono::prelude::*;
use fake::{Dummy, Fake};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use std::collections::HashMap;

// #[derive(Clone, Copy, PartialEq, Eq)]
// enum ArgType {
//     String,
//     Int,
//     Float,
//     DateTime
// }

#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, Setters, MutGetters, CopyGetters)]
pub struct APIParam {
    #[getset(get = "pub", set = "pub")]
    name: String,
    #[getset(get = "pub", set = "pub")]
    description: String,
    #[getset(get = "pub", set = "pub")]
    arg_type: String,
    #[getset(get = "pub", set = "pub")]
    required: bool,
    #[getset(get = "pub", set = "pub")]
    require_generation: bool,
    #[getset(get = "pub", set = "pub")]
    generate_by: String,
}

impl APIParam {
    pub fn new(
        name: &str,
        description: &str,
        arg_type: &str,
        required: bool,
        require_generation: bool,
        generate_by: &str,
    ) -> Self {
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
