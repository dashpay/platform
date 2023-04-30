use platform_value::Value;
use regex::Regex;

use crate::consensus::basic::data_contract::IncompatibleRe2PatternError;
use crate::consensus::basic::json_schema_compilation_error::JsonSchemaCompilationError;
use crate::consensus::basic::value_error::ValueError;
use crate::consensus::{basic::BasicError, ConsensusError};
use crate::validation::SimpleConsensusValidationResult;

pub type SubValidator = fn(
    path: &str,
    key: &str,
    parent: &Value,
    value: &Value,
    result: &mut SimpleConsensusValidationResult,
);

pub fn validate(
    raw_data_contract: &Value,
    validators: &[SubValidator],
) -> SimpleConsensusValidationResult {
    let mut result = SimpleConsensusValidationResult::default();
    let mut values_queue: Vec<(&Value, String)> = vec![(raw_data_contract, String::from(""))];

    while let Some((value, path)) = values_queue.pop() {
        match value {
            Value::Map(current_map) => {
                for (key, current_value) in current_map.iter() {
                    if current_value.is_map() || current_value.is_array() {
                        let new_path =
                            format!("{}/{}", path, key.non_qualified_string_representation());
                        values_queue.push((current_value, new_path))
                    }
                    match key
                        .to_str()
                        .map_err(|err| BasicError::ValueError(ValueError::new(err)))
                    {
                        Ok(key) => {
                            for validator in validators {
                                validator(&path, key, value, current_value, &mut result);
                            }
                        }
                        Err(err) => result.add_error(err),
                    }
                }
            }
            Value::Array(arr) => {
                for (i, value) in arr.iter().enumerate() {
                    if value.is_map() {
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

pub fn pattern_is_valid_regex_validator(
    path: &str,
    key: &str,
    _parent: &Value,
    value: &Value,
    result: &mut SimpleConsensusValidationResult,
) {
    if key == "pattern" {
        if let Some(pattern) = value.as_str() {
            if let Err(err) = Regex::new(pattern) {
                result.add_error(IncompatibleRe2PatternError::new(
                    String::from(pattern),
                    path.to_string(),
                    err.to_string(),
                ));
            }
        } else {
            result.add_error(IncompatibleRe2PatternError::new(
                String::new(),
                path.to_string(),
                format!("{} is not a string", path),
            ));
        }
    }
}

fn unwrap_error_to_result<'a>(
    v: Result<Option<&'a Value>, ConsensusError>,
    result: &mut SimpleConsensusValidationResult,
) -> Option<&'a Value> {
    match v {
        Ok(v) => v,
        Err(e) => {
            result.add_error(e);
            None
        }
    }
}

pub fn byte_array_has_no_items_as_parent_validator(
    path: &str,
    key: &str,
    parent: &Value,
    value: &Value,
    result: &mut SimpleConsensusValidationResult,
) {
    if key == "byteArray"
        && value.is_bool()
        && (unwrap_error_to_result(
            parent.get("items").map_err(|e| {
                ConsensusError::BasicError(BasicError::ValueError(ValueError::new(e)))
            }),
            result,
        )
        .is_some()
            || unwrap_error_to_result(
                parent.get("prefixItems").map_err(|e| {
                    ConsensusError::BasicError(BasicError::ValueError(ValueError::new(e)))
                }),
                result,
            )
            .is_some())
    {
        let compilation_error = format!(
            "invalid path: '{}': byteArray cannot be used with 'items' or 'prefixItems",
            path
        );
        result.add_error(BasicError::JsonSchemaCompilationError(
            JsonSchemaCompilationError::new(compilation_error),
        ));
    }
}

#[cfg(test)]
mod test {
    use crate::consensus::codes::ErrorWithCode;
    use platform_value::platform_value;

    use super::*;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    #[test]
    fn should_return_error_if_bytes_array_parent_contains_items_or_prefix_items() {
        let schema: Value = platform_value!(
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
        let mut result = validate(&schema, &[byte_array_has_no_items_as_parent_validator]);
        assert_eq!(2, result.errors.len());
        let first_error = get_basic_error(result.errors.pop().unwrap());
        let second_error = get_basic_error(result.errors.pop().unwrap());

        assert!(matches!(
            first_error,
            BasicError::JsonSchemaCompilationError(msg) if msg.compilation_error().starts_with("invalid path: '/properties/bar': byteArray cannot"),
        ));
        assert!(matches!(
            second_error,
            BasicError::JsonSchemaCompilationError(msg) if msg.compilation_error().starts_with("invalid path: '/properties': byteArray cannot"),
        ));
    }

    #[test]
    fn should_return_valid_result() {
        let schema: Value = platform_value!(
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

        assert!(validate(&schema, &[pattern_is_valid_regex_validator]).is_valid())
    }

    #[test]
    fn should_return_invalid_result() {
        let schema: Value = platform_value!({
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
        let result = validate(&schema, &[pattern_is_valid_regex_validator]);
        let consensus_error = result.errors.get(0).expect("the error should be returned");

        match consensus_error {
            ConsensusError::BasicError(BasicError::IncompatibleRe2PatternError(err)) => {
                assert_eq!(err.path(), "/properties/bar".to_string());
                assert_eq!(
                    err.pattern(),
                    "^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$".to_string()
                );
                assert_eq!(consensus_error.code(), 1009);
            }
            _ => panic!("Expected error to be IncompatibleRe2PatternError"),
        }
    }

    #[test]
    fn should_be_valid_complex_for_complex_schema() {
        let schema = get_document_schema();
        assert!(validate(&schema, &[pattern_is_valid_regex_validator]).is_valid())
    }

    #[test]
    fn invalid_result_for_array_of_object() {
        let mut schema = get_document_schema();
        schema["properties"]["arrayOfObject"]["items"]["properties"]["simple"]["pattern"] =
            platform_value!("^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$");

        let result = validate(&schema, &[pattern_is_valid_regex_validator]);
        let consensus_error = result.errors.get(0).expect("the error should be returned");

        match consensus_error {
            ConsensusError::BasicError(BasicError::IncompatibleRe2PatternError(err)) => {
                assert_eq!(
                    err.path(),
                    "/properties/arrayOfObject/items/properties/simple".to_string()
                );
                assert_eq!(
                    err.pattern(),
                    "^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$".to_string()
                );
                assert_eq!(consensus_error.code(), 1009);
            }
            _ => panic!("Expected error to be IncompatibleRe2PatternError"),
        }
    }

    #[test]
    fn invalid_result_for_array_of_objects() {
        let mut schema = get_document_schema();
        schema["properties"]["arrayOfObjects"]["items"][0]["properties"]["simple"]["pattern"] =
            platform_value!("^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$");

        let result = validate(&schema, &[pattern_is_valid_regex_validator]);
        let consensus_error = result.errors.get(0).expect("the error should be returned");

        match consensus_error {
            ConsensusError::BasicError(BasicError::IncompatibleRe2PatternError(err)) => {
                assert_eq!(
                    err.path(),
                    "/properties/arrayOfObjects/items/[0]/properties/simple".to_string()
                );
                assert_eq!(
                    err.pattern(),
                    "^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$".to_string()
                );
                assert_eq!(consensus_error.code(), 1009);
            }
            _ => panic!("Expected error to be IncompatibleRe2PatternError"),
        }
    }

    fn get_document_schema() -> Value {
        platform_value!({
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
            return err;
        }
        panic!("the error: {:?} isn't a BasicError", error)
    }
}
