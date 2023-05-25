use chrono::{Local};
use fake::faker::name::en::*;
use fake::uuid::UUIDv4;
use fake::{ Fake, Faker};
use rand::rngs::StdRng;
use rand::SeedableRng;
use uuid::Uuid;
// use fake::locales::{EN, ZH_CN};
use fake::locales::{EN, FR_FR, ZH_CN};
// use fake::Fake;
use fake::faker::chrono::raw::*;
// use chrono::DateTime;
// use chrono::Local;
// use fake::locales::*;
// use fake::locales::ZH_CN;
// use fake::faker::name::zh_cn::Name;
mod infrastructure;
mod application;
mod domain;
mod presentation;
// mod services;


fn main() {
    
}
