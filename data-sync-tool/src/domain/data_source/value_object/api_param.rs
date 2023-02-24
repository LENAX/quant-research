// APIParams Value Object Definition

use std::{error, fmt};

use fake::{Dummy, Fake};
// use fake::{Dummy, Fake};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use lazy_static::lazy_static;
use regex::Regex;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

lazy_static! {
    static ref STRING_ARG_TYPE_PATTERN: Regex = Regex::new("^str|string|String$").unwrap();
    static ref INT_ARG_TYPE_PATTERN: Regex = Regex::new("^int|Integer$").unwrap();
    static ref FLOAT_ARG_TYPE_PATTERN: Regex = Regex::new("^int|Integer$").unwrap();
}

#[derive(Debug, Clone)]
pub struct InvalidArgType;
impl fmt::Display for InvalidArgType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ArgType can only be one of String, Int, and Float!")
    }
}
impl error::Error for InvalidArgType {}


#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
enum ArgType {
    String,
    Int,
    Float,
}

impl TryFrom<String> for ArgType {
    type Error = InvalidArgType;

    fn try_from(value: String) -> std::result::Result<ArgType, InvalidArgType> {
        if STRING_ARG_TYPE_PATTERN.is_match(&value) {
            Ok(ArgType::String)
        } else if INT_ARG_TYPE_PATTERN.is_match(&value) {
            Ok(ArgType::Int)
        } else if FLOAT_ARG_TYPE_PATTERN.is_match(&value) {
            Ok(ArgType::Float)
        } else {
            Err(InvalidArgType)
        }
    }
}

#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, Setters, MutGetters, CopyGetters)]
pub struct APIParam {
    #[getset(get = "pub", set = "pub")]
    name: String,
    #[getset(get = "pub", set = "pub")]
    description: String,
    #[getset(get = "pub", set = "pub")]
    arg_type: Option<ArgType>,
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
    ) -> Result<Self> {
        let param_arg_type = ArgType::try_from(arg_type.to_string())?;

        Ok(Self {
            name: name.to_string(),
            description: description.to_string(),
            arg_type: Some(param_arg_type),
            required,
            require_generation,
            generate_by: generate_by.to_string(),
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
            require_generation: false,
            generate_by: String::from(""),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
