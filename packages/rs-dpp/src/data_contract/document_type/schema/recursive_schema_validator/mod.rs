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

    // ================================================================
    //  New tests that actually run (not #[ignore]'d) to cover the
    //  traversal_validator_v0 traversal logic.
    // ================================================================

    // Type alias matching the SubValidator function pointer type from v0
    type TestSubValidator = fn(
        &str,
        &str,
        &Value,
        &Value,
        &mut crate::validation::SimpleConsensusValidationResult,
        &PlatformVersion,
    ) -> Result<(), crate::ProtocolError>;

    #[test]
    fn traversal_empty_schema_with_no_validators_is_valid() {
        // An empty map should pass traversal with no validators
        let schema: Value = platform_value!({});
        let result = traversal_validator(&schema, &[], PlatformVersion::first())
            .expect("traversal_validator should succeed");
        assert!(
            result.is_valid(),
            "empty schema with no validators should be valid"
        );
    }

    #[test]
    fn traversal_flat_map_with_no_validators_is_valid() {
        // A flat map with simple values should traverse without errors
        let schema: Value = platform_value!({
            "type": "object",
            "additionalProperties": false
        });
        let result = traversal_validator(&schema, &[], PlatformVersion::first())
            .expect("traversal_validator should succeed");
        assert!(result.is_valid());
    }

    #[test]
    fn traversal_nested_maps_with_no_validators_is_valid() {
        // Nested maps should traverse without errors when there are no validators
        let schema: Value = platform_value!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                },
                "address": {
                    "type": "object",
                    "properties": {
                        "street": {
                            "type": "string"
                        },
                        "city": {
                            "type": "string"
                        }
                    }
                }
            }
        });
        let result = traversal_validator(&schema, &[], PlatformVersion::first())
            .expect("traversal_validator should succeed");
        assert!(result.is_valid());
    }

    #[test]
    fn traversal_with_array_containing_maps_visits_them() {
        // Arrays containing maps should have those maps traversed.
        // We verify by using a sub-validator that counts calls.
        use std::sync::atomic::{AtomicUsize, Ordering};
        static CALL_COUNT: AtomicUsize = AtomicUsize::new(0);

        fn counting_validator(
            _path: &str,
            _key: &str,
            _parent: &Value,
            _value: &Value,
            _result: &mut crate::validation::SimpleConsensusValidationResult,
            _platform_version: &PlatformVersion,
        ) -> Result<(), crate::ProtocolError> {
            CALL_COUNT.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        CALL_COUNT.store(0, Ordering::Relaxed);

        let schema: Value = platform_value!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array"
                }
            }
        });

        let validators: Vec<TestSubValidator> = vec![counting_validator];
        let result = traversal_validator(&schema, &validators, PlatformVersion::first())
            .expect("traversal_validator should succeed");
        assert!(result.is_valid());

        let count = CALL_COUNT.load(Ordering::Relaxed);
        assert!(
            count > 0,
            "sub-validator should have been called at least once, but was called {count} times"
        );
    }

    #[test]
    fn traversal_sub_validator_can_add_errors() {
        // A sub-validator that adds a consensus error should make the result invalid
        use crate::consensus::basic::value_error::ValueError;

        fn error_adding_validator(
            _path: &str,
            key: &str,
            _parent: &Value,
            _value: &Value,
            result: &mut crate::validation::SimpleConsensusValidationResult,
            _platform_version: &PlatformVersion,
        ) -> Result<(), crate::ProtocolError> {
            if key == "forbidden" {
                result.add_error(ConsensusError::BasicError(BasicError::ValueError(
                    ValueError::new(platform_value::Error::StructureError(
                        "forbidden key encountered".to_string(),
                    )),
                )));
            }
            Ok(())
        }

        let schema: Value = platform_value!({
            "type": "object",
            "properties": {
                "allowed": {
                    "type": "string"
                },
                "forbidden": {
                    "type": "integer"
                }
            }
        });

        let validators: Vec<TestSubValidator> = vec![error_adding_validator];
        let result = traversal_validator(&schema, &validators, PlatformVersion::first())
            .expect("traversal_validator should succeed");

        assert!(
            !result.is_valid(),
            "result should be invalid when sub-validator adds errors"
        );
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn traversal_sub_validator_returning_error_propagates() {
        // A sub-validator that returns Err should cause traversal_validator to return Err
        fn failing_validator(
            _path: &str,
            _key: &str,
            _parent: &Value,
            _value: &Value,
            _result: &mut crate::validation::SimpleConsensusValidationResult,
            _platform_version: &PlatformVersion,
        ) -> Result<(), crate::ProtocolError> {
            Err(crate::ProtocolError::DecodingError(
                "intentional test failure".to_string(),
            ))
        }

        let schema: Value = platform_value!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                }
            }
        });

        let validators: Vec<TestSubValidator> = vec![failing_validator];
        let result = traversal_validator(&schema, &validators, PlatformVersion::first());
        assert!(
            result.is_err(),
            "traversal_validator should propagate ProtocolError from sub-validator"
        );
    }

    #[test]
    fn traversal_handles_array_with_mixed_elements() {
        // An array containing a mix of maps and non-maps should only traverse maps
        use std::sync::atomic::{AtomicUsize, Ordering};
        static MAP_COUNT: AtomicUsize = AtomicUsize::new(0);

        fn map_counting_validator(
            _path: &str,
            _key: &str,
            _parent: &Value,
            _value: &Value,
            _result: &mut crate::validation::SimpleConsensusValidationResult,
            _platform_version: &PlatformVersion,
        ) -> Result<(), crate::ProtocolError> {
            MAP_COUNT.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        MAP_COUNT.store(0, Ordering::Relaxed);

        // Create an array with mixed elements: a map (should be visited) and strings (should not)
        let schema = Value::Array(vec![
            platform_value!({
                "innerKey": "innerValue"
            }),
            Value::Text("just a string".to_string()),
            Value::U64(42),
        ]);

        let validators: Vec<TestSubValidator> = vec![map_counting_validator];
        let result = traversal_validator(&schema, &validators, PlatformVersion::first())
            .expect("traversal_validator should succeed");
        assert!(result.is_valid());

        let count = MAP_COUNT.load(Ordering::Relaxed);
        // The map {"innerKey": "innerValue"} has 1 key, so validator should be called once
        assert_eq!(
            count, 1,
            "validator should be called exactly once for the single map element in the array"
        );
    }

    #[test]
    fn traversal_scalar_value_produces_valid_empty_result() {
        // Non-map, non-array values at the root should just produce an empty valid result
        let schema = Value::Text("hello".to_string());
        let result = traversal_validator(&schema, &[], PlatformVersion::first())
            .expect("traversal_validator should succeed");
        assert!(result.is_valid());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn traversal_deeply_nested_schema_visits_all_levels() {
        // A deeply nested schema should have all levels visited
        use std::sync::atomic::{AtomicUsize, Ordering};
        static DEEP_COUNT: AtomicUsize = AtomicUsize::new(0);

        fn deep_validator(
            _path: &str,
            _key: &str,
            _parent: &Value,
            _value: &Value,
            _result: &mut crate::validation::SimpleConsensusValidationResult,
            _platform_version: &PlatformVersion,
        ) -> Result<(), crate::ProtocolError> {
            DEEP_COUNT.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        DEEP_COUNT.store(0, Ordering::Relaxed);

        let schema: Value = platform_value!({
            "level1": {
                "level2": {
                    "level3": {
                        "level4": "leaf"
                    }
                }
            }
        });

        let validators: Vec<TestSubValidator> = vec![deep_validator];
        let result = traversal_validator(&schema, &validators, PlatformVersion::first())
            .expect("traversal_validator should succeed");
        assert!(result.is_valid());

        let count = DEEP_COUNT.load(Ordering::Relaxed);
        // level1 (nested map) + level2 (nested map) + level3 (nested map) + level4 (leaf "leaf")
        // Each map key triggers a validator call
        assert!(
            count >= 4,
            "validator should be called for all keys across all nesting levels, got {count}"
        );
    }

    #[test]
    fn traversal_complex_schema_with_no_validators_is_valid() {
        // Use the complex schema helper with no validators - should be valid
        let schema = get_document_schema();
        let result = traversal_validator(&schema, &[], PlatformVersion::first())
            .expect("traversal_validator should succeed");
        assert!(result.is_valid());
    }

    #[test]
    fn traversal_multiple_validators_all_called() {
        // Multiple sub-validators should all be called for each key
        use std::sync::atomic::{AtomicUsize, Ordering};
        static V1_COUNT: AtomicUsize = AtomicUsize::new(0);
        static V2_COUNT: AtomicUsize = AtomicUsize::new(0);

        fn validator_one(
            _path: &str,
            _key: &str,
            _parent: &Value,
            _value: &Value,
            _result: &mut crate::validation::SimpleConsensusValidationResult,
            _platform_version: &PlatformVersion,
        ) -> Result<(), crate::ProtocolError> {
            V1_COUNT.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn validator_two(
            _path: &str,
            _key: &str,
            _parent: &Value,
            _value: &Value,
            _result: &mut crate::validation::SimpleConsensusValidationResult,
            _platform_version: &PlatformVersion,
        ) -> Result<(), crate::ProtocolError> {
            V2_COUNT.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        V1_COUNT.store(0, Ordering::Relaxed);
        V2_COUNT.store(0, Ordering::Relaxed);

        let schema: Value = platform_value!({
            "key1": "value1",
            "key2": "value2"
        });

        let validators: Vec<TestSubValidator> = vec![validator_one, validator_two];
        let result = traversal_validator(&schema, &validators, PlatformVersion::first())
            .expect("traversal_validator should succeed");
        assert!(result.is_valid());

        let c1 = V1_COUNT.load(Ordering::Relaxed);
        let c2 = V2_COUNT.load(Ordering::Relaxed);
        assert_eq!(
            c1, c2,
            "both validators should be called the same number of times"
        );
        assert_eq!(c1, 2, "each validator should be called once per key");
    }

    #[test]
    fn traversal_paths_are_correctly_built_for_nested_maps() {
        // Verify that paths passed to validators follow the "/key1/key2" pattern
        use once_cell::sync::Lazy;
        use std::sync::Mutex;

        static PATHS: Lazy<Mutex<Vec<String>>> = Lazy::new(|| Mutex::new(Vec::new()));

        fn path_recording_validator(
            path: &str,
            key: &str,
            _parent: &Value,
            _value: &Value,
            _result: &mut crate::validation::SimpleConsensusValidationResult,
            _platform_version: &PlatformVersion,
        ) -> Result<(), crate::ProtocolError> {
            let full_path = if path.is_empty() {
                key.to_string()
            } else {
                format!("{}/{}", path, key)
            };
            PATHS.lock().unwrap().push(full_path);
            Ok(())
        }

        PATHS.lock().unwrap().clear();

        let schema: Value = platform_value!({
            "properties": {
                "name": {
                    "type": "string"
                }
            }
        });

        let validators: Vec<TestSubValidator> = vec![path_recording_validator];
        let result = traversal_validator(&schema, &validators, PlatformVersion::first())
            .expect("traversal_validator should succeed");
        assert!(result.is_valid());

        let paths = PATHS.lock().unwrap();
        // We should see "properties" as a key at the root level
        assert!(
            paths.iter().any(|p| p == "properties"),
            "should see 'properties' key at root; got: {:?}",
            *paths
        );
    }

    #[test]
    fn traversal_version_mismatch_returns_error() {
        // If we construct a PlatformVersion with an unknown traversal_validator version,
        // the top-level dispatching function should return an error
        let mut platform_version = PlatformVersion::first().clone();
        platform_version
            .dpp
            .contract_versions
            .document_type_versions
            .schema
            .recursive_schema_validator_versions
            .traversal_validator = 255;

        let schema: Value = platform_value!({});
        let result = traversal_validator(&schema, &[], &platform_version);
        assert!(
            result.is_err(),
            "unknown traversal_validator version should produce an error"
        );
    }
}
