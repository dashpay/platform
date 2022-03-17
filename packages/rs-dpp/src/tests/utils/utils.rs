use getrandom::getrandom;
use crate::errors::consensus::ConsensusError;
use crate::validation::ValidationResult;

pub fn generate_random_identifier() -> [u8; 32] {
    let mut buffer = [0u8; 32];
    getrandom(&mut buffer);
    buffer
}

pub fn assert_json_schema_error(result: &ValidationResult, expected_errors_count: usize) {
    let error_count = result.errors().len();

    assert_eq!(error_count, expected_errors_count);

    for error in result.errors() {
        match error {
            ConsensusError::JsonSchemaError(_) => {}
            _ => panic!("Expected JsonSchemaError")
        }
    }
}