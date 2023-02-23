//! Data Source Domain Object Definition

use chrono::prelude::*;
use uuid::Uuid;
use std::{collections::HashMap, cell::{RefCell, BorrowMutError, Ref}, rc::Rc};
use fake::{Dummy, Fake};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use derivative::Derivative;

use super::dataset::Dataset;

// type Result<T> = std::result::Result<T, UpdateTimeEarlierThanCreationError>;

// Errors
#[derive(Debug, Clone)]
pub struct UpdateTimeEarlierThanCreationError;

#[derive(Debug, Clone)]
pub struct UpdateStatusShouldCoexistWithItsDate;

#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, Setters, MutGetters, CopyGetters)]
pub struct DataSource {
    #[getset(get, set)]
    id: Uuid,

    #[getset(get, set)]
    name: String,

    #[getset(get, set)]
    description: String,

    #[getset(get, set)]
    api_key: String,
    
    #[getset(get, set)]
    create_date: DateTime<Utc>,
    
    #[getset(get)]
    last_update: Option<DateTime<Utc>>, 
   
    #[getset(get, set)]
    update_successful: Option<bool>, // defaults to true if last_update is provided
    
    #[getset(get)]
    datasets: HashMap<String, Rc<RefCell<Dataset>>>
}

impl DataSource {
    pub fn new(id: Uuid, name: &str, description: &str, 
               api_key: &str, create_date: DateTime<Utc>,
               last_update: Option<DateTime<Utc>>, update_successful: Option<bool>,
               datasets: &[Rc<RefCell<Dataset>>]) -> Result<Self, UpdateStatusShouldCoexistWithItsDate> {
        let mut id_mapped_datasets: HashMap<String, Rc<RefCell<Dataset>>> = HashMap::new();
        
        datasets.into_iter()
                .for_each(|v| {
                    let dataset_id = v.as_ref().borrow().id.to_string();
                    id_mapped_datasets.insert(dataset_id, v.clone());
                });
        
        match last_update {
            None => {
                if let Some(_) = update_successful {
                    return Err(UpdateStatusShouldCoexistWithItsDate);
                    
                } else {
                    return Ok(Self {
                            id,
                            name: name.to_string(),
                            description: description.to_string(),
                            api_key: api_key.to_string(),
                            create_date,
                            last_update: None,
                            update_successful: None,
                            datasets: id_mapped_datasets
                        });
                }
            },
            Some(update_dt) => {
                if let Some(update_ok) = update_successful {
                    return Ok(Self {
                            id,
                            name: name.to_string(),
                            description: description.to_string(),
                            api_key: api_key.to_string(),
                            create_date,
                            last_update: Some(update_dt),
                            update_successful: Some(update_ok),
                            datasets: id_mapped_datasets
                        });
                    
                } else {
                    return Ok(Self {
                            id,
                            name: name.to_string(),
                            description: description.to_string(),
                            api_key: api_key.to_string(),
                            create_date,
                            last_update: Some(update_dt),
                            update_successful: Some(false),
                            datasets: id_mapped_datasets
                        });
                }
            }
        }
    }

    pub fn set_last_update(&mut self, update_dt: DateTime<Utc>) -> Result<&mut Self, UpdateTimeEarlierThanCreationError> {
        if self.create_date > update_dt {
            Err(UpdateTimeEarlierThanCreationError)
        } else {
            self.last_update = Some(update_dt);
            Ok(self)
        }
    }

    pub fn add_datasets(&mut self, datasets: &Vec<Rc<RefCell<Dataset>>>) -> Result<&mut Self, BorrowMutError> {
        for dataset in datasets {
            self.datasets.insert(dataset.borrow().id.to_string(), dataset.clone());
        }
        return Ok(self)
    }

    pub fn get_datasets_by_ids(&self, dataset_ids: &Vec<&str>) -> HashMap<String, Rc<RefCell<Dataset>>> {
        let mut result_map: HashMap<String, Rc<RefCell<Dataset>>> = HashMap::new();
        for id in dataset_ids {
            if let Some(matched_ds) = self.datasets.get(*id) {
                result_map.insert(String::from(*id), matched_ds.clone());
            }
        }
        result_map
    }

    pub fn remove_dataset_by_ids(&mut self, dataset_ids: &Vec<String>) -> &mut Self {
        for dataset_id in dataset_ids {
            self.datasets.remove(dataset_id);
        }
        return self
    }

