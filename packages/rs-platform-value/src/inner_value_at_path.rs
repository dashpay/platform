use crate::value_map::ValueMapHelper;
use crate::{Error, Value};
use std::collections::BTreeMap;

impl Value {
    pub fn remove_value_at_path(&mut self, path: &str) -> Result<Value, Error> {
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
        map.remove_key(last_path_component)
    }

    pub fn remove_values_at_paths<'a>(
        &'a mut self,
        paths: Vec<&'a str>,
    ) -> Result<BTreeMap<&'a str, Value>, Error> {
        paths
            .into_iter()
            .map(|path| Ok((path, self.remove_value_at_path(path)?)))
            .collect()
    }

    pub fn get_value_at_path<'a, 'b>(&'a self, path: &'b str) -> Result<&'a Value, Error> {
        let split = path.split('.');
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
        let split = path.split('.');
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
        let split = path.split('.');
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
        let split = path.split('.');
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
#[cfg(test)]
mod test {
    use crate::platform_value;

    #[test]
    fn insert_with_parents() {
        let mut document = platform_value!({
            "root" :  {
                "from" : {
                    "id": "123",
                    "message": "text_message",
                },
            }
        });

        document
            .set_value_at_full_path("root.to.new_field", platform_value!("new_value"))
            .expect("no errors");
        document
            .set_value_at_full_path("root.array[0].new_field", platform_value!("new_value"))
            .expect("no errors");

        assert_eq!(document["root"]["from"]["id"], platform_value!("123"));
        assert_eq!(
            document["root"]["from"]["message"],
            platform_value!("text_message")
        );
        assert_eq!(
            document["root"]["to"]["new_field"],
            platform_value!("new_value")
        );
        assert_eq!(
            document["root"]["array"][0]["new_field"],
            platform_value!("new_value")
        );
    }
}
