use std::convert::TryInto;

use anyhow::{anyhow, bail};

use serde::de::DeserializeOwned;
use serde_json::{Number, Value as JsonValue};

use crate::{
    errors::ProtocolError,
    identifier::{self},
};

use super::json_path::{JsonPath, JsonPathLiteral, JsonPathStep};

mod insert_with_path;
use insert_with_path::*;

mod remove_path;
use remove_path::*;

const PROPERTY_CONTENT_MEDIA_TYPE: &str = "contentMediaType";
const PROPERTY_PROTOCOL_VERSION: &str = "protocolVersion";

/// JsonValueExt contains a set of helper methods that simplify work with JsonValue
pub trait JsonValueExt {
    /// assumes the Json Value is a map and tries to remove the given property
    fn remove(&mut self, property_name: &str) -> Result<JsonValue, anyhow::Error>;
    /// assumes the Json Value is a map and tries to remove the given property and deserialize into the provided type
    fn remove_into<K: DeserializeOwned>(&mut self, property_name: &str)
        -> Result<K, anyhow::Error>;
    /// assumes the Json Value is a map and tries to insert the given value under given property
    fn insert(&mut self, property_name: String, value: JsonValue) -> Result<(), anyhow::Error>;
    /// assumes the Json Value is an array and tries to add value to the array
    fn push(&mut self, value: JsonValue) -> Result<(), anyhow::Error>;
    fn get_string(&self, property_name: &str) -> Result<&str, anyhow::Error>;
    fn get_i64(&self, property_name: &str) -> Result<i64, anyhow::Error>;
    fn get_f64(&self, property_name: &str) -> Result<f64, anyhow::Error>;
    fn get_u8(&self, property_name: &str) -> Result<u8, anyhow::Error>;
    fn get_u32(&self, property_name: &str) -> Result<u32, anyhow::Error>;
    fn get_u64(&self, property_name: &str) -> Result<u64, anyhow::Error>;
    fn get_bytes(&self, property_name: &str) -> Result<Vec<u8>, anyhow::Error>;
    /// returns the the mutable JsonValue from provided path. The path is dot-separated string. i.e `properties.id`
    fn get_value_mut(&mut self, string_path: &str) -> Result<&mut JsonValue, anyhow::Error>;
    /// returns the the JsonValue from provided path. The path is dot-separated string. i.e `properties[0].id`
    fn get_value(&self, string_path: &str) -> Result<&JsonValue, anyhow::Error>;
    /// return  the JsonValue from from provided path. The path is a slice of [`JsonPathStep`]
    fn get_value_by_path(&self, path: &[JsonPathStep]) -> Result<&JsonValue, anyhow::Error>;
    /// return  the mutable JsonValue from from provided path. The path is a slice of [`JsonPathStep`]
    fn get_value_by_path_mut(
        &mut self,
        path: &[JsonPathStep],
    ) -> Result<&mut JsonValue, anyhow::Error>;

    /// assumes that the JsonValue is a Map and tries to remove the u32
    fn remove_u32(&mut self, property_name: &str) -> Result<u32, anyhow::Error>;

    fn add_protocol_version(
        &mut self,
        property_name: &str,
        protocol_version: u32,
    ) -> Result<(), ProtocolError>;

    /// Insert value under the path. Path is dot-separated string. i.e `properties[0].id`. If parents don't
    /// exists they will be created
    fn insert_with_path(&mut self, path: &str, value: JsonValue) -> Result<(), anyhow::Error>;

    /// Removes data from given path and tries deserialize it into provided type
    fn remove_value_at_path_into<K: DeserializeOwned>(
        &mut self,
        property_name: &str,
    ) -> Result<K, anyhow::Error>;
    fn get_bool(&self, property_name: &str) -> Result<bool, anyhow::Error>;
}

impl JsonValueExt for JsonValue {
    fn push(&mut self, value: JsonValue) -> Result<(), anyhow::Error> {
        match self.as_array_mut() {
            Some(map) => {
                map.push(value);
                Ok(())
            }
            None => bail!("data isn't an array: '{:?}'", self),
        }
    }

