use crate::value_map::ValueMapHelper;
use crate::{Error, Value};

impl Value {
    pub fn get_value_at_path<'a>(&'a self, path: &'a str) -> Result<&'a Value, Error> {
        let mut split = path.split('.');
        let mut current_value = self;
        for path_component in split {
            let map = current_value.to_map_ref()?;
            current_value = map.get_key(path_component).ok_or_else(|| {
                Error::StructureError(format!("unable to get property {path_component} in {path}"))
            })?;
        }
        Ok(current_value)
    }

    pub fn get_optional_value_at_path<'a>(
        &'a self,
        path: &'a str,
    ) -> Result<Option<&'a Value>, Error> {
        let mut split = path.split('.');
        let mut current_value = self;
        for path_component in split {
            let map = current_value.to_map_ref()?;
            let Some(new_value) = map.get_key(path_component) else {
                return Ok(None);
            };
            current_value = new_value;
        }
        Ok(Some(current_value))
    }

    pub fn get_mut_value_at_path<'a>(&'a mut self, path: &'a str) -> Result<&'a mut Value, Error> {
        let mut split = path.split('.');
        let mut current_value = self;
        for path_component in split {
            let map = current_value.to_map_mut()?;
            current_value = map.get_key_mut(path_component).ok_or_else(|| {
                Error::StructureError(format!("unable to get property {path_component} in {path}"))
            })?;
        }
        Ok(current_value)
    }

    pub fn get_optional_mut_value_at_path<'a>(
        &'a mut self,
        path: &'a str,
    ) -> Result<Option<&'a mut Value>, Error> {
        let mut split = path.split('.');
        let mut current_value = self;
        for path_component in split {
            let map = current_value.to_map_mut()?;
            let Some(new_value) = map.get_key_mut(path_component) else {
                return Ok(None);
            };
            current_value = new_value;
        }
        Ok(Some(current_value))
    }

    pub fn set_value_at_full_path(&mut self, path: &str, value: Value) -> Result<(), Error> {
        let mut split = path.split('.').peekable();
        let mut current_value = self;
        let mut last_path_component = None;
        while let Some(path_component) = split.next() {
            if split.peek().is_none() {
                last_path_component = Some(path_component);
            } else {
                let map = current_value.to_map_mut()?;
                current_value = map.get_key_mut(path_component).ok_or_else(|| {
                    Error::StructureError(format!(
                        "unable to get property {path_component} in {path}"
                    ))
                })?;
            };
        }
        let Some(last_path_component) = last_path_component else {
            return Err(Error::StructureError(format!("path was empty")));
        };
        let map = current_value.as_map_mut_ref()?;
        Ok(Self::insert_in_map(map, last_path_component, value))
    }

    pub fn set_value_at_path(&mut self, path: &str, key: &str, value: Value) -> Result<(), Error> {
        let map = self.get_mut_value_at_path(path)?.as_map_mut_ref()?;
        Ok(Self::insert_in_map(map, key, value))
    }
}
