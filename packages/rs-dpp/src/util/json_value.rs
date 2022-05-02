use crate::util::deserializer;
use anyhow::{anyhow, bail};
use std::{collections::BTreeMap, convert::TryInto};

use log::trace;
use serde_json::{Number, Value as JsonValue};

use super::{
    json_path::{JsonPath, JsonPathLiteral, JsonPathStep},
    string_encoding::Encoding,
};
use crate::{
    errors::ProtocolError,
    identifier::{self, Identifier},
};

const PROPERTY_CONTENT_MEDIA_TYPE: &str = "contentMediaType";

#[derive(Debug, Clone, Copy)]
pub enum ReplaceWith {
    Bytes,
    Base58,
}

/// JsonValueExt contains a set of helper methods that simplify work with JsonValue
pub trait JsonValueExt {
    fn get_string(&self, property_name: &str) -> Result<&str, anyhow::Error>;
    fn get_i64(&self, property_name: &str) -> Result<i64, anyhow::Error>;
    fn get_f64(&self, property_name: &str) -> Result<f64, anyhow::Error>;
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
    /// replaces Identifiers specified by path with either the Bytes format or Base58-encoded string format
    fn replace_identifier_paths<'a>(
        &mut self,
        paths: impl IntoIterator<Item = &'a str>,
        with: ReplaceWith,
    ) -> Result<(), anyhow::Error>;

    fn parse_and_add_protocol_version(
        &mut self,
        protocol_bytes: &[u8],
    ) -> Result<(), ProtocolError>;
}

impl JsonValueExt for JsonValue {
    fn get_string(&self, property_name: &str) -> Result<&str, anyhow::Error> {
        let property_value = self
            .get(property_name)
            .ok_or_else(|| anyhow!("the property {} doesn't exist in Json Value", property_name))?;

        if let JsonValue::String(s) = property_value {
            return Ok(s);
        }
        bail!("{:?} isn't a string", property_value);
    }

    fn get_u64(&self, property_name: &str) -> Result<u64, anyhow::Error> {
        let property_value = self
            .get(property_name)
            .ok_or_else(|| anyhow!("the property {} doesn't exist in Json Value", property_name))?;

        if let JsonValue::Number(s) = property_value {
            return s
                .as_u64()
                .ok_or_else(|| anyhow!("unable convert {} to u64", s));
        }
        bail!("{:?} isn't a number", property_value);
    }

    fn get_i64(&self, property_name: &str) -> Result<i64, anyhow::Error> {
        let property_value = self
            .get(property_name)
            .ok_or_else(|| anyhow!("the property {} doesn't exist in Json Value", property_name))?;

        if let JsonValue::Number(s) = property_value {
            return s
                .as_i64()
                .ok_or_else(|| anyhow!("unable convert {} to i64", s));
        }
        bail!("{:?} isn't a number", property_value);
    }

    fn get_f64(&self, property_name: &str) -> Result<f64, anyhow::Error> {
        let property_value = self
            .get(property_name)
            .ok_or_else(|| anyhow!("the property {} doesn't exist in Json Value", property_name))?;

        if let JsonValue::Number(s) = property_value {
            return s
                .as_f64()
                .ok_or_else(|| anyhow!("unable convert {} to f64", s));
        }
        bail!("{:?} isn't a number", property_value);
    }

    // TODO this method has an additional allocation which should be avoided
    fn get_bytes(&self, property_name: &str) -> Result<Vec<u8>, anyhow::Error> {
        let property_value = self
            .get(property_name)
            .ok_or_else(|| anyhow!("the property {} doesn't exist in Json Value", property_name))?;

        if let JsonValue::Array(s) = property_value {
            let data = serde_json::to_vec(s)?;
            return Ok(data);
        }
        bail!("{:?} isn't an array", property_value);
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

    fn replace_identifier_paths<'a>(
        &mut self,
        paths: impl IntoIterator<Item = &'a str>,
        with: ReplaceWith,
    ) -> Result<(), anyhow::Error> {
        for raw_path in paths {
            let mut to_replace = get_value_mut(raw_path, self);
            match to_replace {
                Some(ref mut v) => {
                    replace_identifier(v, with)?;
                }
                None => {
                    trace!("path '{}' is not found, replacing to {:?} ", raw_path, with)
                }
            }
        }
        Ok(())
    }

    fn parse_and_add_protocol_version<'a>(
        &mut self,
        protocol_bytes: &[u8],
    ) -> Result<(), ProtocolError> {
        let protocol_version = deserializer::get_protocol_version(protocol_bytes)?;
        match self {
            JsonValue::Object(ref mut m) => {
                m.insert(
                    String::from("$protocolVersion"),
                    JsonValue::Number(Number::from(protocol_version)),
                );
            }
            _ => return Err(anyhow!("The '{:?}' isn't a map", self).into()),
        }

        Ok(())
    }
}

