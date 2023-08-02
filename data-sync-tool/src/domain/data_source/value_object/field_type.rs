//! Schema Enum Definition
//! Defines data types supported by the data synchronization tool

use std::{error, fmt, str::FromStr};

#[derive(Debug, Clone)]
pub struct InvalidFieldType;
impl fmt::Display for InvalidFieldType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unrecognized type!")
    }
}
impl error::Error for InvalidFieldType {}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FieldType {
    String,
    Int,
    Float,
    Double,
    Datetime,
    Date,
}

impl FromStr for FieldType {
    type Err = InvalidFieldType;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "str" | "String" | "string" | "STRING" | "Str" | "STR" => Ok(FieldType::String),
            "int" | "Int" | "Integer" | "INT" | "integer" | "i32" | "i64" | "uint" => {
                Ok(FieldType::Int)
            }
            "float" | "Float" | "FLOAT" => Ok(FieldType::Float),
            "double" | "Double" => Ok(FieldType::Double),
            "datetime" | "Datetime" => Ok(FieldType::Datetime),
            "Date" | "date" => Ok(FieldType::Date),
            _ => Err(InvalidFieldType),
        }
    }
}
