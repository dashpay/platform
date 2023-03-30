use std::{convert::TryFrom, sync::Arc};

use async_trait::async_trait;
use platform_value::Value;

use crate::consensus::basic::state_transition::{
    InvalidStateTransitionTypeError, StateTransitionMaxSizeExceededError,
};
use crate::{
    consensus::{basic::BasicError, ConsensusError},
    state_repository::StateRepositoryLike,
    state_transition::{
        create_state_transition,
        state_transition_execution_context::StateTransitionExecutionContext,
        StateTransitionConvert, StateTransitionType,
    },
    validation::{AsyncDataValidatorWithContext, SimpleValidationResult},
    ProtocolError,
};

use super::validate_state_transition_by_type::ValidatorByStateTransitionType;

pub struct StateTransitionBasicValidator<SR, VBT>
where
    SR: StateRepositoryLike,
    VBT: ValidatorByStateTransitionType,
{
    state_repository: Arc<SR>,
    validate_state_transition_by_type: VBT,
}

impl<SR, VBT> StateTransitionBasicValidator<SR, VBT>
where
    SR: StateRepositoryLike,
    VBT: ValidatorByStateTransitionType,
{
    pub fn new(state_repository: Arc<SR>, validate_state_transition_by_type: VBT) -> Self {
        StateTransitionBasicValidator {
            state_repository,
            validate_state_transition_by_type,
        }
    }
}

