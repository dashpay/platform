use regex::Regex;
use serde_json::Value as JsonValue;

use crate::{
    consensus::{basic::BasicError, ConsensusError},
    validation::ValidationResult,
};

type AtomicValidator =
    fn(path: &str, key: &str, parent: &JsonValue, value: &JsonValue, result: &mut ValidationResult);

pub fn validate(raw_data_contract: &JsonValue, validators: &[AtomicValidator]) -> ValidationResult {
    let mut result = ValidationResult::default();
    let mut values_queue: Vec<(&JsonValue, String)> = vec![(raw_data_contract, String::from(""))];

    while let Some((value, path)) = values_queue.pop() {
        match value {
            JsonValue::Object(current_map) => {
                for (key, current_value) in current_map.iter() {
                    if current_value.is_object() || current_value.is_array() {
                        let new_path = format!("{}/{}", path, key);
                        values_queue.push((current_value, new_path))
                    }
                    for validator in validators {
                        validator(&path, key, value, current_value, &mut result);
                    }
                }
            }
            JsonValue::Array(arr) => {
                for (i, value) in arr.iter().enumerate() {
                    if value.is_object() {
                        let new_path = format!("{}/[{}]", path, i);
                        values_queue.push((value, new_path))
                    }
                }
            }
            _ => {}
        };
    }
    result
}

pub fn pattern_validator(
    path: &str,
    key: &str,
    _parent: &JsonValue,
    value: &JsonValue,
    result: &mut ValidationResult,
) {
    if key == "pattern" {
        if let Some(pattern) = value.as_str() {
            if let Err(err) = Regex::new(pattern) {
                result.add_error(ConsensusError::IncompatibleRe2PatternError {
                    pattern: String::from(pattern),
                    path: path.to_string(),
                    message: err.to_string(),
                });
            }
        }
    }
}

pub fn byte_array_parent_validator(
    path: &str,
    key: &str,
    parent: &JsonValue,
    value: &JsonValue,
    result: &mut ValidationResult,
) {
    if key == "byteArray"
        && value.is_boolean()
        && (parent.get("items").is_some() || parent.get("prefixItems").is_some())
    {
        result.add_error(BasicError::JsonSchemaCompilationError(format!(
            "invalid path: '{}': byteArray cannot be used with 'items' or 'prefixItems",
            path
        )));
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    #[test]
    fn should_return_error_if_bytes_array_parent_contains_items_or_prefix_items() {
        let schema = json!(
             {
                "type": "object",
                "properties": {
                  "foo": { "type": "integer" },
                  "bar": {
                    "items" : {},
                    "byteArray": true
                  },
                  "byteArray" : false,
                  "items" : {},
                },
                "required": ["foo"],
                "additionalProperties": false,
              }
        );
        let mut result = validate(&schema, &[byte_array_parent_validator]);
        assert_eq!(2, result.errors().len());
        let first_error = get_basic_error(result.errors.pop().unwrap());
        let second_error = get_basic_error(result.errors.pop().unwrap());

        assert!(matches!(
            first_error,
            BasicError::JsonSchemaCompilationError(msg) if msg.starts_with("invalid path: '/properties/bar': byteArray cannot"),
        ));
        assert!(matches!(
            second_error,
            BasicError::JsonSchemaCompilationError(msg) if msg.starts_with("invalid path: '/properties': byteArray cannot"),
        ));
    }

    #[test]
    fn should_return_valid_result() {
        let schema = json!(
             {
                "type": "object",
                "properties": {
                  "foo": { "type": "integer" },
                  "bar": {
                    "type": "string",
                    "pattern": "([a-z]+)+$",
                  },
                },
                "required": ["foo"],
                "additionalProperties": false,
              }
        );

        assert!(validate(&schema, &[pattern_validator]).is_valid())
    }

    #[test]
    fn should_return_invalid_result() {
        let schema = json!({
            "type": "object",
            "properties": {
              "foo": { "type": "integer" },
              "bar": {
                "type": "string",
                "pattern": "^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$",
              },
            },
            "required": ["foo"],
            "additionalProperties": false,

        });
        let result = validate(&schema, &[pattern_validator]);
        let consensus_error = result.errors.get(0).expect("the error should be returned");

        assert!(
            matches!(consensus_error, ConsensusError::IncompatibleRe2PatternError {pattern, path, ..}
             if  path == "/properties/bar" &&
                 pattern == "^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$" &&
                 consensus_error.code() == 1009
            )
        );
    }

    #[test]
    fn should_be_valid_complex_for_complex_schema() {
        let schema = get_document_schema();
        assert!(validate(&schema, &[pattern_validator]).is_valid())
    }

    #[test]
    fn invalid_result_for_array_of_object() {
        let mut schema = get_document_schema();
        schema["properties"]["arrayOfObject"]["items"]["properties"]["simple"]["pattern"] =
            json!("^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$");

        let result = validate(&schema, &[pattern_validator]);
        let consensus_error = result.errors.get(0).expect("the error should be returned");

        assert!(
            matches!(consensus_error, ConsensusError::IncompatibleRe2PatternError {pattern, path, ..}
             if  path == "/properties/arrayOfObject/items/properties/simple" &&
                 pattern == "^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$" &&
                 consensus_error.code() == 1009
            )
        );
    }

    #[test]
    fn invalid_result_for_array_of_objects() {
        let mut schema = get_document_schema();
        schema["properties"]["arrayOfObjects"]["items"][0]["properties"]["simple"]["pattern"] =
            json!("^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$");

        let result = validate(&schema, &[pattern_validator]);
        let consensus_error = result.errors.get(0).expect("the error should be returned");

        assert!(
            matches!(consensus_error, ConsensusError::IncompatibleRe2PatternError {pattern, path, ..}
             if  path == "/properties/arrayOfObjects/items/[0]/properties/simple" &&
                 pattern == "^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$" &&
                 consensus_error.code() == 1009
            )
        );
    }

    fn get_document_schema() -> JsonValue {
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
                                "type": "string",
                                "pattern": ".*valid_pattern.*"
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
                                    "type": "string",
                                    "pattern": ".*valid_pattern.*"
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

    fn get_basic_error(error: ConsensusError) -> BasicError {
        if let ConsensusError::BasicError(err) = error {
            return *err;
        }
        panic!("the error: {} isn't a BasicError", error)
    }
}
