use std::collections::BTreeMap;

use serde_json::{Map, Value as JsonValue};

use crate::util::json_value::JsonValueSchemaExt;

const PROPERTY_PROPERTIES: &str = "properties";
const PROPERTY_ITEMS: &str = "items";
const PROPERTY_TYPE: &str = "type";

///  Construct and get all properties with `contentEncoding` keyword
pub fn get_binary_properties(schema: &JsonValue) -> BTreeMap<String, &JsonValue> {
    let mut binary_properties: BTreeMap<String, &JsonValue> = BTreeMap::new();
    if let Some(JsonValue::Object(schema_properties)) = schema.get(PROPERTY_PROPERTIES) {
        for (property_name, property_value) in schema_properties {
            build_binary_properties_map(
                property_value,
                Some(property_name.to_string()),
                &mut binary_properties,
            );
        }
    }
    binary_properties
}
/// Recursively build properties map
fn build_binary_properties_map<'a>(
    schema: &'a JsonValue,
    property_path: Option<String>,
    binary_paths: &mut BTreeMap<String, &'a JsonValue>,
) {
    if let JsonValue::Object(map) = schema {
        match get_schema_property_type(map) {
            Some("object") => {
                visit_map(map, property_path.as_ref(), binary_paths);
            }
            Some("array") => {
                if let Some(JsonValue::Object(items)) = map.get(PROPERTY_ITEMS) {
                    if get_schema_property_type(items) == Some("object") {
                        visit_map(items, property_path.as_ref(), binary_paths);
                    }
                }
                if let Some(JsonValue::Array(items)) = map.get(PROPERTY_ITEMS) {
                    visit_array(items, property_path.as_ref(), binary_paths);
                }
            }
            _ => {}
        }
    }
    if schema.is_byte_array() {
        binary_paths.insert(property_path.unwrap_or_else(|| String::from("")), schema);
    }
}

fn visit_array<'a>(
    array: &'a [JsonValue],
    property_path: Option<&String>,
    binary_paths: &mut BTreeMap<String, &'a JsonValue>,
) {
    for (index, v) in array.iter().enumerate() {
        let property_path = if let Some(ref path) = property_path {
            format!("{}[{}]", path, index)
        } else {
            index.to_string()
        };
        build_binary_properties_map(v, Some(property_path), binary_paths);
    }
}

fn visit_map<'a>(
    map: &'a Map<String, JsonValue>,
    property_path: Option<&String>,
    binary_paths: &mut BTreeMap<String, &'a JsonValue>,
) {
    if let Some(JsonValue::Object(properties)) = map.get(PROPERTY_PROPERTIES) {
        for (key, v) in properties {
            let property_path = if let Some(ref path) = property_path {
                format!("{}.{}", path, key)
            } else {
                key.to_string()
            };
            build_binary_properties_map(v, Some(property_path), binary_paths);
        }
    }
}

fn get_schema_property_type(m: &serde_json::Map<String, JsonValue>) -> Option<&str> {
    if let Some(JsonValue::String(ref st)) = m.get(PROPERTY_TYPE) {
        return Some(st);
    }
    None
}

#[cfg(test)]
mod test {
    use super::get_binary_properties;
    use serde_json::{json, Value as JsonValue};
    use std::collections::BTreeMap;

    #[test]
    fn test_get_binary_properties() {
        let schema = get_input_data();

        let content = json!({
            "type" : "object",
            "byteArray" : true
        });

        let mut expected_result: BTreeMap<String, &JsonValue> = BTreeMap::new();
        let expected_paths = &[
            "arrayOfObject.withByteArray",
            "arrayOfObjects[0].withByteArray",
            "arrayOfObjects[2][0].withByteArray",
            "nestedObject.withByteArray",
            "withByteArray",
        ];
        for path in expected_paths {
            expected_result.insert(path.to_string(), &content);
        }

        let result = get_binary_properties(&schema);
        assert_eq!(expected_result, result);
    }

    fn get_input_data() -> JsonValue {
        json!({
            "properties": {
                "simple": {
                    "type": "string"
                },
                "withByteArray": {
                    "type": "object",
                    "byteArray": true
                },
                "nestedObject": {
                    "type": "object",
                    "properties": {
                        "simple": {
                            "type": "string"
                        },
                        "withByteArray": {
                            "type": "object",
                            "byteArray": true
                        }
                    }
                },
                "arrayOfObject": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "simple": {
                                "type": "string"
                            },
                            "withByteArray": {
                                "type": "object",
                                "byteArray": true
                            }
                        }
                    }
                },
                "arrayOfObjects": {
                    "type": "array",
                    "items": [
                        {
                            "type": "object",
                            "properties": {
                                "simple": {
                                    "type": "string"
                                },
                                "withByteArray": {
                                    "type": "object",
                                    "byteArray": true
                                }
                            }
                        },
                        {
                            "type": "string"
                        },
                        {
                            "type": "array",
                            "items": [
                                {
                                    "type": "object",
                                    "properties": {
                                        "simple": {
                                            "type": "string"
                                        },
                                        "withByteArray": {
                                            "type": "object",
                                            "byteArray": true
                                        }
                                    }
                                }
                            ]
                        }
                    ]
                }
            }
        })
    }
}
