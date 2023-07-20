use crate::consensus::basic::json_schema_error::JsonSchemaError;
use crate::consensus::basic::value_error::ValueError;
use crate::consensus::basic::BasicError;
use crate::consensus::fee::fee_error::FeeError;
use crate::consensus::signature::SignatureError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::validation::{ConsensusValidationResult, SimpleConsensusValidationResult};

pub fn get_schema_error(
    result: &SimpleConsensusValidationResult,
    number: usize,
) -> &JsonSchemaError {
    json_schema_error(
        result
            .errors
            .get(number)
            .expect("the error should be returned in validation result"),
    )
}

pub fn get_basic_error(consensus_error: &ConsensusError) -> &BasicError {
    match consensus_error {
        ConsensusError::BasicError(basic_error) => basic_error,
        _ => panic!("error '{:?}' isn't a basic error", consensus_error),
    }
}

// TODO: Not sure it should be here. Looks more like a test helper
pub fn json_schema_error(consensus_error: &ConsensusError) -> &JsonSchemaError {
    match consensus_error {
        ConsensusError::BasicError(BasicError::JsonSchemaError(err)) => err,
        _ => panic!("error '{:?}' isn't a json schema error", consensus_error),
    }
}

pub fn value_error(consensus_error: &ConsensusError) -> &ValueError {
    match consensus_error {
        ConsensusError::BasicError(BasicError::ValueError(err)) => err,
        _ => panic!("error '{:?}' isn't a value error", consensus_error),
    }
}

pub fn get_state_error_from_result<TData: Clone>(
    result: &ConsensusValidationResult<TData>,
    error_number: usize,
) -> &StateError {
    match result
        .errors
        .get(error_number)
        .expect("error should be found")
    {
        ConsensusError::StateError(state_error) => state_error,
        _ => panic!(
            "error '{:?}' isn't a state error",
            result.errors[error_number]
        ),
    }
}

pub fn get_basic_error_from_result(
    result: &SimpleConsensusValidationResult,
    error_number: usize,
) -> &BasicError {
    match result
        .errors
        .get(error_number)
        .expect("basic error should be found")
    {
        ConsensusError::BasicError(basic_error) => basic_error,
        _ => panic!(
            "error '{:?}' isn't a Basic error",
            result.errors[error_number]
        ),
    }
}

pub fn get_signature_error_from_result<K: Clone>(
    result: &ConsensusValidationResult<K>,
    error_number: usize,
) -> &SignatureError {
    match result
        .errors
        .get(error_number)
        .expect("error should be found")
    {
        ConsensusError::SignatureError(signature_error) => signature_error,
        _ => panic!(
            "error '{:?}' isn't a Signature error",
            result.errors[error_number]
        ),
    }
}

pub fn get_fee_error_from_result<K: Clone>(
    result: &ConsensusValidationResult<K>,
    error_number: usize,
) -> &FeeError {
    match result
        .errors
        .get(error_number)
        .expect("error should be found")
    {
        ConsensusError::FeeError(signature_error) => signature_error,
        _ => panic!(
            "error '{:?}' isn't a Fee error",
            result.errors[error_number]
        ),
    }
}