    fn insert(&mut self, property_name: String, value: JsonValue) -> Result<(), anyhow::Error> {
        match self.as_object_mut() {
            Some(map) => {
                map.insert(property_name, value);
                Ok(())
            }
            None => bail!(
                "getting property '{}' failed: the data isn't a map: '{:?}'",
                self,
                property_name
            ),
        }
    }

    fn remove_into<K: DeserializeOwned>(
        &mut self,
        property_name: &str,
    ) -> Result<K, anyhow::Error> {
        match self.as_object_mut() {
            Some(map) => {
                if let Some(data) = map.remove(property_name) {
                    serde_json::from_value(data)
                        .map_err(|err| anyhow!("unable convert data: {}`", err))
                } else {
                    bail!(
                        "the property '{}' doesn't exist in {:?}",
                        property_name,
                        self
                    )
                }
            }
            None => bail!("the property '{}' isn't a map: '{:?}'", property_name, self),
        }
    }

    fn remove(&mut self, property_name: &str) -> Result<JsonValue, anyhow::Error> {
        match self.as_object_mut() {
            Some(map) => map.remove(property_name).ok_or_else(|| {
                anyhow!(
                    "the property '{}' doesn't exists in '{:?}'",
                    property_name,
                    self
                )
            }),
            None => bail!("the property '{}' isn't a map: '{:?}'", property_name, self),
        }
    }

    fn get_string(&self, property_name: &str) -> Result<&str, anyhow::Error> {
        let property_value = self.get(property_name).ok_or_else(|| {
            anyhow!(
                "the property '{}' doesn't exist in {:?}",
                property_name,
                self
            )
        })?;

        if let JsonValue::String(s) = property_value {
            return Ok(s);
        }
        bail!(
            "getting property '{}' failed: {:?} isn't a String",
            property_name,
            property_value
        );
    }

    fn get_u8(&self, property_name: &str) -> Result<u8, anyhow::Error> {
        let property_value = self.get(property_name).ok_or_else(|| {
            anyhow!(
                "the property '{}' doesn't exist in '{:?}'",
                property_name,
                self
            )
        })?;

        if let JsonValue::Number(s) = property_value {
            return s
                .as_u64()
                .ok_or_else(|| anyhow!("unable convert {} to u64", s))?
                .try_into()
                .map_err(|e| anyhow!("unable convert {} to u8: {}", s, e));
        }
        bail!(
            "getting property '{}' failed: {:?} isn't a number",
            property_name,
            property_value
        );
    }

    fn get_u32(&self, property_name: &str) -> Result<u32, anyhow::Error> {
        let property_value = self.get(property_name).ok_or_else(|| {
            anyhow!(
                "the property '{}' doesn't exist in '{:?}'",
                property_name,
                self
            )
        })?;

        if let JsonValue::Number(s) = property_value {
            return s
                .as_u64()
                .ok_or_else(|| anyhow!("unable convert {} to u64", s))?
                .try_into()
                .map_err(|e| anyhow!("unable convert {} to u32: {}", s, e));
        }
        bail!(
            "getting property '{}' failed: {:?} isn't a number",
            property_name,
            property_value
        );
    }

    fn get_u64(&self, property_name: &str) -> Result<u64, anyhow::Error> {
        let property_value = self.get(property_name).ok_or_else(|| {
            anyhow!(
                "the property '{}' doesn't exist in '{:?}'",
                property_name,
                self
            )
        })?;

        if let JsonValue::Number(s) = property_value {
            return s
                .as_u64()
                .ok_or_else(|| anyhow!("unable convert {} to u64", s));
        }
        bail!(
            "getting property '{}' failed: {:?} isn't a number",
            property_name,
            property_value
        );
    }

