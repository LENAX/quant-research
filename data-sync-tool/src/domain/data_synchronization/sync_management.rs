// Sync Management Domain Object Definition
// Defines sync related configuration

// use chrono::prelude::*;
use fake::{Dummy, Fake};
use uuid::Uuid;
// use std::collections::HashMap;

#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
#[readonly::make]
pub struct SyncManagement {
    id: Uuid,
}

impl SyncManagement {
    pub fn new(id: Uuid) -> Self {
        Self {
            id
        }
    }
}
