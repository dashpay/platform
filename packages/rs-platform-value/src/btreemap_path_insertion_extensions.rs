use crate::value_map::{ValueMap, ValueMapHelper};
use crate::{Error, Value};
use std::collections::BTreeMap;

pub trait BTreeValueMapInsertionPathHelper {
    fn insert_at_path(&mut self, path: &str, value: Value) -> Result<(), Error>;
}

impl BTreeValueMapInsertionPathHelper for BTreeMap<String, Value> {
    fn insert_at_path(&mut self, path: &str, value: Value) -> Result<(), Error> {
        let mut split = path.split(".").peekable();
        let first = split.next();
        let Some(first_path_component) = first else {
            return Err(Error::PathError("path was empty".to_string()));
        };
        if split.peek().is_none() {
            self.insert(first_path_component.to_string(), value);
        } else {
            let mut current_value = self
                .entry(first_path_component.to_string())
                .or_insert(Value::Map(ValueMap::new()));
            let mut last_path_component = None;
            while let Some(path_component) = split.next() {
                if split.peek().is_some() {
                    let map = current_value.as_map_mut_ref()?;
                    current_value =
                        map.get_key_mut_or_insert(path_component, Value::Map(ValueMap::new()));
                } else {
                    last_path_component = Some(path_component)
                }
            }
            if let Some(last_path_component) = last_path_component {
                let map = current_value.as_map_mut_ref()?;
                if let Some(mut new_value) = map.get_key_mut(last_path_component) {
                    *new_value = value;
                } else {
                    map.push((Value::Text(last_path_component.to_string()), value));
                }
            }
        }

        Ok(())
    }
}