/// replaces the Identifiers specified in binary_properties with Bytes or Base58
pub fn identifiers_to(
    binary_properties: &BTreeMap<String, JsonValue>,
    dynamic_data: &mut JsonValue,
    to: ReplaceWith,
) -> Result<(), ProtocolError> {
    let identifier_paths = binary_properties
        .iter()
        .filter(|(_, p)| identifier_filter(p))
        .map(|(name, _)| name.as_str());

    dynamic_data.replace_identifier_paths(identifier_paths, to)?;
    Ok(())
}

/// replaces the Identifier wrapped in Json Value to either the Bytes or Base58 form
pub fn replace_identifier(
    to_replace: &mut JsonValue,
    with: ReplaceWith,
) -> Result<(), ProtocolError> {
    let mut json_value = JsonValue::Null;
    std::mem::swap(to_replace, &mut json_value);
    match with {
        ReplaceWith::Base58 => {
            let data_bytes: Vec<u8> = serde_json::from_value(json_value)?;

            let identifier = Identifier::from_bytes(&data_bytes)?;
            *to_replace = JsonValue::String(identifier.to_string(Encoding::Base58));
        }
        ReplaceWith::Bytes => {
            let data_string: String = serde_json::from_value(json_value)?;
            let identifier = Identifier::from_string(&data_string, Encoding::Base58)?.to_vec();
            *to_replace = JsonValue::Array(identifier);
        }
    }
    Ok(())
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
    fn test_replace_identifier_paths_happy_path() {
        let mut document = json!({
            "root" :  {
                "from" : {
                    "id": "6oCKUeLVgjr7VZCyn1LdGbrepqKLmoabaff5WQqyTKYP",
                    "message": "text_message",
                },
                "to" : {
                    "id": "5wpZAEWndYcTeuwZpkmSa8s49cHXU5q2DhdibesxFSu8",
                    "message": "text_message",
                },
                "transactions" : [
                    {
                    "message": "text_message",
                    },
                    {
                    "id": "5wpZAEWndYcTeuwZpkmSa8s49cHXU5q2DhdibesxFSu8",
                    "message": "text_message",
                    "inner":  {
                        "document_id" : "5wpZAEWndYcTeuwZpkmSa8s49cHXU5q2DhdibesxFSu8",
                    }
                    }
                ]
            }
        });

        assert!(document["root"]["from"]["id"].is_string());
        assert!(document["root"]["from"]["message"].is_string());
        assert!(document["root"]["to"]["id"].is_string());
        assert!(document["root"]["to"]["message"].is_string());
        assert!(document["root"]["transactions"][1]["id"].is_string());
        assert!(document["root"]["transactions"][1]["inner"]["document_id"].is_string());

        let mut binary_properties: BTreeMap<String, JsonValue> = Default::default();
        let paths = vec![
            "root.from.id",
            "root.to.id",
            "root.transactions[1].id",
            "root.transactions[1].inner.document_id",
        ];

        for p in paths {
            binary_properties.insert(
                p.to_string(),
                json!({ "contentMediaType": "application/x.dash.dpp.identifier"}),
            );
        }

        identifiers_to(&binary_properties, &mut document, ReplaceWith::Bytes).unwrap();
        assert!(document["root"]["from"]["id"].is_array());
        assert!(document["root"]["from"]["message"].is_string());
        assert!(document["root"]["to"]["id"].is_array());
        assert!(document["root"]["to"]["message"].is_string());
        assert!(document["root"]["transactions"][1]["id"].is_array());
        assert!(document["root"]["transactions"][1]["inner"]["document_id"].is_array());

        identifiers_to(&binary_properties, &mut document, ReplaceWith::Base58).unwrap();
        assert!(document["root"]["from"]["id"].is_string());
        assert!(document["root"]["from"]["message"].is_string());
        assert!(document["root"]["to"]["id"].is_string());
        assert!(document["root"]["to"]["message"].is_string());
        assert!(document["root"]["transactions"][1]["id"].is_string());
        assert!(document["root"]["transactions"][1]["inner"]["document_id"].is_string());
    }

    #[test]
    fn test_replace_identifier_path_with_bytes_wrong_identifier() {
        let mut document = json!({
            "root" :  {
                "from" : {
                    "id": "123",
                    "message": "text_message",
                },
            }
        });

        assert!(document["root"]["from"]["id"].is_string());

        let mut binary_properties: BTreeMap<String, JsonValue> = BTreeMap::new();
        binary_properties.insert(
            "root.from.id".to_string(),
            json!({ "contentMediaType": "application/x.dash.dpp.identifier"}),
        );
        binary_properties.insert(
            "root.to.id".to_string(),
            json!({ "contentMediaType": "application/x.dash.dpp.identifier"}),
        );

        match identifiers_to(&binary_properties, &mut document, ReplaceWith::Bytes) {
            Err(ProtocolError::IdentifierError(_)) => {}
            v => {
                panic!("unexpected returned value: {:?}", v)
            }
        }
    }
}
