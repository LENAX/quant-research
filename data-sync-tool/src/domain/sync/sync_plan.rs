// Sync Management Domain Object Definition
// Defines sync related configuration

// use chrono::prelude::*;
use fake::{Dummy, Fake};
use uuid::Uuid;
// use std::collections::HashMap;

#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
#[readonly::make]
pub struct SyncPlan {
    id: Uuid,
}

impl SyncPlan {
    pub fn new(id: Uuid) -> Self {
        Self {
            id
        }
    }
}
