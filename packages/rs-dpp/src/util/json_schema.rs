use anyhow::{anyhow, bail};
use platform_value::Value;
use serde_json::Value as JsonValue;

use crate::data_contract::errors::DataContractError;
use crate::identifier;

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
    // fn get_indices<I: FromIterator<Index>>(&self) -> Result<I, anyhow::Error>;
    /// returns the indexes from Json Schema
    // fn get_indices_map<I: FromIterator<(String, Index)>>(&self) -> Result<I, anyhow::Error>;
    /// returns true if json value contains property `contentMediaType` and it equals to Identifier
    fn is_type_of_identifier(&self) -> bool;
}

pub fn resolve_uri<'a>(value: &'a Value, uri: &str) -> Result<&'a Value, DataContractError> {
    if !uri.starts_with("#/") {
        return Err(DataContractError::InvalidURI(
            "only local uri references are allowed".to_string(),
        ));
    }

    let string_path = uri.strip_prefix("#/").unwrap().replace('/', ".");
    value.get_value_at_path(&string_path).map_err(|e| e.into())
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

    // TODO: Why we are doing this?
    // fn get_indices<I: FromIterator<Index>>(&self) -> Result<I, anyhow::Error> {
    //     let indices_with_raw_properties: Vec<IndexWithRawProperties> = match self.get("indices") {
    //         Some(raw_indices) => serde_json::from_value(raw_indices.to_owned())?,
    //
    //         None => vec![],
    //     };
    //
    //     indices_with_raw_properties
    //         .into_iter()
    //         .map(Index::try_from)
    //         .collect::<Result<I, anyhow::Error>>()
    // }

    fn is_type_of_identifier(&self) -> bool {
        if let JsonValue::Object(ref map) = self {
            if let Some(JsonValue::String(media_type)) = map.get("contentMediaType") {
                return media_type == identifier::MEDIA_TYPE;
            }
        }
        false
    }

    // TODO: Why do we need this?
    // fn get_indices_map<I: FromIterator<(String, Index)>>(&self) -> Result<I, Error> {
    //     let indices_with_raw_properties: Vec<IndexWithRawProperties> = match self.get("indices") {
    //         Some(raw_indices) => serde_json::from_value(raw_indices.to_owned())?,
    //
    //         None => vec![],
    //     };
    //
    //     indices_with_raw_properties
    //         .into_iter()
    //         .map(|r| {
    //             let index = Index::try_from(r)?;
    //             Ok((index.name().clone(), index))
    //         })
    //         .collect::<Result<I, anyhow::Error>>()
    // }
}

#[cfg(test)]
mod test {

    use crate::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use crate::data_contract::document_type::DocumentType;

    use platform_value::Identifier;
    use platform_version::version::LATEST_PLATFORM_VERSION;
    use serde_json::json;

    #[test]
    fn test_extract_indices() {
        let input = json!({
            "type": "object",
            "indices": [
                {
                    "properties": [
                        {
                            "$ownerId": "asc"
                        }
                    ],
                    "name": "&ownerId",
                    "unique": true
                },
                {
                    "properties": [
                        {
                            "$ownerId": "asc"
                        },
                        {
                            "$updatedAt": "asc"
                        }
                    ],
                    "name": "&ownerId&updatedAt"
                }
            ],
            "properties": {
                "avatarUrl": {
                    "type": "string",
                    "format": "uri",
                    "maxLength": 2048,
                    "position": 0
                },
                "publicMessage": {
                    "type": "string",
                    "maxLength": 140,
                    "position": 1
                },
                "displayName": {
                    "type": "string",
                    "maxLength": 25,
                    "position": 2
                }
            },
            "required": [
                "$createdAt",
                "$updatedAt"
            ],
            "additionalProperties": false
        });

        let platform_value = platform_value::to_value(input).unwrap();

        let document_type = DocumentType::try_from_schema(
            Identifier::random(),
            "doc",
            platform_value,
            None,
            false,
            false,
            false,
            LATEST_PLATFORM_VERSION,
        )
        .unwrap();

        let indices = document_type.indices();

        assert_eq!(indices.len(), 2);

        assert_eq!(indices[0].name, "&ownerId");
        assert_eq!(indices[0].properties.len(), 1);
        assert_eq!(indices[0].properties[0].name, "$ownerId");
        assert!(indices[0].properties[0].ascending);
        assert!(indices[0].unique);

        assert_eq!(indices[1].name, "&ownerId&updatedAt");
        assert_eq!(indices[1].properties.len(), 2);
        assert_eq!(indices[1].properties[0].name, "$ownerId");
        assert_eq!(indices[1].properties[1].name, "$updatedAt");
        assert!(!indices[1].unique);
    }
}
