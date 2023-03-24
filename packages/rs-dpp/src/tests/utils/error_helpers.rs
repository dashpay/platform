use crate::{
    consensus::{
        basic::{BasicError, IndexError, JsonSchemaError},
        fee::FeeError,
        signature::SignatureError,
        ConsensusError,
    },
    data_trigger::DataTriggerExecutionResult,
    validation::ValidationResult,
    DataTriggerError, StateError,
};

pub fn get_schema_error(result: &ValidationResult<()>, number: usize) -> &JsonSchemaError {
    result
        .consensus_errors
        .get(number)
        .expect("the error should be returned in validation result")
        .json_schema_error()
        .expect("the error should be json schema error")
}

pub fn get_basic_error(consensus_error: &ConsensusError) -> &BasicError {
    match consensus_error {
        ConsensusError::BasicError(basic_error) => basic_error,
        _ => panic!("error '{:?}' isn't a basic error", consensus_error),
    }
}

pub fn get_index_error(consensus_error: &ConsensusError) -> &IndexError {
    match consensus_error {
        ConsensusError::BasicError(basic_error) => match &**basic_error {
            BasicError::IndexError(index_error) => index_error,
            _ => panic!("error '{:?}' isn't a index error", consensus_error),
        },
        _ => panic!("error '{:?}' isn't a basic error", consensus_error),
    }
}

pub fn get_state_error_from_result(
    result: &ValidationResult<()>,
    error_number: usize,
) -> &StateError {
    match result
        .consensus_errors
        .get(error_number)
        .expect("error should be found")
    {
        ConsensusError::StateError(state_error) => state_error,
        _ => panic!(
            "error '{:?}' isn't a state error",
            result.consensus_errors[error_number]
        ),
    }
}

pub fn get_basic_error_from_result(
    result: &ValidationResult<()>,
    error_number: usize,
) -> &BasicError {
    match result
        .consensus_errors
        .get(error_number)
        .expect("basic error should be found")
    {
        ConsensusError::BasicError(basic_error) => basic_error,
        _ => panic!(
            "error '{:?}' isn't a Basic error",
            result.consensus_errors[error_number]
        ),
    }
}

pub fn get_signature_error_from_result<K: Clone>(
    result: &ValidationResult<K>,
    error_number: usize,
) -> &SignatureError {
    match result
        .consensus_errors
        .get(error_number)
        .expect("error should be found")
    {
        ConsensusError::SignatureError(signature_error) => signature_error,
        _ => panic!(
            "error '{:?}' isn't a Signature error",
            result.consensus_errors[error_number]
        ),
    }
}

pub fn get_fee_error_from_result<K: Clone>(
    result: &ValidationResult<K>,
    error_number: usize,
) -> &FeeError {
    match result
        .consensus_errors
        .get(error_number)
        .expect("error should be found")
    {
        ConsensusError::FeeError(signature_error) => signature_error,
        _ => panic!(
            "error '{:?}' isn't a Fee error",
            result.consensus_errors[error_number]
        ),
    }
}

pub fn get_data_trigger_error_from_execution_result(
    result: &DataTriggerExecutionResult,
    error_number: usize,
) -> &DataTriggerError {
    match result
        .errors
        .get(error_number)
        .expect("basic error should be found")
    {
        StateError::DataTriggerError(error) => error,
        _ => panic!(
            "error '{:?}' isn't a Data Trigger error",
            result.errors[error_number]
        ),
    }
}
