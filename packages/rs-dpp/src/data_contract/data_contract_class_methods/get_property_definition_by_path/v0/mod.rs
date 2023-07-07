use std::include_str;

use anyhow::anyhow;
use lazy_static::lazy_static;
use serde_json::Value as JsonValue;

use crate::errors::ProtocolError;
use crate::prelude::DataContract;
#[cfg(test)]
use crate::tests::utils::SerdeTestExtension;
use crate::util::{json_schema::JsonSchemaExt, json_value::JsonValueExt};

lazy_static! {
    static ref EXTENDED_DOCUMENT_SCHEMA: JsonValue = serde_json::from_str(include_str!(
        "../../../../../schema/document/documentExtended.json"
    ))
    .unwrap();
}

impl DataContract {
    // Get user property definition
    pub(super) fn get_property_definition_by_path_v0<'a>(
        document_definition: &'a JsonValue,
        path: &str,
    ) -> Result<&'a JsonValue, ProtocolError> {
        // Return system properties schema
        if path.starts_with('$') {
            return Ok(EXTENDED_DOCUMENT_SCHEMA.get_value(&format!("properties.{}", path))?);
        }

        let mut path_components = path.split('.');

        let mut current_value: &JsonValue = document_definition.get_schema_properties()?;
        let top_level_property = path_components
            .next()
            .ok_or_else(|| anyhow!("the path '{}' is empty", path))?;
        current_value = current_value.get(top_level_property).ok_or_else(|| {
            anyhow!(
                "the top-level property '{}' cannot be found in {:?}",
                top_level_property,
                current_value
            )
        })?;

        for path in path_components {
            let schema_type = current_value.get_string("type");
            match schema_type {
                Ok("object") => {
                    let properties = current_value.get_schema_properties()?;
                    current_value = properties.get(path).ok_or_else(|| {
                        anyhow!(
                            "unable to find the property '{}' in '{:?}'",
                            path,
                            properties
                        )
                    })?;
                }

                Ok("array") => {
                    let items = current_value
                        .get("items")
                        .ok_or_else(|| anyhow!("the array '{}' doesn't contain items", path))?;
                    if !items.is_type_of_object() {
                        return Err(anyhow!("the items '{:?}' isn't type of object", items).into());
                    }

                    current_value = items.get_schema_properties()?.get(path).ok_or_else(|| {
                        anyhow!("unable to find the property '{}' in '{:?}'", path, items)
                    })?;
                }

                _ => {
                    return Err(anyhow!("the '{}' is not array or object", path).into());
                }
            }
        }

        Ok(current_value)
    }
}
