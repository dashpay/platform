use getrandom::getrandom;
use crate::errors::consensus::basic::JsonSchemaError;
use crate::errors::consensus::ConsensusError;
use crate::validation::ValidationResult;

pub fn generate_random_identifier() -> [u8; 32] {
    let mut buffer = [0u8; 32];
    getrandom(&mut buffer);
    buffer
}

/// Checks the variant of all errors in the array, asserts error count and
/// returns unwrapped errors
pub fn assert_json_schema_error(result: &ValidationResult, expected_errors_count: usize) -> Vec<&JsonSchemaError> {
    let error_count = result.errors().len();

    assert_eq!(error_count, expected_errors_count);

    let mut errors: Vec<&JsonSchemaError> = Vec::new();

    for error in result.errors() {
        match error {
            ConsensusError::JsonSchemaError(err) => { errors.push(err) }
            _ => panic!("Expected JsonSchemaError")
        }
    }

    errors
}

/// Sets a key value pair in serde_json object, returns the modified object
pub fn serde_set<T, S>(mut object: serde_json::Value, key: T, value: S) -> serde_json::Value
    where T: Into<String>, S: Into<serde_json::Value>, serde_json::Value: From<S>
{
    let map = object
        .as_object_mut()
        .expect("Expected value to be an JSON object");
    map.insert(key.into(), serde_json::Value::from(value));

    object
}

/// Removes a key value pair in serde_json object, returns the modified object
pub fn serde_remove<T>(mut object: serde_json::Value, key: T) -> serde_json::Value
    where T: Into<String>
{
    let map = object
        .as_object_mut()
        .expect("Expected value to be an JSON object");
    map.remove(&key.into());

    object
}