#[async_trait(?Send)]
impl<SR, VBT> AsyncDataValidatorWithContext for StateTransitionBasicValidator<SR, VBT>
where
    SR: StateRepositoryLike,
    VBT: ValidatorByStateTransitionType,
{
    type Item = Value;

    async fn validate(
        &self,
        raw_state_transition: &Value,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleValidationResult, ProtocolError> {
        let mut result = SimpleValidationResult::default();

        let Ok(state_transition_type) = raw_state_transition.get_integer("type") else {
            result.add_error(
                ConsensusError::BasicError(
                    BasicError::MissingStateTransitionTypeError
                )
            );

            return Ok(result);
        };

        let Ok(state_transition_type) = StateTransitionType::try_from(state_transition_type) else {
            result.add_error(
                ConsensusError::BasicError(
                        BasicError::InvalidStateTransitionTypeError(InvalidStateTransitionTypeError::new(state_transition_type))
                )
            );

            return Ok(result);
        };

        let validate_result = self
            .validate_state_transition_by_type
            .validate(
                raw_state_transition,
                state_transition_type,
                execution_context,
            )
            .await?;

        result.merge(validate_result);

        if !result.is_valid() {
            return Ok(result);
        }

        let state_transition =
            create_state_transition(self.state_repository.as_ref(), raw_state_transition.clone())
                .await?;

        if let Err(ProtocolError::MaxEncodedBytesReachedError {
            max_size_kbytes,
            payload,
        }) = state_transition.to_buffer(false)
        {
            result.add_error(BasicError::StateTransitionMaxSizeExceededError(
                StateTransitionMaxSizeExceededError::new(payload.len() / 1024, max_size_kbytes),
            ));
        }

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use platform_value::{platform_value, Value};
    use std::sync::Arc;

    use crate::{
        data_contract::state_transition::data_contract_create_transition::DataContractCreateTransition,
        state_transition::validation::validate_state_transition_by_type::MockValidatorByStateTransitionType,
        validation::AsyncDataValidatorWithContext,
    };

    use super::StateTransitionBasicValidator;

    use crate::validation::SimpleValidationResult;
    use crate::{
        consensus::basic::BasicError,
        data_contract::{
            validation::data_contract_validator::DataContractValidator, DataContract,
            DataContractFactory,
        },
        state_repository::MockStateRepositoryLike,
        state_transition::{
            state_transition_execution_context::StateTransitionExecutionContext,
            StateTransitionConvert, StateTransitionLike,
        },
        tests::{fixtures::get_data_contract_fixture, utils::get_basic_error_from_result},
        version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
        NativeBlsModule,
    };

    struct TestData {
        data_contract: DataContract,
        state_transition: DataContractCreateTransition,
        raw_state_transition: Value,
        bls: NativeBlsModule,
    }

    fn setup_test() -> TestData {
        let bls = NativeBlsModule::default();
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
        let data_contract_factory = DataContractFactory::new(1, Arc::new(data_contract_validator));

        let mut state_transition = data_contract_factory
            .create_data_contract_create_transition(data_contract.clone())
            .unwrap();

        state_transition
            .sign_by_private_key(
                &private_key_bytes,
                crate::identity::KeyType::ECDSA_SECP256K1,
                &bls,
            )
            .expect("the state transition should be signed");

        let raw_state_transition = state_transition.to_object(false).unwrap();

        TestData {
            data_contract,
            state_transition,
            raw_state_transition,
            bls,
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

        let validator = StateTransitionBasicValidator::new(
            Arc::new(state_repository_mock),
            validate_by_type_mock,
        );

        let execution_context = StateTransitionExecutionContext::default();

        let result = validator
            .validate(&raw_state_transition, &execution_context)
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

        raw_state_transition["type"] = platform_value!(123u32);

        let validator = StateTransitionBasicValidator::new(
            Arc::new(state_repository_mock),
            validate_by_type_mock,
        );

        let execution_context = StateTransitionExecutionContext::default();

        let result = validator
            .validate(&raw_state_transition, &execution_context)
            .await
            .expect("the validation result should be returned");

        let basic_error = get_basic_error_from_result(&result, 0);

        match basic_error {
            BasicError::InvalidStateTransitionTypeError(err) => {
                assert_eq!(err.transition_type(), 123)
            }
            _ => panic!(
                "Expected InvalidStateTransitionTypeError, got {}",
                basic_error
            ),
        }
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

        raw_state_transition["type"] = platform_value!(123u32);

        let validator = StateTransitionBasicValidator::new(
            Arc::new(state_repository_mock),
            validate_by_type_mock,
        );

        let execution_context = StateTransitionExecutionContext::default();

        let result = validator
            .validate(&raw_state_transition, &execution_context)
            .await
            .expect("the validation result should be returned");

        let basic_error = get_basic_error_from_result(&result, 0);

        match basic_error {
            BasicError::InvalidStateTransitionTypeError(err) => {
                assert_eq!(err.transition_type(), 123)
            }
            _ => panic!(
                "Expected InvalidStateTransitionTypeError, got {}",
                basic_error
            ),
        }
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
            .returning(|_, _, _| Ok(SimpleValidationResult::default()));

        for i in 0..500 {
            let document_type_name = format!("anotherDocument{}", i);
            raw_state_transition["dataContract"]["documents"][document_type_name] =
                raw_state_transition["dataContract"]["documents"]["niceDocument"].clone();
        }

        let validator = StateTransitionBasicValidator::new(
            Arc::new(state_repository_mock),
            validate_by_type_mock,
        );

        let execution_context = StateTransitionExecutionContext::default();

        let result = validator
            .validate(&raw_state_transition, &execution_context)
            .await
            .expect("the validation result should be returned");

        let basic_error = get_basic_error_from_result(&result, 0);

        match basic_error {
            BasicError::StateTransitionMaxSizeExceededError(err) => {
                assert_eq!(err.actual_size_kbytes(), 53);
                assert_eq!(err.max_size_kbytes(), 16);
            }
            _ => panic!(
                "Expected StateTransitionMaxSizeExceededError, got {}",
                basic_error
            ),
        }
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
            .returning(|_, _, _| Ok(SimpleValidationResult::default()));

        let validator = StateTransitionBasicValidator::new(
            Arc::new(state_repository_mock),
            validate_by_type_mock,
        );

        let execution_context = StateTransitionExecutionContext::default();

        let result = validator
            .validate(&raw_state_transition, &execution_context)
            .await
            .expect("should return validation result");

        assert!(result.is_valid());
    }
}