    fn get_i64(&self, property_name: &str) -> Result<i64, anyhow::Error> {
        let property_value = self.get(property_name).ok_or_else(|| {
            anyhow!(
                "the property '{}' doesn't exist in '{:?}'",
                property_name,
                self
            )
        })?;

        if let JsonValue::Number(s) = property_value {
            return s
                .as_i64()
                .ok_or_else(|| anyhow!("unable convert {} to i64", s));
        }
        bail!(
            "getting property '{}' failed: {:?} isn't a number",
            property_name,
            property_value
        );
    }

    fn get_f64(&self, property_name: &str) -> Result<f64, anyhow::Error> {
        let property_value = self.get(property_name).ok_or_else(|| {
            anyhow!(
                "the property '{}' doesn't exist in '{:?}'",
                property_name,
                self
            )
        })?;

        if let JsonValue::Number(s) = property_value {
            return s
                .as_f64()
                .ok_or_else(|| anyhow!("unable convert {} to f64", s));
        }
        bail!(
            "getting property '{}' failed: {:?} isn't a number",
            property_name,
            property_value
        );
    }

    // TODO this method has an additional allocation which should be avoided
    fn get_bytes(&self, property_name: &str) -> Result<Vec<u8>, anyhow::Error> {
        let property_value = self.get(property_name).ok_or_else(|| {
            anyhow!(
                "the property '{}' doesn't exist in '{:?}'",
                property_name,
                self
            )
        })?;

        serde_json::from_value(property_value.clone())
            .map_err(|e| anyhow!("getting property '{}' failed: {}", property_name, e))
    }

    /// returns the value from the JsonValue based on the path: i.e "root.data[0].id"
    fn get_value_mut(&mut self, string_path: &str) -> Result<&mut JsonValue, anyhow::Error> {
        let path_literal: JsonPathLiteral = string_path.into();
        let path: JsonPath = path_literal.try_into().unwrap();
        get_value_from_json_path_mut(&path, self)
            .ok_or_else(|| anyhow!("the property '{}' not found", string_path))
    }

    /// returns the value from the JsonValue based on the path: i.e "root.data[0].id"
    fn get_value(&self, string_path: &str) -> Result<&JsonValue, anyhow::Error> {
        let path_literal: JsonPathLiteral = string_path.into();
        let path: JsonPath = path_literal.try_into().unwrap();
        get_value_from_json_path(&path, self)
            .ok_or_else(|| anyhow!("the property '{}' not found", string_path))
    }

    /// returns the value from the JsonValue based on the path: i.e "root.data[0].id"
    fn get_value_by_path(&self, path: &[JsonPathStep]) -> Result<&JsonValue, anyhow::Error> {
        get_value_from_json_path(path, self)
            .ok_or_else(|| anyhow!("the property '{:?}' not found", path))
    }

    fn get_value_by_path_mut(
        &mut self,
        path: &[JsonPathStep],
    ) -> Result<&mut JsonValue, anyhow::Error> {
        get_value_from_json_path_mut(path, self)
            .ok_or_else(|| anyhow!("the property '{:?}' not found", path))
    }