    pub fn remove_all_datasets(&mut self) -> &mut Self {
        self.datasets.clear();
        return self;
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

impl Default for DataSource {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::from("New DataSource"),
            description: String::from("Please write a description."),
            api_key: String::from(""),
            create_date: chrono::offset::Utc::now(),
            last_update: None,
            update_successful: None,
            datasets: HashMap::new()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use fake::faker::lorem::en::Paragraph;
    use fake::faker::name::en::Name;
    use fake::locales::ZH_CN;
    use chrono::Utc;
    use fake::faker::chrono::raw::*;
    use fake::{Fake, Faker};
    use fake::uuid::UUIDv4;
    use chrono::DateTime;
    use rand::seq::SliceRandom;

    #[test]
    fn it_should_create_an_empty_datasource() {
        let id = UUIDv4.fake();
        let name: String = Name().fake();
        let description: String = Paragraph(3..5).fake();
        let api_key = Faker.fake::<String>();
        let create_date: chrono::DateTime<Utc> = DateTimeBefore(ZH_CN, Utc::now()).fake();
        let last_update: Option<DateTime<Utc>> = None;
        let update: Option<bool> = None;
        let datasets: Vec<Rc<RefCell<Dataset>>> = vec![];

        let empty_datasource = DataSource::new(
            id, &name, &description, &api_key, create_date, last_update, 
            update, datasets.as_slice())
            .expect("Update status and update datetime should coexist in a new DataSource object.");
        
        // test field access
        assert_eq!(empty_datasource.id, id);
        assert_eq!(empty_datasource.name, name);
        assert_eq!(empty_datasource.description, description);
        assert_eq!(empty_datasource.api_key, api_key);
        assert_eq!(empty_datasource.create_date, create_date);
        assert_eq!(empty_datasource.last_update, None);
        assert_eq!(empty_datasource.update_successful, None);
        assert_eq!(empty_datasource.datasets.len(), 0);
    }

    #[test]
    fn getters_should_return_the_same_value_as_fields() {
        let fake_datasource: DataSource = Faker.fake();
        
        // test getters
        assert_eq!(*fake_datasource.id(), fake_datasource.id);
        assert_eq!(*fake_datasource.name(), fake_datasource.name);
        assert_eq!(*fake_datasource.description(), fake_datasource.description);
        assert_eq!(*fake_datasource.api_key(), fake_datasource.api_key);
        assert_eq!(*fake_datasource.create_date(), fake_datasource.create_date);
        assert_eq!(*fake_datasource.last_update(), fake_datasource.last_update);
        assert_eq!(*fake_datasource.update_successful(), fake_datasource.update_successful);
        assert_eq!(*fake_datasource.datasets(), fake_datasource.datasets);
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

    #[test]
    fn it_should_add_datasets_as_expected() {
        let mut datasource = DataSource::default();

        let test_datasets= vec![
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
        ];

        let updated_datasource = datasource
            .add_datasets(&test_datasets).expect("Failed to borrow field dataset");
        
        let all_dataset_matched = test_datasets
            .as_slice()
            .into_iter()
            .map(|ds| {
                let dataset_id = (*ds.as_ref()).borrow().id.to_string();
                let matched_ds = updated_datasource.datasets.get(&dataset_id);

                match matched_ds {
                    Some(d) => {
                        *(*d.as_ref()).borrow() == *(*ds.as_ref()).borrow()
                    },
                    None => false
                }
            })
            .fold(true, |compare_result, val| compare_result && val);
        println!("All matched: {}", all_dataset_matched);

    }

    #[test]
    fn it_should_clear_datasets_as_expected() {
        let mut fake_datasource: DataSource = Faker.fake();
        let test_datasets= vec![
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
        ];

        fake_datasource.add_datasets(&test_datasets).unwrap();
        assert!(fake_datasource.datasets.len() > 0);

        fake_datasource.remove_all_datasets();
        assert!(fake_datasource.datasets.is_empty());
    }

    #[test]
    fn it_should_get_datasets_by_ids_as_expected() {
        let mut fake_datasource: DataSource = Faker.fake();
        let test_datasets= vec![
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
        ];

        fake_datasource.add_datasets(&test_datasets).unwrap();
        let mut rng = rand::thread_rng();
        let dataset_ids: Vec<&str> = fake_datasource.datasets().keys().map(|k| k.as_str()).collect();
        let sample_ids: Vec<_> = dataset_ids.choose_multiple (&mut rng, 3).collect();

        if sample_ids.len() > 0 {
            let fetched_datasets = fake_datasource.get_datasets_by_ids(&dataset_ids);
            for id in dataset_ids {
                assert!(fetched_datasets.contains_key(id));
            }
        } else {
            println!("Not enough samples.")
        }
    }

    #[test]
    fn it_should_remove_datasets_as_expected() {
        let mut fake_datasource: DataSource = Faker.fake();
        fake_datasource.remove_all_datasets();
        let test_datasets= vec![
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
            Rc::new(RefCell::new(Faker.fake::<Dataset>())),
        ];
        fake_datasource.add_datasets(&test_datasets).unwrap();
        let dataset_ids: Vec<String> = test_datasets[0..3]
            .into_iter()
            .map(|ds| {
                (*ds.as_ref()).borrow().id.to_string()
            })
            .collect();
        fake_datasource.remove_dataset_by_ids(&dataset_ids);
        assert_eq!(fake_datasource.datasets.len(), 4);

        for id in dataset_ids {
            assert!(!fake_datasource.datasets.contains_key(&id));
        }
    }

}
