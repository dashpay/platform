use std::{collections::HashMap, convert::TryInto};

use log::trace;
use serde_json::Value as JsonValue;

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

pub fn replace_json_value_with(
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

pub fn replace_paths_with<'a>(
    paths: impl IntoIterator<Item = &'a str>,
    value: &mut JsonValue,
    with: ReplaceWith,
) -> Result<(), ProtocolError> {
    for raw_path in paths {
        let mut to_replace = get_value_mut(raw_path, value);
        match to_replace {
            Some(ref mut v) => {
                replace_json_value_with(v, with)?;
            }
            None => {
                trace!("path '{}' is not found, replacing to {:?} ", raw_path, with)
            }
        }
    }
    Ok(())
}

pub fn identifiers_to(
    binary_properties: &HashMap<String, JsonValue>,
    dynamic_data: &mut JsonValue,
    to: ReplaceWith,
) -> Result<(), ProtocolError> {
    let identifier_paths = binary_properties
        .iter()
        .filter(|(_, p)| identifier_filter(p))
        .map(|(name, _)| name.as_str());

    replace_paths_with(identifier_paths, dynamic_data, to)?;
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

pub fn get_value_mut<'a>(string_path: &str, value: &'a mut JsonValue) -> Option<&'a mut JsonValue> {
    let path_literal: JsonPathLiteral = string_path.into();
    let path: JsonPath = path_literal.try_into().unwrap();
    get_value_from_path_mut(&path, value)
}

pub fn get_value_from_path_mut<'a>(
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

        let mut binary_properties: HashMap<String, JsonValue> = HashMap::new();
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

        let mut binary_properties: HashMap<String, JsonValue> = HashMap::new();
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
