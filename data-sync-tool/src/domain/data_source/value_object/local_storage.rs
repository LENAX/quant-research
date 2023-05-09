//! Local Storage Value Object
//! 
use getset::{Getters, Setters};

#[derive(Getters, Setters, Default)]
#[getset(get, set, get_mut)]
pub struct LocalStorage {
    host: String,
    port: String,
    username: String,
    password: String
}

impl LocalStorage {
    pub fn new(host: &str, port: &str, username: &str, password: &str) -> Self {
        Self {
            host: host.to_string(),
            port: port.to_string(),
            username: username.to_string(),
            password: password.to_string()
        }
    }
}
