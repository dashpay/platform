use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

pub trait JsonSchemaExt {
    /// returns true if json value contains property 'type`, and it equals 'object'
    fn is_type_of_object(&self) -> bool;
    /// returns true if json value contains property 'type`, and it equals 'array'
    fn is_type_of_array(&self) -> bool;
    /// returns true if json value contains property `byteArray` and it equals true
    fn is_type_of_byte_array(&self) -> bool;
    /// returns true if json value contains property 'type`, and it equals 'string'
    fn is_type_of_string(&self) -> bool;
    /// returns the properties of Json Schema object
    fn get_schema_properties(&self) -> Result<&JsonValue, anyhow::Error>;
    /// returns the required fields of Json Schema object
    fn get_schema_required_fields(&self) -> Result<Vec<&str>, anyhow::Error>;
    /// returns the indexes from Json Schema
    fn get_indices(&self) -> Result<Vec<Index>, anyhow::Error>;
}

impl JsonSchemaExt for JsonValue {
    fn get_schema_required_fields(&self) -> Result<Vec<&str>, anyhow::Error> {
        if let JsonValue::Object(ref map) = self {
            let required = map.get("required");
            if required.is_none() {
                return Ok(vec![]);
            }
            if let JsonValue::Array(required_list) = required.unwrap() {
                return required_list
                    .iter()
                    .map(|v| v.as_str())
                    .collect::<Option<Vec<&str>>>()
                    .ok_or_else(|| anyhow!("unable to convert list of required fields to string"));
            }
            bail!("the 'required' property is not array");
        }
        bail!("the json value is not a map");
    }

    fn is_type_of_string(&self) -> bool {
        if let JsonValue::Object(ref map) = self {
            if let Some(JsonValue::String(schema_type)) = map.get("type") {
                return schema_type == "string";
            }
        }
        false
    }

    fn is_type_of_object(&self) -> bool {
        if let JsonValue::Object(ref map) = self {
            if let Some(JsonValue::String(schema_type)) = map.get("type") {
                return schema_type == "object";
            }
        }
        false
    }

    fn is_type_of_array(&self) -> bool {
        if let JsonValue::Object(ref map) = self {
            if let Some(JsonValue::String(schema_type)) = map.get("type") {
                return schema_type == "array";
            }
        }
        false
    }

    fn is_type_of_byte_array(&self) -> bool {
        if let JsonValue::Object(ref map) = self {
            if let Some(JsonValue::Bool(is_byte_array)) = map.get("byteArray") {
                return *is_byte_array;
            }
        }
        false
    }

    fn get_schema_properties(&self) -> Result<&JsonValue, anyhow::Error> {
        if let JsonValue::Object(ref map) = self {
            return map
                .get("properties")
                .ok_or_else(|| anyhow!("Couldn't find 'properties' in '{:?}'", map));
        }
        bail!("the {:?} isn't an map", self);
    }

    fn get_indices(&self) -> Result<Vec<Index>, anyhow::Error> {
        match self.get("indices") {
            Some(raw_indices) => Ok(serde_json::from_value(raw_indices.to_owned())?),
            None => Ok(vec![]),
        }
    }
}

// Indices documentation:  https://dashplatform.readme.io/docs/reference-data-contracts#document-indices
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Index {
    pub name: String,
    pub properties: Vec<BTreeMap<String, OrderBy>>,
    #[serde(default)]
    pub unique: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd, Clone)]
pub enum OrderBy {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_indices() {
        let input = json!({
            "properties" : {
                "field_one" : {
                    "type" : "string"
                },
                "field_two" : {
                    "type" : "string"
                }
            },
            "indices" : [
                {
                    "name" : "first_index",
                    "properties" :[
                        {"field_one" : "asc"},
                        {"field_two" : "desc"},
                    ],
                    "unique" : true

                },
                {
                    "name" : "second_index",
                    "properties" : [
                        {"field_two" : "desc"},
                    ],
                }
             ]
        });

        let indices_result = input.get_indices();

        let indices = indices_result.unwrap();
        assert_eq!(indices.len(), 2);
        assert_eq!(indices[0].name, "first_index");
        assert_eq!(indices[0].properties.len(), 2);
        assert_eq!(indices[0].properties[0]["field_one"], OrderBy::Asc);
        assert_eq!(indices[0].properties[1]["field_two"], OrderBy::Desc);
        assert!(indices[0].unique);

        assert_eq!(indices[1].name, "second_index");
        assert_eq!(indices[1].properties.len(), 1);
        assert!(!indices[1].unique);
    }
}
