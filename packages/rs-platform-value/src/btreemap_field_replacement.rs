use crate::btreemap_path_extensions::BTreeValueMapPathHelper;
use crate::value_map::{ValueMap, ValueMapHelper};
use crate::{Error, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy)]
pub enum ReplacementType {
    Bytes,
    TextBase58,
    TextBase64,
}

impl ReplacementType {
    pub fn replace_for_bytes(&self, bytes: Vec<u8>) -> Value {
        match self {
            ReplacementType::Bytes => Value::Bytes(bytes),
            ReplacementType::TextBase58 => Value::Text(bs58::encode(bytes).into_string()),
            ReplacementType::TextBase64 => Value::Text(base64::encode(bytes)),
        }
    }

    pub fn replace_consume_value(&self, value: Value) -> Result<Value, Error> {
        let bytes = value.into_system_bytes()?;
        Ok(self.replace_for_bytes(bytes))
    }
}

pub trait BTreeValueMapInsertionPathHelper {
    fn replace_at_path(
        &mut self,
        path: &str,
        replacement_type: ReplacementType,
    ) -> Result<bool, Error>;
    fn replace_at_paths<I: IntoIterator<Item = String>>(
        &mut self,
        paths: I,
        replacement_type: ReplacementType,
    ) -> Result<HashMap<String, bool>, Error>;
}

impl BTreeValueMapInsertionPathHelper for BTreeMap<String, Value> {
    fn replace_at_path(
        &mut self,
        path: &str,
        replacement_type: ReplacementType,
    ) -> Result<bool, Error> {
        let mut split = path.split(".").peekable();
        let first = split.next();
        let Some(first_path_component) = first else {
            return Err(Error::PathError("path was empty".to_string()));
        };
        let Some(mut current_value) = self.get_mut(first_path_component) else {
            return Ok(false);
        };
        while let Some(path_component) = split.next() {
            let map = current_value.as_map_mut_ref()?;
            let Some(mut new_value) = map.get_key_mut(path_component) else {
                return Ok(false);
            };
            current_value = new_value;
            if split.peek().is_none() {
                let bytes = current_value.to_system_bytes()?;
                new_value = &mut replacement_type.replace_for_bytes(bytes);
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn replace_at_paths<I: IntoIterator<Item = String>>(
        &mut self,
        paths: I,
        replacement_type: ReplacementType,
    ) -> Result<HashMap<String, bool>, Error> {
        paths
            .into_iter()
            .map(|path| {
                let success = self.replace_at_path(path.as_str(), replacement_type)?;
                Ok((path, success))
            })
            .collect()
    }
}
