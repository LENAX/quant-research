// Argument Generation Domain Definition
//! Defines configuration related to dynamic argument generation
//! This domain object describes arg generators
//! Arg Generators have these categories: ExternalDataGenerator, AlgorithmicGenerator
//! ExternalDataGenerator loads some data from some data source and provides the loaded data as generated arguments
//! AlgorithmicGenerator is a type of arg generators that generate arguments during runtime using some algorithms
//! For example, we can have a datetime_generator object that implements AlgorithmicGenerator trait to generate datetime values

// use chrono::prelude::*;
use fake::{ Fake};
use uuid::Uuid;

// use std::collections::HashMap;

#[derive(Debug,  PartialEq, Eq, Clone)]
#[readonly::make]
pub struct Template {
    id: Uuid,
}

impl Template {
    pub fn new(id: Uuid) -> Self {
        Self { id }
    }
}
