// APIParams Value Object Definition

use super::field_type::FieldType;

use fake::{ Fake};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use uuid::Uuid;

use crate::common::errors::Result;

#[derive(Debug,  PartialEq, Eq, Clone, Getters, Setters, MutGetters, CopyGetters)]
pub struct APIParam {
    #[getset(get = "pub")]
    name: String,
    #[getset(get = "pub")]
    description: String,
    #[getset(get = "pub")]
    arg_type: Option<FieldType>,
    #[getset(get = "pub")]
    required: bool,
    #[getset(get = "pub")]
    template_id: Option<Uuid>,
}

impl APIParam {
    pub fn new(
        name: &str,
        description: &str,
        arg_type: &str,
        required: bool,
        template_id: Option<Uuid>,
    ) -> Result<Self> {
        let param_arg_type = FieldType::try_from(arg_type.to_string())?;

        Ok(Self {
            name: name.to_string(),
            description: description.to_string(),
            arg_type: Some(param_arg_type),
            required,
            template_id,
        })
    }
}

impl Default for APIParam {
    fn default() -> Self {
        Self {
            name: String::from("New APIParam"),
            description: String::from("Please write a description."),
            arg_type: None,
            required: false,
            template_id: None,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
