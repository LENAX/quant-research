use chrono::{Local};
// use chrono::DateTime;
// use chrono::Local;
mod infrastructure;
mod application;
mod domain;
mod presentation;
// mod services;

use log::{info, trace, warn, error};

use env_logger;

fn main() {
    env_logger::init();
    log::warn!("warn");
    log::info!("info");
    log::debug!("debug");
    info!("Print an info!");
    warn!("This is a warning!");
    error!("Oh no! An error occurred!");
}
