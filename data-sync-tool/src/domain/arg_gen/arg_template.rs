// Argument Generation Domain Definition
// Defines configuration related to dynamic argument generation

// use chrono::prelude::*;
use fake::{Dummy, Fake};
use uuid::Uuid;

// use std::collections::HashMap;

#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
#[readonly::make]
pub struct ArgTemplate {
    id: Uuid,
}

impl ArgTemplate {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
        }
    }
}
