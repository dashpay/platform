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
#[allow(clippy::approx_constant)]
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

    // ------- New coverage tests -------

    // push

    #[test]
    fn push_adds_to_array() {
        let mut arr = json!([1, 2, 3]);
        arr.push(json!(4)).expect("should push");
        assert_eq!(arr, json!([1, 2, 3, 4]));
    }

    #[test]
    fn push_on_non_array_fails() {
        let mut obj = json!({"a": 1});
        assert!(obj.push(json!(1)).is_err());
    }

    // insert

    #[test]
    fn insert_adds_key_to_object() {
        let mut obj = json!({"existing": 1});
        JsonValueExt::insert(&mut obj, "new_key".to_string(), json!("new_value"))
            .expect("should insert");
        assert_eq!(obj["new_key"], json!("new_value"));
        assert_eq!(obj["existing"], json!(1));
    }

    #[test]
    fn insert_on_non_object_fails() {
        let mut arr = json!([1, 2]);
        assert!(JsonValueExt::insert(&mut arr, "key".to_string(), json!(1)).is_err());
    }

    // remove

    #[test]
    fn remove_existing_property() {
        let mut obj = json!({"a": 1, "b": 2});
        let removed = JsonValueExt::remove(&mut obj, "a").expect("should remove");
        assert_eq!(removed, json!(1));
        assert!(obj.get("a").is_none());
    }

    #[test]
    fn remove_nonexistent_property_fails() {
        let mut obj = json!({"a": 1});
        assert!(JsonValueExt::remove(&mut obj, "missing").is_err());
    }

    #[test]
    fn remove_on_non_object_fails() {
        let mut val = json!("string");
        assert!(JsonValueExt::remove(&mut val, "key").is_err());
    }

    // remove_into

    #[test]
    fn remove_into_deserializes_value() {
        let mut obj = json!({"count": 42});
        let count: u64 = obj
            .remove_into("count")
            .expect("should remove and deserialize");
        assert_eq!(count, 42);
    }

    #[test]
    fn remove_into_missing_property_fails() {
        let mut obj = json!({"a": 1});
        let result: Result<u64, _> = obj.remove_into("missing");
        assert!(result.is_err());
    }

    #[test]
    fn remove_into_on_non_object_fails() {
        let mut val = json!(123);
        let result: Result<u64, _> = val.remove_into("key");
        assert!(result.is_err());
    }

    // get_string

    #[test]
    fn get_string_returns_string_value() {
        let obj = json!({"name": "Alice"});
        let result = obj.get_string("name").expect("should get string");
        assert_eq!(result, "Alice");
    }

    #[test]
    fn get_string_missing_property_fails() {
        let obj = json!({"a": 1});
        assert!(obj.get_string("missing").is_err());
    }

    #[test]
    fn get_string_non_string_value_fails() {
        let obj = json!({"num": 42});
        assert!(obj.get_string("num").is_err());
    }

    // get_u8

    #[test]
    fn get_u8_returns_u8_value() {
        let obj = json!({"val": 200});
        let result = obj.get_u8("val").expect("should get u8");
        assert_eq!(result, 200);
    }

    #[test]
    fn get_u8_too_large_fails() {
        let obj = json!({"val": 300});
        assert!(obj.get_u8("val").is_err());
    }

    #[test]
    fn get_u8_missing_property_fails() {
        let obj = json!({});
        assert!(obj.get_u8("val").is_err());
    }

    #[test]
    fn get_u8_non_number_fails() {
        let obj = json!({"val": "text"});
        assert!(obj.get_u8("val").is_err());
    }

    // get_u32

    #[test]
    fn get_u32_returns_u32_value() {
        let obj = json!({"val": 100000});
        let result = obj.get_u32("val").expect("should get u32");
        assert_eq!(result, 100_000);
    }

    #[test]
    fn get_u32_missing_property_fails() {
        let obj = json!({});
        assert!(obj.get_u32("val").is_err());
    }

    #[test]
    fn get_u32_non_number_fails() {
        let obj = json!({"val": true});
        assert!(obj.get_u32("val").is_err());
    }

    // get_u64

    #[test]
    fn get_u64_returns_u64_value() {
        let obj = json!({"val": 9999999999u64});
        let result = obj.get_u64("val").expect("should get u64");
        assert_eq!(result, 9_999_999_999);
    }

    #[test]
    fn get_u64_missing_property_fails() {
        let obj = json!({});
        assert!(obj.get_u64("val").is_err());
    }

    #[test]
    fn get_u64_non_number_fails() {
        let obj = json!({"val": "text"});
        assert!(obj.get_u64("val").is_err());
    }

    // get_i64

    #[test]
    fn get_i64_returns_i64_value() {
        let obj = json!({"val": -42});
        let result = obj.get_i64("val").expect("should get i64");
        assert_eq!(result, -42);
    }

    #[test]
    fn get_i64_positive_value() {
        let obj = json!({"val": 100});
        let result = obj.get_i64("val").expect("should get i64");
        assert_eq!(result, 100);
    }

    #[test]
    fn get_i64_missing_property_fails() {
        let obj = json!({});
        assert!(obj.get_i64("val").is_err());
    }

    #[test]
    fn get_i64_non_number_fails() {
        let obj = json!({"val": "text"});
        assert!(obj.get_i64("val").is_err());
    }

    // get_f64

    #[test]
    fn get_f64_returns_f64_value() {
        let obj = json!({"val": 3.14});
        let result = obj.get_f64("val").expect("should get f64");
        assert!((result - 3.14).abs() < f64::EPSILON);
    }

    #[test]
    fn get_f64_integer_value() {
        let obj = json!({"val": 42});
        let result = obj.get_f64("val").expect("should get f64 from integer");
        assert!((result - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn get_f64_missing_property_fails() {
        let obj = json!({});
        assert!(obj.get_f64("val").is_err());
    }

    #[test]
    fn get_f64_non_number_fails() {
        let obj = json!({"val": "text"});
        assert!(obj.get_f64("val").is_err());
    }

    // get_bytes

    #[test]
    fn get_bytes_from_array() {
        let obj = json!({"data": [1, 2, 3, 4, 5]});
        let result = obj.get_bytes("data").expect("should get bytes");
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn get_bytes_missing_property_fails() {
        let obj = json!({});
        assert!(obj.get_bytes("data").is_err());
    }

    // get_bool

    #[test]
    fn get_bool_true() {
        let obj = json!({"flag": true});
        let result = obj.get_bool("flag").expect("should get bool");
        assert!(result);
    }

    #[test]
    fn get_bool_false() {
        let obj = json!({"flag": false});
        let result = obj.get_bool("flag").expect("should get bool");
        assert!(!result);
    }

    #[test]
    fn get_bool_missing_property_fails() {
        let obj = json!({});
        assert!(obj.get_bool("flag").is_err());
    }

    #[test]
    fn get_bool_non_bool_fails() {
        let obj = json!({"flag": 1});
        assert!(obj.get_bool("flag").is_err());
    }

    // get_value and get_value_mut

    #[test]
    fn get_value_navigates_nested_path() {
        let obj = json!({
            "a": {
                "b": {
                    "c": "deep_value"
                }
            }
        });

        let result = obj.get_value("a.b.c").expect("should find nested value");
        assert_eq!(result, &json!("deep_value"));
    }

    #[test]
    fn get_value_with_array_index() {
        let obj = json!({
            "items": [10, 20, 30]
        });

        let result = obj
            .get_value("items[1]")
            .expect("should find array element");
        assert_eq!(result, &json!(20));
    }

    #[test]
    fn get_value_missing_path_fails() {
        let obj = json!({"a": 1});
        assert!(obj.get_value("a.b.c").is_err());
    }

    #[test]
    fn get_value_mut_modifies_nested_value() {
        let mut obj = json!({
            "a": {
                "b": "old"
            }
        });

        let val = obj.get_value_mut("a.b").expect("should find value");
        *val = json!("new");

        assert_eq!(obj["a"]["b"], json!("new"));
    }

    // get_value_by_path and get_value_by_path_mut

    #[test]
    fn get_value_by_path_with_steps() {
        let obj = json!({
            "root": {
                "items": [1, 2, 3]
            }
        });

        let path = vec![
            JsonPathStep::Key("root".to_string()),
            JsonPathStep::Key("items".to_string()),
            JsonPathStep::Index(2),
        ];

        let result = obj.get_value_by_path(&path).expect("should find value");
        assert_eq!(result, &json!(3));
    }

    #[test]
    fn get_value_by_path_missing_fails() {
        let obj = json!({"a": 1});
        let path = vec![JsonPathStep::Key("nonexistent".to_string())];
        assert!(obj.get_value_by_path(&path).is_err());
    }

    #[test]
    fn get_value_by_path_mut_modifies_value() {
        let mut obj = json!({
            "list": ["a", "b", "c"]
        });

        let path = vec![
            JsonPathStep::Key("list".to_string()),
            JsonPathStep::Index(1),
        ];

        let val = obj.get_value_by_path_mut(&path).expect("should find value");
        *val = json!("modified");

        assert_eq!(obj["list"][1], json!("modified"));
    }

    // remove_u32

    #[test]
    fn remove_u32_removes_and_returns_value() {
        let mut obj = json!({"version": 42});
        let result = obj.remove_u32("version").expect("should remove u32");
        assert_eq!(result, 42);
        assert!(obj.get("version").is_none());
    }

    #[test]
    fn remove_u32_missing_property_fails() {
        let mut obj = json!({"a": 1});
        assert!(obj.remove_u32("missing").is_err());
    }

    #[test]
    fn remove_u32_non_number_fails() {
        let mut obj = json!({"val": "text"});
        assert!(obj.remove_u32("val").is_err());
    }

    #[test]
    fn remove_u32_on_non_object_fails() {
        let mut val = json!([1, 2, 3]);
        assert!(val.remove_u32("key").is_err());
    }

    // add_protocol_version

    #[test]
    fn add_protocol_version_inserts_number() {
        let mut obj = json!({});
        obj.add_protocol_version("protocolVersion", 5)
            .expect("should add protocol version");

        assert_eq!(obj["protocolVersion"], json!(5));
    }

    #[test]
    fn add_protocol_version_on_non_object_fails() {
        let mut val = json!([1, 2]);
        assert!(val.add_protocol_version("protocolVersion", 1).is_err());
    }

    // remove_value_at_path_into

    #[test]
    fn remove_value_at_path_into_removes_and_deserializes() {
        let mut obj = json!({
            "config": {
                "timeout": 30
            }
        });

        let timeout: u64 = obj
            .remove_value_at_path_into("config.timeout")
            .expect("should remove and deserialize");
        assert_eq!(timeout, 30);
        assert!(obj["config"].get("timeout").is_none());
    }

    #[test]
    fn remove_value_at_path_into_missing_path_fails() {
        let mut obj = json!({"a": 1});
        let result: Result<u64, _> = obj.remove_value_at_path_into("a.b.c");
        assert!(result.is_err());
    }

    // Free functions: get_value_mut, get_value_from_json_path, get_value_from_json_path_mut

    #[test]
    fn free_get_value_mut_works() {
        let mut obj = json!({"x": {"y": 10}});
        let val = get_value_mut("x.y", &mut obj).expect("should find value");
        *val = json!(20);
        assert_eq!(obj["x"]["y"], json!(20));
    }

    #[test]
    fn get_value_from_json_path_navigates_keys_and_indices() {
        let obj = json!({
            "data": [
                {"id": "first"},
                {"id": "second"}
            ]
        });

        let path = vec![
            JsonPathStep::Key("data".to_string()),
            JsonPathStep::Index(1),
            JsonPathStep::Key("id".to_string()),
        ];

        let result = get_value_from_json_path(&path, &obj).expect("should find");
        assert_eq!(result, &json!("second"));
    }

    #[test]
    fn get_value_from_json_path_returns_none_for_missing() {
        let obj = json!({"a": 1});
        let path = vec![JsonPathStep::Key("nonexistent".to_string())];
        assert!(get_value_from_json_path(&path, &obj).is_none());
    }

    #[test]
    fn get_value_from_json_path_mut_modifies_value() {
        let mut obj = json!({"items": [100, 200, 300]});

        let path = vec![
            JsonPathStep::Key("items".to_string()),
            JsonPathStep::Index(0),
        ];

        let val = get_value_from_json_path_mut(&path, &mut obj).expect("should find");
        *val = json!(999);
        assert_eq!(obj["items"][0], json!(999));
    }

    #[test]
    fn get_value_from_json_path_empty_path_returns_root() {
        let obj = json!({"a": 1});
        let path: Vec<JsonPathStep> = vec![];
        let result = get_value_from_json_path(&path, &obj).expect("should return root");
        assert_eq!(result, &json!({"a": 1}));
    }

    // insert_with_path via trait

    #[test]
    fn insert_with_path_creates_deeply_nested_structure() {
        let mut obj = json!({});
        obj.insert_with_path("a.b.c", json!("deep"))
            .expect("should insert");
        assert_eq!(obj["a"]["b"]["c"], json!("deep"));
    }

    #[test]
    fn insert_with_path_into_existing_structure() {
        let mut obj = json!({"a": {"b": 1}});
        obj.insert_with_path("a.c", json!(2))
            .expect("should insert sibling");
        assert_eq!(obj["a"]["c"], json!(2));
        assert_eq!(obj["a"]["b"], json!(1));
    }
}
