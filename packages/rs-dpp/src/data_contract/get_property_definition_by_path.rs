use std::include_str;

use anyhow::anyhow;
use lazy_static::lazy_static;
use serde_json::Value as JsonValue;

use crate::errors::ProtocolError;
use crate::util::{json_schema::JsonSchemaExt, json_value::JsonValueExt};

lazy_static! {
    static ref BASE_DOCUMENT_SCHEMA: JsonValue =
        serde_json::from_str(include_str!("../../schema/document/documentBase.json")).unwrap();
}
// Get user property definition
pub fn get_property_definition_by_path<'a>(
    document_definition: &'a JsonValue,
    path: &str,
) -> Result<&'a JsonValue, ProtocolError> {
    // Return system properties schema
    if path.starts_with('$') {
        return Ok(BASE_DOCUMENT_SCHEMA.get_value(&format!("properties.{}", path))?);
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

#[cfg(test)]
mod test {
    use serde_json::json;

    use crate::assert_error_contains;

    use super::*;

    #[test]
    fn test_get_system_properties() {
        let schema = get_schema();
        let result = get_property_definition_by_path(&schema, "$protocolVersion");
        let expect = json!({
            "type": "integer",
            "$comment": "Maximum is the latest protocol version"
        });
        assert_eq!(result.unwrap(), &expect);
    }

    #[test]
    fn test_top_level_property_not_found() {
        let schema = get_schema();
        let result = get_property_definition_by_path(&schema, "nope");
        assert_error_contains!(result, "the top-level property 'nope' cannot be found");
    }

    #[test]
    fn test_top_level_property_returned() {
        let schema = get_schema();
        let result = get_property_definition_by_path(&schema, "a");
        assert_eq!(result.unwrap(), &json!({"type" : "string"}));
    }

    #[test]
    fn test_return_nested_def_from_array() {
        let schema = get_schema();
        let result = get_property_definition_by_path(&schema, "b.inner");
        let expect = json!({
                "type": "object",
                "properties": {
                    "abc": {
                        "type": "string"
                    }
                }
        });
        assert_eq!(result.unwrap(), &expect);
    }

    #[test]
    fn should_return_nested_definition_from_object() {
        let schema = get_schema();
        let result = get_property_definition_by_path(&schema, "c.inner");
        let expect = json!({
                "type": "object",
                "patternProperties": {
                    "[a-z]": {
                        "type": "string"
                    }
                }

        });
        assert_eq!(result.unwrap(), &expect);
    }

    #[test]
    fn test_should_return_error_if_not_match_pattern() {
        let schema = get_schema();
        let result = get_property_definition_by_path(&schema, "c.inner.NOPE");
        assert_error_contains!(result, "Couldn't find 'properties'");
    }

    #[test]
    fn test_error_if_top_level_not_array_or_object() {
        let schema = get_schema();
        let result = get_property_definition_by_path(&schema, "a.someOther");
        assert_error_contains!(result, "the 'someOther' is not array or object");
    }

    fn get_schema() -> JsonValue {
        json!({
            "properties": {
                "a": {
                    "type": "string"
                },
                "b": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "inner": {
                                "type": "object",
                                "properties": {
                                    "abc": {
                                        "type": "string"
                                    }
                                }
                            }
                        }
                    }
                },
                "c": {
                    "type": "object",
                    "properties": {
                        "inner": {
                            "type": "object",
                            "patternProperties": {
                                "[a-z]": {
                                    "type": "string"
                                }
                            }
                        }
                    }
                }
            }
        })
    }
}
