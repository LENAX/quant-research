// Dataset Domain Object Definition
// Dataset describes the schema, the api endpoint, and the parameters to request for data

use super::{
    data_source::{UpdateStatusShouldCoexistWithItsDate, UpdateTimeEarlierThanCreationError},
    value_object::{
        api_param::APIParam,
        data_schema::{Column, DataSchema},
    },
};
use chrono::prelude::*;
use fake::{Dummy, Fake};
use getset::{Getters, Setters};
use std::{
    cell::{BorrowMutError, RefCell},
    collections::HashMap,
    rc::Rc,
};
use uuid::Uuid;

#[derive(Debug, Dummy, PartialEq, Eq, Clone, Getters, Setters)]
#[readonly::make]
pub struct Dataset {
    #[getset(get = "pub", set = "pub")]
    id: Uuid,
    #[getset(get = "pub", set = "pub")]
    name: String,
    #[getset(get = "pub", set = "pub")]
    description: String,
    #[getset(get = "pub", set = "pub")]
    endpoint: String, // web endpoint of this api_param
    #[getset(get = "pub")]
    api_params: HashMap<String, Rc<RefCell<APIParam>>>, // a hashmap of api arguments
    #[getset(get = "pub", get_mut = "pub")]
    schema: Rc<RefCell<DataSchema>>, // schema of this api_param
    #[getset(get = "pub", set = "pub")]
    create_date: DateTime<Utc>,
    #[getset(get = "pub")]
    last_update: Option<DateTime<Utc>>,
    #[getset(get = "pub", set = "pub")]
    update_successful: Option<bool>,
    #[getset(get = "pub", set = "pub")]
    sync_enabled: bool,
}

impl Dataset {
    pub fn new(
        id: Uuid,
        name: &str,
        description: &str,
        endpoint: &str,
        api_params: &Vec<Rc<RefCell<APIParam>>>,
        schema: DataSchema,
        create_date: DateTime<Utc>,
        last_update: Option<DateTime<Utc>>,
        update_successful: Option<bool>,
        sync_enabled: bool,
    ) -> Result<Self, UpdateStatusShouldCoexistWithItsDate> {
        let mut params_map: HashMap<String, Rc<RefCell<APIParam>>> = HashMap::new();
        api_params.into_iter().for_each(|p| {
            params_map.insert(p.as_ref().borrow().name().clone(), p.clone());
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
                        endpoint: endpoint.to_string(),
                        api_params: params_map,
                        schema: Rc::new(RefCell::new(schema)),
                        create_date,
                        last_update: None,
                        update_successful: None,
                        sync_enabled,
                    });
                }
            }
            Some(update_dt) => {
                if let Some(update_ok) = update_successful {
                    return Ok(Self {
                        id,
                        name: name.to_string(),
                        description: description.to_string(),
                        endpoint: endpoint.to_string(),
                        api_params: params_map,
                        schema: Rc::new(RefCell::new(schema)),
                        create_date,
                        last_update: Some(update_dt),
                        update_successful: Some(update_ok),
                        sync_enabled,
                    });
                } else {
                    return Ok(Self {
                        id,
                        name: name.to_string(),
                        description: description.to_string(),
                        endpoint: endpoint.to_string(),
                        api_params: params_map,
                        schema: Rc::new(RefCell::new(schema)),
                        create_date,
                        last_update: Some(update_dt),
                        update_successful: Some(false),
                        sync_enabled,
                    });
                }
            }
        }
    }

    pub fn set_last_update(
        &mut self,
        update_dt: DateTime<Utc>,
    ) -> Result<&mut Self, UpdateTimeEarlierThanCreationError> {
        if self.create_date > update_dt {
            Err(UpdateTimeEarlierThanCreationError)
        } else {
            self.last_update = Some(update_dt);
            Ok(self)
        }
    }

    pub fn add_api_params(
        &mut self,
        api_params: &Vec<Rc<RefCell<APIParam>>>,
    ) -> Result<&mut Self, BorrowMutError> {
        for api_param in api_params {
            self.api_params.insert(
                api_param.as_ref().borrow().name().to_string(),
                api_param.clone(),
            );
        }
        return Ok(self);
    }

    pub fn get_api_params_by_name(
        &self,
        api_param_name: &Vec<&str>,
    ) -> HashMap<String, Rc<RefCell<APIParam>>> {
        let mut result_map: HashMap<String, Rc<RefCell<APIParam>>> = HashMap::new();
        for name in api_param_name {
            if let Some(matched_param) = self.api_params.get(*name) {
                result_map.insert(String::from(*name), matched_param.clone());
            }
        }
        result_map
    }

    pub fn remove_api_param_by_names(&mut self, api_param_names: &Vec<String>) -> &mut Self {
        for api_param_id in api_param_names {
            self.api_params.remove(api_param_id);
        }
        return self;
    }

    pub fn remove_all_api_params(&mut self) -> &mut Self {
        self.api_params.clear();
        return self;
    }

    pub fn add_columns_to_schema(&mut self, columns: &Vec<Rc<RefCell<Column>>>) -> &mut Self {
        self.schema().as_ref().borrow_mut().insert_columns(columns);
        return self;
    }

    pub fn remove_columns_from_schema_by_name(&mut self, column_names: &Vec<String>) -> &mut Self {
        self.schema()
            .as_ref()
            .borrow_mut()
            .remove_columns_by_name(column_names);
        return self;
    }

    pub fn remove_all_columns_from_schema(&mut self) -> &mut Self {
        self.schema().as_ref().borrow_mut().remove_all_columns();
        return self;
    }
}

impl Default for Dataset {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: String::from("New APIParam"),
            description: String::from("Please write a description."),
            endpoint: String::from("/example/endpoint"),
            api_params: HashMap::new(),
            schema: Rc::new(RefCell::new(DataSchema::default())),
            create_date: chrono::offset::Utc::now(),
            last_update: None,
            update_successful: None,
            sync_enabled: false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
}
