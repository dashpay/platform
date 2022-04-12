use std::num::ParseIntError;
use anyhow::Result;
use getrandom::getrandom;
use serde_json::Value;
use crate::errors::consensus::basic::JsonSchemaError;
use crate::errors::consensus::ConsensusError;
use crate::validation::ValidationResult;

pub fn generate_random_identifier() -> [u8; 32] {
    let mut buffer = [0u8; 32];
    let _ = getrandom(&mut buffer);
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

pub fn assert_validation_error(result: &ValidationResult, error_to_match: ConsensusError, expected_errors_count: usize) -> Vec<&ConsensusError> {
    let error_count = result.errors().len();

    assert_eq!(error_count, expected_errors_count);

    let mut errors: Vec<&ConsensusError> = Vec::new();

    for error in result.errors() {
        match error {
            error_to_match => { errors.push(error.into()) }
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

/// Sets a key value pair in serde_json object, returns the modified object
pub fn serde_set_ref<T, S>(object: &mut Value, key: T, value: S)
    where T: Into<String>, S: Into<serde_json::Value>, serde_json::Value: From<S>
{
    let map = object
        .as_object_mut()
        .expect("Expected value to be an JSON object");
    map.insert(key.into(), serde_json::Value::from(value));
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

/// Removes a key value pair in serde_json object, returns the modified object
pub fn serde_remove_ref<T>(object: &mut Value, key: T)
    where T: Into<String>
{
    object
        .as_object_mut()
        .expect("Expected value to be an JSON object")
        .remove(&key.into());
}

pub fn get_data_from_file(file_path: &str) -> Result<String> {
    let current_dir = std::env::current_dir()?;
    let file_path = format!("{}/{}", current_dir.display(), file_path);
    let d = std::fs::read_to_string(file_path)?;
    Ok(d)
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, ParseIntError> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}
