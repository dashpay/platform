use std::convert::TryFrom;

use async_trait::async_trait;
#[cfg(test)]
use mockall::{automock, predicate::*};
use serde_json::Value as JsonValue;

use crate::{
    consensus::basic::BasicError,
    state_repository::StateRepositoryLike,
    state_transition::{create_state_transition, StateTransitionConvert, StateTransitionType},
    util::json_value::JsonValueExt,
    validation::SimpleValidationResult,
    ProtocolError,
};

async fn validate_state_transition_basic(
    state_repository: &impl StateRepositoryLike,
    validate_functions_by_type: &impl ValidatorByStateTransitionType,
    raw_state_transition: JsonValue,
) -> Result<SimpleValidationResult, ProtocolError> {
    let mut result = SimpleValidationResult::default();

    let raw_transition_type = match raw_state_transition.get_u64("type") {
        Err(_) => {
            result.add_error(BasicError::MissingStateTransitionTypeError);
            return Ok(result);
        }

        Ok(transaction_type) => transaction_type,
    } as u8;

    let state_transition_type = match StateTransitionType::try_from(raw_transition_type) {
        Err(_) => {
            result.add_error(BasicError::InvalidStateTransitionTypeError {
                transition_type: raw_transition_type,
            });
            return Ok(result);
        }
        Ok(transition_type) => transition_type,
    };

    let validate_result = validate_functions_by_type
        .validate(&raw_state_transition, state_transition_type)
        .await?;
    result.merge(validate_result);
    if !result.is_valid() {
        return Ok(result);
    }

    let state_transition = create_state_transition(state_repository, raw_state_transition).await?;
    if let Err(ProtocolError::MaxEncodedBytesReachedError {
        max_size_kbytes,
        payload,
    }) = state_transition.to_buffer(false)
    {
        result.add_error(BasicError::StateTransitionMaxSizeExceededError {
            actual_size_kbytes: payload.len() / 1024,
            max_size_kbytes,
        });
    }

    Ok(result)
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait ValidatorByStateTransitionType {
    async fn validate(
        &self,
        raw_state_transition: &JsonValue,
        state_transition_type: StateTransitionType,
    ) -> Result<SimpleValidationResult, ProtocolError>;
}

#[cfg(test)]
mod test {
    use serde_json::{json, Value as JsonValue};
    use std::sync::Arc;

    use crate::{
        consensus::basic::BasicError,
        data_contract::{
            state_transition::DataContractCreateTransition,
            validation::data_contract_validator::DataContractValidator, DataContract,
            DataContractFactory,
        },
        state_repository::MockStateRepositoryLike,
        state_transition::{StateTransitionConvert, StateTransitionLike},
        tests::{fixtures::get_data_contract_fixture, utils::get_basic_error_from_result},
        util::json_value::JsonValueExt,
        validation::ValidationResult,
        version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
    };

    use super::{validate_state_transition_basic, MockValidatorByStateTransitionType};

    struct TestData {
        data_contract: DataContract,
        state_transition: DataContractCreateTransition,
        raw_state_transition: JsonValue,
    }

    fn setup_test() -> TestData {
        let data_contract = get_data_contract_fixture(None);
        let private_key_bytes =
            hex::decode("9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2")
                .unwrap();

        let protocol_version_validator = ProtocolVersionValidator::new(
            LATEST_VERSION,
            LATEST_VERSION,
            COMPATIBILITY_MAP.clone(),
        );
        let data_contract_validator =
            DataContractValidator::new(Arc::new(protocol_version_validator));
        let data_contract_factory = DataContractFactory::new(1, data_contract_validator);

        let mut state_transition = data_contract_factory
            .create_data_contract_create_transition(data_contract.clone())
            .unwrap();

        state_transition
            .sign_by_private_key(
                &private_key_bytes,
                crate::identity::KeyType::ECDSA_SECP256K1,
            )
            .expect("the state transition should be signed");

        let raw_state_transition = state_transition.to_object(false).unwrap();

        TestData {
            data_contract,
            state_transition,
            raw_state_transition,
        }
    }

    #[tokio::test]
    async fn should_return_invalid_result_if_state_transition_type_is_missing() {
        let TestData {
            mut raw_state_transition,
            ..
        } = setup_test();

        let state_repository_mock = MockStateRepositoryLike::new();
        let validate_by_type_mock = MockValidatorByStateTransitionType::new();

        raw_state_transition
            .remove("type")
            .expect("type should exist and be remove");

        let result = validate_state_transition_basic(
            &state_repository_mock,
            &validate_by_type_mock,
            raw_state_transition,
        )
        .await
        .expect("the validation result should be returned");
        let basic_error = get_basic_error_from_result(&result, 0);

        assert!(matches!(
            basic_error,
            BasicError::MissingStateTransitionTypeError
        ));
    }

    #[tokio::test]
    async fn should_return_invalid_result_if_state_transition_type_is_not_valid() {
        let TestData {
            mut raw_state_transition,
            ..
        } = setup_test();

        let state_repository_mock = MockStateRepositoryLike::new();
        let validate_by_type_mock = MockValidatorByStateTransitionType::new();

        raw_state_transition["type"] = json!(123);

        let result = validate_state_transition_basic(
            &state_repository_mock,
            &validate_by_type_mock,
            raw_state_transition,
        )
        .await
        .expect("the validation result should be returned");
        let basic_error = get_basic_error_from_result(&result, 0);

        assert!(matches!(
            basic_error,
            BasicError::InvalidStateTransitionTypeError { transition_type } if { transition_type == &123}
        ));
    }

    #[tokio::test]
    async fn should_return_invalid_result_if_state_transition_is_invalid_against_validation_function(
    ) {
        let TestData {
            mut raw_state_transition,
            ..
        } = setup_test();

        let state_repository_mock = MockStateRepositoryLike::new();
        let validate_by_type_mock = MockValidatorByStateTransitionType::new();

        raw_state_transition["type"] = json!(123);

        let result = validate_state_transition_basic(
            &state_repository_mock,
            &validate_by_type_mock,
            raw_state_transition,
        )
        .await
        .expect("the validation result should be returned");
        let basic_error = get_basic_error_from_result(&result, 0);

        assert!(matches!(
            basic_error,
            BasicError::InvalidStateTransitionTypeError { transition_type } if { transition_type == &123}
        ));
    }

    #[tokio::test]
    async fn should_return_invalid_result_if_state_transition_size_is_more_than_16_kb() {
        let TestData {
            mut raw_state_transition,
            ..
        } = setup_test();

        let state_repository_mock = MockStateRepositoryLike::new();
        let mut validate_by_type_mock = MockValidatorByStateTransitionType::new();
        validate_by_type_mock
            .expect_validate()
            .returning(|_, _| Ok(ValidationResult::<()>::default()));

        for i in 0..500 {
            let document_type_name = format!("anotherDocument{}", i);
            raw_state_transition["dataContract"]["documents"][document_type_name] =
                raw_state_transition["dataContract"]["documents"]["niceDocument"].clone();
        }

        let result = validate_state_transition_basic(
            &state_repository_mock,
            &validate_by_type_mock,
            raw_state_transition,
        )
        .await
        .expect("the validation result should be returned");

        let basic_error = get_basic_error_from_result(&result, 0);

        assert!(matches!(
            basic_error,
            BasicError::StateTransitionMaxSizeExceededError { actual_size_kbytes, max_size_kbytes} if {
                *actual_size_kbytes == 53 &&
                *max_size_kbytes == 16
            }
        ));
    }

    #[tokio::test]
    async fn should_return_valid_result() {
        let TestData {
            raw_state_transition,
            ..
        } = setup_test();

        let state_repository_mock = MockStateRepositoryLike::new();
        let mut validate_by_type_mock = MockValidatorByStateTransitionType::new();
        validate_by_type_mock
            .expect_validate()
            .returning(|_, _| Ok(ValidationResult::<()>::default()));

        let result = validate_state_transition_basic(
            &state_repository_mock,
            &validate_by_type_mock,
            raw_state_transition,
        )
        .await
        .expect("should return validation result");

        assert!(result.is_valid());
    }
}