    fn add_protocol_version<'a>(
        &mut self,
        property_name: &str,
        protocol_version: u32,
    ) -> Result<(), ProtocolError> {
        match self {
            JsonValue::Object(ref mut m) => {
                m.insert(
                    String::from(property_name),
                    JsonValue::Number(Number::from(protocol_version)),
                );
            }
            _ => return Err(anyhow!("The '{:?}' isn't a map", self).into()),
        }

        Ok(())
    }

    fn remove_u32(&mut self, property_name: &str) -> Result<u32, anyhow::Error> {
        match self {
            JsonValue::Object(ref mut m) => match m.remove(property_name) {
                Some(JsonValue::Number(number)) => Ok(number.as_u64().ok_or_else(|| {
                    anyhow!("unable to convert '{}' into unsigned integer", number)
                })? as u32),
                _ => {
                    bail!("Unable to find '{}' in '{}'", property_name, self)
                }
            },
            _ => bail!("the Json Value isn't a map: {:?}", self),
        }
    }

    /// Insert value under the path. Path is dot-separated string. i.e `properties[0].id`
    fn insert_with_path(
        &mut self,
        string_path: &str,
        value: JsonValue,
    ) -> Result<(), anyhow::Error> {
        let path_literal: JsonPathLiteral = string_path.into();
        let path: JsonPath = path_literal.try_into().unwrap();
        insert_with_path(self, &path, value)
    }

    /// Removes the value under given path and tries to deserialize it into provided type
    fn remove_value_at_path_into<K: DeserializeOwned>(
        &mut self,
        path: &str,
    ) -> Result<K, anyhow::Error> {
        let path_literal: JsonPathLiteral = path.into();
        let json_path: JsonPath = path_literal.try_into().unwrap();

        let data = remove_path(&json_path, self)
            .ok_or_else(|| anyhow!("the '{path}' doesn't exists in '{self:#?}'"))?;

        serde_json::from_value(data).map_err(|err| anyhow!("unable convert data: {}`", err))
    }

    fn get_bool(&self, property_name: &str) -> Result<bool, anyhow::Error> {
        let property_value = self.get(property_name).ok_or_else(|| {
            anyhow!(
                "the property '{}' doesn't exist in '{:?}'",
                property_name,
                self
            )
        })?;

        if let JsonValue::Bool(s) = property_value {
            return Ok(*s);
        }
        bail!(
            "getting property '{}' failed: {:?} isn't a boolean",
            property_name,
            property_value
        );
    }
}

fn identifier_filter(value: &JsonValue) -> bool {
    if let JsonValue::Object(object) = value {
        if let Some(JsonValue::String(media_type)) = object.get(PROPERTY_CONTENT_MEDIA_TYPE) {
            return media_type == identifier::MEDIA_TYPE;
        }
    }
    false
}

/// returns the value from the JsonValue based on the path: i.e "root.data[0].id"
pub fn get_value_mut<'a>(string_path: &str, value: &'a mut JsonValue) -> Option<&'a mut JsonValue> {
    let path_literal: JsonPathLiteral = string_path.into();
    let path: JsonPath = path_literal.try_into().unwrap();
    get_value_from_json_path_mut(&path, value)
}

/// returns the value from the JsonValue based on the JsonPath
pub fn get_value_from_json_path_mut<'a>(
    path: &[JsonPathStep],
    value: &'a mut JsonValue,
) -> Option<&'a mut JsonValue> {
    let mut last_ptr: &mut JsonValue = value;

    for step in path {
        match step {
            JsonPathStep::Index(index) => {
                last_ptr = last_ptr.get_mut(index)?;
            }

            JsonPathStep::Key(key) => {
                last_ptr = last_ptr.get_mut(key)?;
            }
        }
    }
    Some(last_ptr)
}

/// returns the value from the JsonValue based on the JsonPath
pub fn get_value_from_json_path<'a>(
    path: &[JsonPathStep],
    value: &'a JsonValue,
) -> Option<&'a JsonValue> {
    let mut last_ptr: &JsonValue = value;

    for step in path {
        match step {
            JsonPathStep::Index(index) => {
                last_ptr = last_ptr.get(index)?;
            }
            JsonPathStep::Key(key) => {
                last_ptr = last_ptr.get(key)?;
            }
        }
    }
    Some(last_ptr)
}

#[cfg(test)]
mod test {
    use serde_json::json;

    use super::*;

    #[test]
    fn insert_with_parents() {
        let mut document = json!({
            "root" :  {
                "from" : {
                    "id": "123",
                    "message": "text_message",
                },
            }
        });

        document
            .insert_with_path("root.to.new_field", json!("new_value"))
            .expect("no errors");
        document
            .insert_with_path("root.array[0].new_field", json!("new_value"))
            .expect("no errors");

        assert_eq!(document["root"]["from"]["id"], json!("123"));
        assert_eq!(document["root"]["from"]["message"], json!("text_message"));
        assert_eq!(document["root"]["to"]["new_field"], json!("new_value"));
        assert_eq!(
            document["root"]["array"][0]["new_field"],
            json!("new_value")
        );
    }
}
