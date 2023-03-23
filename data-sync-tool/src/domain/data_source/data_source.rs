//! Data Source Domain Object Definition

use chrono::prelude::*;
use fake::{Dummy, Fake};
use getset::{CopyGetters, Getters, MutGetters, Setters};
use std::{cell::RefCell, collections::HashMap, error, fmt, sync::Arc};
use uuid::Uuid;

use super::{dataset::Dataset, value_object::local_storage::LocalStorage};

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

// Errors
#[derive(Debug, Clone)]
pub struct UpdateTimeEarlierThanCreationError;
impl fmt::Display for UpdateTimeEarlierThanCreationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Update time should be later than creation time!")
    }
}

#[derive(Debug, Clone)]
pub struct UpdateStatusShouldCoexistWithItsDate;
impl fmt::Display for UpdateStatusShouldCoexistWithItsDate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "If the entity is updated, then it should has an update status."
        )
    }
}

impl error::Error for UpdateStatusShouldCoexistWithItsDate {}
impl error::Error for UpdateTimeEarlierThanCreationError {}

#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, Setters, MutGetters, CopyGetters)]
pub struct DataSource {
    #[getset(get = "pub", set = "pub")]
    id: Uuid,

    #[getset(get = "pub", set = "pub")]
    name: String,

    #[getset(get = "pub", set = "pub")]
    description: String,

    #[getset(get = "pub", set = "pub")]
    api_key: String,

    #[getset(get = "pub", set = "pub")]
    create_date: DateTime<Utc>,

    #[getset(get = "pub")]
    last_update_time: Option<DateTime<Utc>>,

    #[getset(get = "pub", set = "pub")]
    update_successful: Option<bool>, // defaults to true if last_update_time is provided

    #[getset(get = "pub")]
    datasets: HashMap<String, Dataset>,

    #[getset(get = "pub", set = "pub")]
    local_storage: LocalStorage,
}

impl DataSource {
    pub fn new(
        id: Uuid,
        name: &str,
        description: &str,
        api_key: &str,
        create_date: DateTime<Utc>,
        last_update_time: Option<DateTime<Utc>>,
        update_successful: Option<bool>,
        datasets: &[Dataset],
    ) -> Result<Self> {
        let mut id_mapped_datasets: HashMap<String, Dataset> = HashMap::new();

        datasets.into_iter().for_each(|v| {
            let dataset_id = v.id().to_string();
            id_mapped_datasets.insert(dataset_id, v.clone());
        });

        match last_update_time {
            None => {
                if let Some(_) = update_successful {
                    return Err(Box::new(UpdateStatusShouldCoexistWithItsDate));
                } else {
                    return Ok(Self {
                        id,
                        name: name.to_string(),
                        description: description.to_string(),
                        api_key: api_key.to_string(),
                        create_date,
                        last_update_time: None,
                        update_successful: None,
                        datasets: id_mapped_datasets,
                        local_storage: LocalStorage::default(),
                    });
                }
            }
            Some(update_dt) => {
                if let Some(update_ok) = update_successful {
                    return Ok(Self {
                        id,
                        name: name.to_string(),
                        description: description.to_string(),
                        api_key: api_key.to_string(),
                        create_date,
                        last_update_time: Some(update_dt),
                        update_successful: Some(update_ok),
                        datasets: id_mapped_datasets,
                        local_storage: LocalStorage::default()
                    });
                } else {
                    return Ok(Self {
                        id,
                        name: name.to_string(),
                        description: description.to_string(),
                        api_key: api_key.to_string(),
                        create_date,
                        last_update_time: Some(update_dt),
                        update_successful: Some(false),
                        datasets: id_mapped_datasets,
                        local_storage: LocalStorage::default()
                    });
                }
            }
        }
    }

    pub fn set_last_update_time(&mut self, update_dt: DateTime<Utc>) -> Result<&mut Self> {
        if self.create_date > update_dt {
            Err(Box::new(UpdateTimeEarlierThanCreationError))
        } else {
            self.last_update_time = Some(update_dt);
            Ok(self)
        }
    }

    pub fn add_datasets(&mut self, datasets: &Vec<Dataset>) -> Result<&mut Self> {
        for dataset in datasets {
            self.datasets
                .insert(dataset.id().to_string(), dataset.clone());
        }
        return Ok(self);
    }

