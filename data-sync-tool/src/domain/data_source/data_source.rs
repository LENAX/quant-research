// Data Source Domain Object Definition

use chrono::prelude::*;
use uuid::Uuid;
use std::collections::HashMap;
use fake::{Dummy, Fake};

use super::dataset::Dataset;

#[derive(Debug, Dummy, PartialEq, Eq, Clone)]
#[readonly::make]
pub struct DataSource {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub api_key: String,
    pub create_date: DateTime<Local>,
    pub last_update: Option<DateTime<Local>>,
    pub update_successful: Option<bool>,
    pub datasets: HashMap<String, Dataset>
}

impl DataSource {
    pub fn new(id: Uuid, name: &str, description: &str, 
               api_key: &str, create_date: DateTime<Local>,
               last_update: Option<DateTime<Local>>, update_successful: Option<bool>,
               datasets: HashMap<String, Dataset>) -> Self {
        Self {
            id,
            name: name.to_string(),
            description: description.to_string(),
            api_key: description.to_string(),
            create_date,
            last_update,
            update_successful,
            datasets
        }  
    }
}

#[cfg(test)]
mod test {
    use super::*;
    // use fake::faker::chrono::zh_cn::DateTimeBefore;
    use fake::faker::lorem::en::{Paragraphs, Words};
    use fake::locales::ZH_CN;
    use chrono::Utc;
    use fake::faker::chrono::raw::*;
    use fake::{Fake, Faker};
    use fake::uuid::UUIDv4;
    use chrono::DateTime;

    #[test]
    fn it_should_create_an_empty_datasource() {
        let id = UUIDv4.fake();
        let name = Words(5..10).fake();
        let description = Paragraphs(10..100).fake();
        let api_key = Faker.fake::<String>();
        let create_date: chrono::DateTime<Utc> = DateTimeBefore(ZH_CN, Utc::now()).fake();
        let last_update: Option<DateTime<Local>> = None;
        let update: Option<bool> = None;
        let datasets: HashMap<String, Dataset> = HashMap::new();

        let empty_datasource = DataSource::new(
            id,&name, &description, &api_key, create_date, last_update, update, datasets);

        assert_eq!(empty_datasource.id, id);
    }

}
