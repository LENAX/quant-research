// Data Source Domain Object Definition

use chrono::prelude::*;
use uuid::Uuid;
use std::{collections::HashMap, cell::RefCell, rc::Rc};
use fake::{Dummy, Fake};
use getset::{CopyGetters, Getters, MutGetters, Setters};

use super::dataset::Dataset;

#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, Setters, MutGetters, CopyGetters)]
#[readonly::make]
pub struct DataSource {
    #[getset(get, set, get_mut)]
    id: Rc<RefCell<Uuid>>,
    #[getset(get, set, get_mut)]
    name: Rc<RefCell<String>>,
    #[getset(get, set, get_mut)]
    description: Rc<RefCell<String>>,
    #[getset(get, set, get_mut)]
    api_key: Rc<RefCell<String>>,
    #[getset(get, set, get_mut)]
    create_date: Rc<RefCell<DateTime<Utc>>>, // fixme, Local is not compatible with Dummy
    #[getset(get, set, get_mut)]
    last_update: Option<Rc<RefCell<DateTime<Utc>>>>,
    #[getset(get, set, get_mut)]
    update_successful: Option<Rc<RefCell<bool>>>,
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
                    id: Rc::new(RefCell::new(id)),
                    name: Rc::new(RefCell::new(name.to_string())),
                    description: Rc::new(RefCell::new(description.to_string())),
                    api_key: Rc::new(RefCell::new(api_key.to_string())),
                    create_date: Rc::new(RefCell::new(create_date)),
                    last_update: Some(Rc::new(RefCell::new(update_dt))),
                    update_successful: Some(Rc::new(RefCell::new(update_ok))),
                    datasets: datasets
                }
            } else {
                Self {
                    id: Rc::new(RefCell::new(id)),
                    name: Rc::new(RefCell::new(name.to_string())),
                    description: Rc::new(RefCell::new(description.to_string())),
                    api_key: Rc::new(RefCell::new(api_key.to_string())),
                    create_date: Rc::new(RefCell::new(create_date)),
                    last_update: Some(Rc::new(RefCell::new(update_dt))),
                    update_successful: None,
                    datasets: datasets
                }
            }
            
        } else {
            Self {
                id: Rc::new(RefCell::new(id)),
                name: Rc::new(RefCell::new(name.to_string())),
                description: Rc::new(RefCell::new(description.to_string())),
                api_key: Rc::new(RefCell::new(api_key.to_string())),
                create_date: Rc::new(RefCell::new(create_date)),
                last_update: None,
                update_successful: None,
                datasets: datasets
            }
        }
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
            id,&name, &description, &api_key, create_date, last_update, update, datasets.clone());
        
        // test field access
        assert_eq!(*empty_datasource.id.borrow(), id);
        assert_eq!(*empty_datasource.name.borrow(), name);
        assert_eq!(*empty_datasource.description.borrow(), description);
        assert_eq!(*empty_datasource.api_key.borrow(), api_key);
        assert_eq!(*empty_datasource.create_date.borrow(), create_date);
        assert_eq!(empty_datasource.last_update, None);
        assert_eq!(empty_datasource.update_successful, None);
        assert_eq!(*empty_datasource.datasets.borrow(), *datasets.borrow());
    }

    #[test]
    fn getters_should_return_the_same_value_as_fields() {
        let fake_datasource: DataSource = Faker.fake();
        println!("{:?}", fake_datasource);
        println!("{:?}", *fake_datasource.create_date().borrow());
        println!("{:?}", *fake_datasource.last_update());
        println!("{:?}", *fake_datasource.update_successful());
        
        // test getters
        assert_eq!(*fake_datasource.id().borrow(), *fake_datasource.id.borrow());
        assert_eq!(*fake_datasource.name().borrow(), *fake_datasource.name.borrow());
        assert_eq!(*fake_datasource.description().borrow(), *fake_datasource.description.borrow());
        assert_eq!(*fake_datasource.api_key().borrow(), *fake_datasource.api_key.borrow());
        assert_eq!(*fake_datasource.create_date().borrow(), *fake_datasource.create_date.borrow());
        assert_eq!(*fake_datasource.last_update(), fake_datasource.last_update);
        assert_eq!(*fake_datasource.update_successful(), fake_datasource.update_successful);
        assert_eq!(*fake_datasource.datasets().borrow(), *fake_datasource.datasets.borrow());
    }

    #[test]
    fn its_setters_should_modify_fields_as_expected() {
        let mut fake_datasource: DataSource = Faker.fake();
        // let target_data

        println!("Before update:\n{:?}", fake_datasource);
        fake_datasource.set_name(Rc::new(RefCell::new("fake datasource".to_string())));
        // fake_datasource.set_name(String::from("Fake data"));


    }

}
