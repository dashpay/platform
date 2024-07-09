mod traversal_validator;
pub use traversal_validator::*;

#[cfg(test)]
mod test {
    use super::*;
    use crate::consensus::basic::BasicError;
    use crate::consensus::codes::ErrorWithCode;
    use crate::consensus::ConsensusError;
    use assert_matches::assert_matches;
    use platform_value::{platform_value, Value};
    use platform_version::version::PlatformVersion;

    fn init() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .try_init();
    }

    #[ignore]
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
        let mut result = traversal_validator(&schema, &[], PlatformVersion::first())
            .expect("expected traversal validator to succeed");
        assert_eq!(2, result.errors.len());
        let first_error = get_basic_error(result.errors.pop().unwrap());
        let second_error = get_basic_error(result.errors.pop().unwrap());

        assert_matches!(
            first_error,
            BasicError::JsonSchemaCompilationError(msg) if msg.compilation_error().starts_with("invalid path: '/properties/bar': byteArray cannot")
        );
        assert_matches!(
            second_error,
            BasicError::JsonSchemaCompilationError(msg) if msg.compilation_error().starts_with("invalid path: '/properties': byteArray cannot")
        );
    }

    #[ignore]
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
        assert!(traversal_validator(&schema, &[], PlatformVersion::first())
            .expect("expected traversal validator to succeed")
            .is_valid());
    }

    #[ignore]
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
        let result = traversal_validator(&schema, &[], PlatformVersion::first())
            .expect("expected traversal validator to succeed");
        let consensus_error = result.errors.first().expect("the error should be returned");

        match consensus_error {
            ConsensusError::BasicError(BasicError::IncompatibleRe2PatternError(err)) => {
                assert_eq!(err.path(), "/properties/bar".to_string());
                assert_eq!(
                    err.pattern(),
                    "^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$".to_string()
                );
                assert_eq!(consensus_error.code(), 10202);
            }
            _ => panic!("Expected error to be IncompatibleRe2PatternError"),
        }
    }

    #[ignore]
    #[test]
    fn should_be_valid_complex_for_complex_schema() {
        let schema = get_document_schema();

        assert!(traversal_validator(&schema, &[], PlatformVersion::first())
            .expect("expected traversal validator to exist for first protocol version")
            .is_valid())
    }

    #[ignore]
    #[test]
    fn invalid_result_for_array_of_object() {
        let mut schema = get_document_schema();
        schema["properties"]["arrayOfObject"]["items"]["properties"]["simple"]["pattern"] =
            platform_value!("^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$");

        let result = traversal_validator(&schema, &[], PlatformVersion::first())
            .expect("expected traversal validator to exist for first protocol version");
        let consensus_error = result.errors.first().expect("the error should be returned");

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
                assert_eq!(consensus_error.code(), 10202);
            }
            _ => panic!("Expected error to be IncompatibleRe2PatternError"),
        }
    }

    #[ignore]
    #[test]
    fn invalid_result_for_array_of_objects() {
        let mut schema = get_document_schema();
        schema["properties"]["arrayOfObjects"]["items"][0]["properties"]["simple"]["pattern"] =
            platform_value!("^((?!-|_)[a-zA-Z0-9-_]{0,62}[a-zA-Z0-9])$");

        let result = traversal_validator(&schema, &[], PlatformVersion::first())
            .expect("expected traversal validator to exist for first protocol version");
        let consensus_error = result.errors.first().expect("the error should be returned");

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
                assert_eq!(consensus_error.code(), 10202);
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
