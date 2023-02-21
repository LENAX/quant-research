// Data Source Domain Object Definition

use chrono::prelude::*;
use uuid::Uuid;
use std::{collections::HashMap, cell::RefCell, rc::Rc};
use fake::{Dummy, Fake};
use getset::{CopyGetters, Getters, MutGetters, Setters};

use super::dataset::Dataset;

type Result<T> = std::result::Result<T, UpdateTimeEarlierThanCreationError>;

#[derive(Debug, Clone)]
pub struct UpdateTimeEarlierThanCreationError;

#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, Setters, MutGetters, CopyGetters)]
#[readonly::make]
pub struct DataSource {
    #[getset(get, set, get_mut)]
    id: Uuid,
    #[getset(get, set, get_mut)]
    name: String,
    #[getset(get, set, get_mut)]
    description: String,
    #[getset(get, set, get_mut)]
    api_key: String,
    #[getset(get, set, get_mut)]
    create_date: DateTime<Utc>, // fixme, Local is not compatible with Dummy
    #[getset(get, get_mut)]
    last_update: Option<DateTime<Utc>>,
    #[getset(get, set, get_mut)]
    update_successful: Option<bool>,
    #[getset(get, set, get_mut)]
    datasets: Rc<RefCell<HashMap<String, Dataset>>>
}

impl DataSource {
    pub fn new(id: Uuid, name: &str, description: &str, 
               api_key: &str, create_date: DateTime<Utc>,
               last_update: Option<DateTime<Utc>>, update_successful: Option<bool>,
               datasets: Rc<RefCell<HashMap<String, Dataset>>>) -> Self {
        if let Some(update_dt) = last_update {
            if let Some(update_ok) = update_successful {
                Self {
                    id,
                    name: name.to_string(),
                    description: description.to_string(),
                    api_key: api_key.to_string(),
                    create_date: create_date,
                    last_update: Some(update_dt),
                    update_successful: Some(update_ok),
                    datasets: datasets.clone()
                }
            } else {
                Self {
                    id: id,
                    name: name.to_string(),
                    description: description.to_string(),
                    api_key: api_key.to_string(),
                    create_date: create_date,
                    last_update: Some(update_dt),
                    update_successful: None,
                    datasets: datasets.clone()
                }
            }
            
        } else {
            Self {
                id: id,
                name: name.to_string(),
                description: description.to_string(),
                api_key: api_key.to_string(),
                create_date: create_date,
                last_update: None,
                update_successful: None,
                datasets: datasets.clone()
            }
        }
    }

    pub fn set_last_update(&mut self, update_dt: DateTime<Utc>) -> Result<&mut Self> {
        if self.create_date > update_dt {
            Err(UpdateTimeEarlierThanCreationError)
        } else {
            self.last_update = Some(update_dt);
            Ok(self)
        }
    }


}

