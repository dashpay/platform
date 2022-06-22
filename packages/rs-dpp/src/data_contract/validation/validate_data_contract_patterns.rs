use regex::Regex;
use serde_json::Value as JsonValue;

use crate::{consensus::ConsensusError, validation::ValidationResult};
pub fn validate_data_contract_patterns(raw_data_contract: &JsonValue) -> ValidationResult {
    let mut result = ValidationResult::default();
    let mut values_queue: Vec<(&JsonValue, String)> = vec![(raw_data_contract, String::from(""))];

    while let Some((value, path)) = values_queue.pop() {
        match value {
            JsonValue::Object(current_map) => {
                for (key, value) in current_map.iter() {
                    if value.is_object() || value.is_array() {
                        let new_path = format!("{}/{}", path, key);
                        values_queue.push((value, new_path))
                    }
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

        assert!(validate_data_contract_patterns(&schema).is_valid())
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
        let result = validate_data_contract_patterns(&schema);
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
        assert!(validate_data_contract_patterns(&schema).is_valid())
    }

    #[test]
    fn invalid_result_for_array_of_object() {
        let mut schema = get_document_schema();
        schema["properties"]["arrayOfObject"]["items"]["properties"]["simple"]["pattern"] =
            json!("^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$");

        let result = validate_data_contract_patterns(&schema);
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

        let result = validate_data_contract_patterns(&schema);
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
}