    pub fn get_datasets_by_ids(
        &self,
        dataset_ids: &Vec<&str>,
    ) -> HashMap<String, Dataset> {
        let mut result_map: HashMap<String, Dataset> = HashMap::new();
        for id in dataset_ids {
            if let Some(matched_ds) = self.datasets.get(*id) {
                result_map.insert(String::from(*id), matched_ds.clone());
            }
        }
        result_map
    }

    pub fn get_datasets_requires_sync(&self) -> HashMap<String, Dataset> {
        let mut datasets_require_sync: HashMap<String, Dataset> = HashMap::new();
        self.datasets
            .iter()
            .filter(|kv| {
                let ds = kv.1.clone();
                let ds_ref = ds;
                let need_sync = *ds_ref.sync_enabled();
                need_sync
            })
            .for_each(|kv| {
                datasets_require_sync.insert(kv.0.to_string(), kv.1.clone());
            });
        return datasets_require_sync;
    }

    pub fn remove_dataset_by_ids(&mut self, dataset_ids: &Vec<String>) -> &mut Self {
        for dataset_id in dataset_ids {
            self.datasets.remove(dataset_id);
        }
        return self;
    }

    pub fn remove_all_datasets(&mut self) -> &mut Self {
        self.datasets.clear();
        return self;
    }

    // TODO: Advanced Features: Automatic Schema Import and Parsing

    // TODO: Advanced Features: Automatic DB Schema Creation

    // TODO: Advanced Features: Automatic Schema Migration
}

impl std::fmt::Display for DataSource {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f,
               "DataSource(id: {},  name: {}, description: {}, api_key: {}, create_date: {}, last_update_time: {:?}, update_successful: {:?}, datasets: {:?})",
               self.id, self.name, self.description, self.api_key, self.create_date.with_timezone(&Local), Some(self.last_update_time),
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
            last_update_time: None,
            update_successful: None,
            datasets: HashMap::new(),
            local_storage: LocalStorage::default()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::DateTime;
    use chrono::Utc;
    use fake::faker::chrono::raw::*;
    use fake::faker::lorem::en::Paragraph;
    use fake::faker::name::en::Name;
    use fake::locales::ZH_CN;
    use fake::uuid::UUIDv4;
    use fake::{Fake, Faker};
    use rand::seq::SliceRandom;

    #[test]
    fn it_should_create_an_empty_datasource() {
        let id: Uuid = UUIDv4.fake();
        let name: String = Name().fake();
        let description: String = Paragraph(3..5).fake();
        let api_key = Faker.fake::<String>();
        let create_date: chrono::DateTime<Utc> = DateTimeBefore(ZH_CN, Utc::now()).fake();
        let last_update_time: Option<DateTime<Utc>> = None;
        let update: Option<bool> = None;
        let datasets: Vec<Dataset> = vec![];

        let empty_datasource = DataSource::new(
            id,
            &name,
            &description,
            &api_key,
            create_date,
            last_update_time,
            update,
            datasets.as_slice()
        )
        .expect("Update status and update datetime should coexist in a new DataSource object.");

        // test field access
        assert_eq!(empty_datasource.id, id);
        assert_eq!(empty_datasource.name, name);
        assert_eq!(empty_datasource.description, description);
        assert_eq!(empty_datasource.api_key, api_key);
        assert_eq!(empty_datasource.create_date, create_date);
        assert_eq!(empty_datasource.last_update_time, None);
        assert_eq!(empty_datasource.update_successful, None);
        assert_eq!(empty_datasource.datasets.len(), 0);
    }

    #[test]
    fn its_getters_should_return_the_same_value_as_fields() {
        let fake_datasource: DataSource = Faker.fake();

        // test getters
        assert_eq!(*fake_datasource.id(), fake_datasource.id);
        assert_eq!(*fake_datasource.name(), fake_datasource.name);
        assert_eq!(*fake_datasource.description(), fake_datasource.description);
        assert_eq!(*fake_datasource.api_key(), fake_datasource.api_key);
        assert_eq!(*fake_datasource.create_date(), fake_datasource.create_date);
        assert_eq!(
            *fake_datasource.last_update_time(),
            fake_datasource.last_update_time
        );
        assert_eq!(
            *fake_datasource.update_successful(),
            fake_datasource.update_successful
        );
        assert_eq!(*fake_datasource.datasets(), fake_datasource.datasets);
    }

