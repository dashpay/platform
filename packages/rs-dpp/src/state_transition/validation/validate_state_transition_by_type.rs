use async_trait::async_trait;

#[cfg(test)]
use mockall::{automock, predicate::*};

use serde_json::Value as JsonValue;

use crate::{
    state_transition::{
        state_transition_execution_context::StateTransitionExecutionContext, StateTransitionType,
    },
    validation::{AsyncDataValidatorWithContext, SimpleValidationResult, ValidationResult},
    ProtocolError,
};

#[cfg_attr(test, automock)]
#[async_trait(?Send)]
pub trait ValidatorByStateTransitionType {
    async fn validate(
        &self,
        raw_state_transition: &JsonValue,
        state_transition_type: StateTransitionType,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleValidationResult, ProtocolError>;
}

pub struct StateTransitionByTypeValidator<ADV>
where
    ADV: AsyncDataValidatorWithContext<Item = JsonValue>,
{
    data_contract_create_validator: ADV,
    data_contract_update_validator: ADV,
    identity_create_validator: ADV,
    identity_update_validator: ADV,
    identity_top_up_validator: ADV,
    identity_credit_withdrawal_validator: ADV,
    document_batch_validator: ADV,
}

impl<ADV> StateTransitionByTypeValidator<ADV>
where
    ADV: AsyncDataValidatorWithContext<Item = JsonValue>,
{
    pub fn new(
        data_contract_create_validator: ADV,
        data_contract_update_validator: ADV,
        identity_create_validator: ADV,
        identity_update_validator: ADV,
        identity_top_up_validator: ADV,
        identity_credit_withdrawal_validator: ADV,
        document_batch_validator: ADV,
    ) -> Self {
        StateTransitionByTypeValidator {
            data_contract_create_validator,
            data_contract_update_validator,
            identity_create_validator,
            identity_update_validator,
            identity_top_up_validator,
            identity_credit_withdrawal_validator,
            document_batch_validator,
        }
    }
}

#[async_trait(?Send)]
impl<ADV> ValidatorByStateTransitionType for StateTransitionByTypeValidator<ADV>
where
    ADV: AsyncDataValidatorWithContext<Item = JsonValue> + Send + Sync,
{
    async fn validate(
        &self,
        raw_state_transition: &JsonValue,
        state_transition_type: StateTransitionType,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleValidationResult, ProtocolError> {
        let mut result = ValidationResult::default();

        let validator = match state_transition_type {
            StateTransitionType::DataContractCreate => &self.data_contract_create_validator,
            StateTransitionType::DataContractUpdate => &self.data_contract_update_validator,
            StateTransitionType::IdentityCreate => &self.identity_create_validator,
            StateTransitionType::IdentityUpdate => &self.identity_update_validator,
            StateTransitionType::IdentityTopUp => &self.identity_top_up_validator,
            StateTransitionType::IdentityCreditWithdrawal => {
                &self.identity_credit_withdrawal_validator
            }
            StateTransitionType::DocumentsBatch => &self.document_batch_validator,
        };

        let validation_result = validator
            .validate(raw_state_transition, execution_context)
            .await?;

        result.merge(validation_result);

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::Value as JsonValue;

    use crate::{
        data_contract::{
            state_transition::DataContractCreateTransition,
            validation::data_contract_validator::DataContractValidator, DataContract,
            DataContractFactory,
        },
        state_transition::{
            state_transition_execution_context::StateTransitionExecutionContext,
            try_get_transition_type, StateTransitionConvert, StateTransitionLike,
        },
        tests::fixtures::get_data_contract_fixture,
        validation::{MockAsyncDataValidatorWithContext, ValidationResult},
        version::{ProtocolVersionValidator, COMPATIBILITY_MAP, LATEST_VERSION},
        NativeBlsModule,
    };

    use crate::state_transition::validation::validate_state_transition_by_type::ValidatorByStateTransitionType;

    use super::StateTransitionByTypeValidator;

    struct TestData {
        data_contract: DataContract,
        state_transition: DataContractCreateTransition,
        raw_state_transition: JsonValue,
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
    async fn test_correct_validator_is_selected() {
        let TestData {
            raw_state_transition,
            ..
        } = setup_test();

        let mut data_contract_create_validator_mock = MockAsyncDataValidatorWithContext::new();
        let data_contract_update_validator_mock = MockAsyncDataValidatorWithContext::new();
        let identity_create_validator_mock = MockAsyncDataValidatorWithContext::new();
        let identity_update_validator_mock = MockAsyncDataValidatorWithContext::new();
        let identity_top_up_validator_mock = MockAsyncDataValidatorWithContext::new();
        let identity_credit_withdrawal_validator_mock = MockAsyncDataValidatorWithContext::new();
        let document_batch_validator_mock = MockAsyncDataValidatorWithContext::new();

        data_contract_create_validator_mock
            .expect_validate()
            .times(1)
            .returning(|_, _| Ok(ValidationResult::default()));

        let validator = StateTransitionByTypeValidator::new(
            data_contract_create_validator_mock,
            data_contract_update_validator_mock,
            identity_create_validator_mock,
            identity_update_validator_mock,
            identity_top_up_validator_mock,
            identity_credit_withdrawal_validator_mock,
            document_batch_validator_mock,
        );

        let execution_context = StateTransitionExecutionContext::default();

        let state_transition_type =
            try_get_transition_type(&raw_state_transition).expect("to get state transition type");

        validator
            .validate(
                &raw_state_transition,
                state_transition_type,
                &execution_context,
            )
            .await
            .expect("expected to call basic validator `validate` function");
    }
}
