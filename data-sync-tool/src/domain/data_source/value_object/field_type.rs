//! Schema Enum Definition
//! Defines data types supported by the data synchronization tool

use std::{error, fmt};

use fake::{Dummy};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref STRING_ARG_TYPE_PATTERN: Regex = Regex::new("^str|string|String$").unwrap();
    static ref INT_ARG_TYPE_PATTERN: Regex = Regex::new("^int|Integer$").unwrap();
    static ref FLOAT_ARG_TYPE_PATTERN: Regex = Regex::new("^int|Integer$").unwrap();
}

#[derive(Debug, Clone)]
pub struct InvalidFieldType;
impl fmt::Display for InvalidFieldType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FieldType can only be one of String, Int, and Float!")
    }
}
impl error::Error for InvalidFieldType {}


#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
pub enum FieldType {
    String,
    Int,
    Float,
}

impl TryFrom<String> for FieldType {
    type Error = InvalidFieldType;

    fn try_from(value: String) -> std::result::Result<FieldType, InvalidFieldType> {
        if STRING_ARG_TYPE_PATTERN.is_match(&value) {
            Ok(FieldType::String)
        } else if INT_ARG_TYPE_PATTERN.is_match(&value) {
            Ok(FieldType::Int)
        } else if FLOAT_ARG_TYPE_PATTERN.is_match(&value) {
            Ok(FieldType::Float)
        } else {
            Err(InvalidFieldType)
        }
    }
}