    #[test]
    #[should_panic]
    fn it_should_panic_if_attempts_to_set_an_invalid_update_time() {
        let mut fake_datasource: DataSource = Faker.fake();
        let target_update_date: DateTime<Utc> =
            DateTimeBefore(ZH_CN, fake_datasource.create_date).fake();

        assert!(fake_datasource.create_date > target_update_date);
        println!(
            "Set an invalid update time should panic. create_time: {:?}, update_time: {:?}",
            fake_datasource.create_date.with_timezone(&Local),
            target_update_date.with_timezone(&Local)
        );

        fake_datasource
            .set_last_update_time(target_update_date)
            .unwrap();
    }

    #[test]
    fn it_should_add_datasets_as_expected() {
        let mut datasource = DataSource::default();

        let test_datasets = vec![
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
        ];

        let updated_datasource = datasource
            .add_datasets(&test_datasets)
            .expect("Failed to borrow field dataset");

        let all_dataset_matched = test_datasets
            .as_slice()
            .into_iter()
            .map(|ds| {
                let dataset_id = ds.id().to_string();
                let matched_ds = updated_datasource.datasets.get(&dataset_id);

                match matched_ds {
                    Some(d) => d == ds,
                    None => false,
                }
            })
            .fold(true, |compare_result, val| compare_result && val);
        println!("All matched: {}", all_dataset_matched);
        assert!(all_dataset_matched);
    }

    #[test]
    fn it_should_clear_datasets_as_expected() {
        let mut fake_datasource: DataSource = Faker.fake();
        let test_datasets = vec![
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
        ];

        fake_datasource.add_datasets(&test_datasets).unwrap();
        assert!(fake_datasource.datasets.len() > 0);

        fake_datasource.remove_all_datasets();
        assert!(fake_datasource.datasets.is_empty());
    }

    #[test]
    fn it_should_get_datasets_by_ids_as_expected() {
        let mut fake_datasource: DataSource = Faker.fake();
        let test_datasets = vec![
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
        ];

        fake_datasource.add_datasets(&test_datasets).unwrap();
        let mut rng = rand::thread_rng();
        let dataset_ids: Vec<&str> = fake_datasource
            .datasets()
            .keys()
            .map(|k| k.as_str())
            .collect();
        let sample_ids: Vec<_> = dataset_ids.choose_multiple(&mut rng, 3).collect();

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
    fn it_should_get_datasets_require_sync_as_expected() {
        let mut fake_datasource: DataSource = Faker.fake();
        let test_datasets = vec![
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
        ];

        fake_datasource.remove_all_datasets();
        fake_datasource.add_datasets(&test_datasets).unwrap();

        test_datasets[..=3].into_iter().for_each(|d| {
            let dataset_id = d.id().to_string();

            let mut dataset = fake_datasource.get_datasets_by_ids(&vec![&dataset_id]);
            let ds = dataset.get_mut(&dataset_id).unwrap();
            ds.set_sync_enabled(true);
        });

        println!("test_datasets[..3] has {} elements.", test_datasets[..=3].len());

        test_datasets[4..].into_iter().for_each(|d| {
            let dataset_id = d.id().to_string();

            let mut dataset = fake_datasource.get_datasets_by_ids(&vec![&dataset_id]);
            let ds = dataset.get_mut(&dataset_id).unwrap();
            // dataset./get
            ds.set_sync_enabled(false);
        });

        println!("test_datasets[4..] has {} elements.", test_datasets[4..].len());

        let sync_status_of_datasets: Vec<bool> = test_datasets
            .iter()
            .map(|d| *d.sync_enabled())
            .collect();

        println!("test_datasets sync_enabled: {:?}", sync_status_of_datasets);

        let datasets_sync_enabled = fake_datasource.get_datasets_requires_sync();
        // FIXME: could sometimes fail!
        assert_eq!(datasets_sync_enabled.len(), 4);
    }

    #[test]
    fn it_should_remove_datasets_as_expected() {
        let mut fake_datasource: DataSource = Faker.fake();
        fake_datasource.remove_all_datasets();
        let test_datasets = vec![
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
            Faker.fake::<Dataset>(),
        ];
        fake_datasource.add_datasets(&test_datasets).unwrap();
        let dataset_ids: Vec<String> = test_datasets[0..3]
            .into_iter()
            .map(|ds| ds.id().to_string())
            .collect();
        fake_datasource.remove_dataset_by_ids(&dataset_ids);
        assert_eq!(fake_datasource.datasets.len(), 4);

        for id in dataset_ids {
            assert!(!fake_datasource.datasets.contains_key(&id));
        }
    }
}
