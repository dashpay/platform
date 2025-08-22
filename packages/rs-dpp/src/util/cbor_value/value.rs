use std::convert::{TryFrom, TryInto};

use ciborium::value::Value;
use itertools::Itertools;

use crate::util::{
    cbor_value::convert::convert_to,
    json_path::{JsonPath, JsonPathLiteral, JsonPathStep},
};

use super::{FieldType, ReplacePaths, ValuesCollection};

impl ValuesCollection for ciborium::value::Value {
    type Value = ciborium::value::Value;
    type Key = ciborium::value::Value;

    fn get(&self, key: &Self::Value) -> Option<&Self::Value> {
        match self {
            Value::Array(ref arr) => {
                if let Some(idx) = key.as_integer() {
                    let idx: usize = idx.try_into().ok()?;
                    arr.get(idx)
                } else {
                    None
                }
            }
            Value::Map(map) => map
                .iter()
                .find_map(|(k, v)| if k == key { Some(v) } else { None }),

            _ => None,
        }
    }

    fn get_mut(&mut self, key: &Self::Value) -> Option<&mut Self::Value> {
        match self {
            Value::Array(ref mut arr) => {
                if let Some(idx) = key.as_integer() {
                    let idx: usize = idx.try_into().ok()?;
                    arr.get_mut(idx)
                } else {
                    None
                }
            }
            Value::Map(map) => map
                .iter_mut()
                .find_map(|(k, v)| if k == key { Some(v) } else { None }),

            _ => None,
        }
    }

    fn remove(&mut self, key: impl Into<Self::Key>) -> Option<Self::Value> {
        let key_cbor: Self::Key = key.into();
        match self {
            Value::Array(ref mut arr) => {
                if let Some(idx) = key_cbor.as_integer() {
                    let idx: usize = idx.try_into().ok()?;
                    if arr.len() < idx {
                        return Some(arr.remove(idx));
                    }
                }
                None
            }
            Value::Map(map) => {
                if let Some(idx) = map.iter().position(|(el_key, _)| el_key == &key_cbor) {
                    let (_, v) = map.remove(idx);
                    return Some(v);
                }
                None
            }
            _ => None,
        }
    }
}

impl ReplacePaths for ciborium::value::Value {
    type Value = ciborium::value::Value;

    fn replace_paths<I, C>(&mut self, paths: I, from: FieldType, to: FieldType)
    where
        I: IntoIterator<Item = C>,
        C: AsRef<str>,
    {
        for path in paths.into_iter() {
            self.replace_path(path.as_ref(), from, to);
        }
    }

    fn replace_path(&mut self, path: &str, from: FieldType, to: FieldType) -> Option<()> {
        let cbor_value = self.get_path_mut(path)?;
        let replace_with = convert_to(cbor_value, from, to)?;

        *cbor_value = replace_with;

        Some(())
    }

    fn get_path_mut(&mut self, path: &str) -> Option<&mut Value> {
        let cbor_path = to_path_of_cbors(path).ok()?;

        if cbor_path.is_empty() {
            return None;
        }
        if cbor_path.len() == 1 {
            return self.get_mut(&cbor_path[0]);
        }

        let mut current_level: &mut Value = self.get_mut(&cbor_path[0])?;
        for step in cbor_path.iter().skip(1) {
            match current_level {
                Value::Map(ref mut cbor_map) => current_level = get_from_cbor_map(cbor_map, step)?,
                Value::Array(ref mut cbor_array) => {
                    if let Some(idx) = step.as_integer() {
                        let id: usize = idx.try_into().ok()?;
                        current_level = cbor_array.get_mut(id)?
                    } else {
                        return None;
                    }
                }
                _ => {
                    // do nothing if it's not a container type
                }
            }
        }
        Some(current_level)
    }
}

pub fn get_from_cbor_map<'a>(
    cbor_map: &'a mut [(Value, Value)],
    key: &Value,
) -> Option<&'a mut Value> {
    cbor_map.iter_mut().find_map(|(current_key, value)| {
        if current_key == key {
            Some(value)
        } else {
            None
        }
    })
}

pub fn to_path_of_cbors(path: &str) -> Result<Vec<Value>, anyhow::Error> {
    let json_path = JsonPath::try_from(JsonPathLiteral(path))?;

    Ok(json_path
        .into_iter()
        .map(|step| match step {
            JsonPathStep::Key(key) => Value::Text(key),
            JsonPathStep::Index(index) => Value::Integer(index.into()),
        })
        .collect_vec())
}