impl std::fmt::Display for DataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,
               "DataSource(id: {},  name: {}, description: {}, api_key: {}, create_date: {}, last_update: {:?}, update_successful: {:?}, datasets: {:?})",
               self.id, self.name, self.description, self.api_key, self.create_date.with_timezone(&Local), Some(self.last_update),
               Some(self.update_successful), self.datasets)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    // use fake::faker::chrono::zh_cn::DateTimeBefore;
    use fake::faker::lorem::en::Paragraph;
    use fake::faker::name::en::Name;
    use fake::locales::ZH_CN;
    use chrono::Utc;
    use fake::faker::chrono::raw::*;
    use fake::{Fake, Faker};
    use fake::uuid::UUIDv4;
    use chrono::DateTime;

    #[test]
    fn it_should_create_an_empty_datasource() {
        let id = UUIDv4.fake();
        let name: String = Name().fake();
        let description: String = Paragraph(3..5).fake();
        let api_key = Faker.fake::<String>();
        let create_date: chrono::DateTime<Utc> = DateTimeBefore(ZH_CN, Utc::now()).fake();
        let last_update: Option<DateTime<Utc>> = None;
        let update: Option<bool> = None;
        let datasets: Rc<RefCell<HashMap<String, Dataset>>> = Rc::new(RefCell::new(HashMap::new()));

        let empty_datasource = DataSource::new(
            id, &name, &description, &api_key, create_date, last_update, update, datasets.clone());
        
        // test field access
        assert_eq!(empty_datasource.id, id);
        assert_eq!(empty_datasource.name, name);
        assert_eq!(empty_datasource.description, description);
        assert_eq!(empty_datasource.api_key, api_key);
        assert_eq!(empty_datasource.create_date, create_date);
        assert_eq!(empty_datasource.last_update, None);
        assert_eq!(empty_datasource.update_successful, None);
        assert_eq!(*empty_datasource.datasets.borrow(), *datasets.borrow());

        println!("Empty datasets: {:?}", *datasets.borrow())
    }

    #[test]
    fn getters_should_return_the_same_value_as_fields() {
        let fake_datasource: DataSource = Faker.fake();
        println!("{}", fake_datasource);
        println!("{:?}", fake_datasource.create_date().with_timezone(&Local));
        println!("{:?}", fake_datasource.last_update());
        println!("{:?}", fake_datasource.update_successful());
        println!("{:?}", *(fake_datasource.datasets().borrow()));
        
        // test getters
        assert_eq!(*fake_datasource.id(), fake_datasource.id);
        assert_eq!(*fake_datasource.name(), fake_datasource.name);
        assert_eq!(*fake_datasource.description(), fake_datasource.description);
        assert_eq!(*fake_datasource.api_key(), fake_datasource.api_key);
        assert_eq!(*fake_datasource.create_date(), fake_datasource.create_date);
        assert_eq!(*fake_datasource.last_update(), fake_datasource.last_update);
        assert_eq!(*fake_datasource.update_successful(), fake_datasource.update_successful);
        assert_eq!(*(fake_datasource.datasets().borrow()), *fake_datasource.datasets.borrow());
    }

    #[test]
    fn its_setters_should_modify_fields_as_expected() {
        let mut fake_datasource: DataSource = Faker.fake();
        let target_name = "fake datasource".to_string();
        let target_description = "blah".to_string();
        let target_api_key = "1q2w3e4r5t6y".to_string();
        let target_create_date = DateTimeBefore(ZH_CN, Utc::now()).fake();
        let target_last_update = chrono::offset::Utc::now();
        let datasets_ref = fake_datasource.datasets.clone();
        let target_data = DataSource::new(
            fake_datasource.id, &target_name,
            &target_description, &target_api_key,
            target_create_date, Some(target_last_update), Some(true),
            fake_datasource.datasets.clone()
        );

        println!("Before update:\n{:?}\n", fake_datasource);
        fake_datasource.set_name(target_name)
                       .set_description(target_description)
                       .set_api_key(target_api_key)
                       .set_create_date(target_create_date)
                       .set_last_update(target_last_update)
                       .expect("Update time should be later than create time.")
                       .set_update_successful(Some(true))
                       .set_datasets(datasets_ref);

        println!("Expect to become:\n{}\n", target_data);
        println!("After update:\n{}", fake_datasource);
        assert_eq!(fake_datasource, target_data);
    }

    #[test]
    #[should_panic]
    fn it_should_panic_if_attempts_to_set_an_invalid_update_time() {
        let mut fake_datasource: DataSource = Faker.fake();
        let target_update_date: DateTime<Utc>  = DateTimeBefore(ZH_CN, fake_datasource.create_date).fake();

        assert!(fake_datasource.create_date > target_update_date);
        println!("Set an invalid update time should panic. create_time: {:?}, update_time: {:?}",
                 fake_datasource.create_date.with_timezone(&Local),
                 target_update_date.with_timezone(&Local));
        
        fake_datasource.set_last_update(target_update_date).unwrap();
    }

}
