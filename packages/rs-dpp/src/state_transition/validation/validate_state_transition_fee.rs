use anyhow::Context;

use crate::{
    consensus::fee::FeeError,
    identity::{
        convert_satoshi_to_credits,
        state_transition::asset_lock_proof::AssetLockTransactionOutputFetcher,
    },
    prelude::Identity,
    state_repository::StateRepositoryLike,
    state_transition::{StateTransition, StateTransitionIdentitySigned, StateTransitionLike},
    validation::SimpleValidationResult,
    ProtocolError,
};
use std::sync::Arc;

pub struct StateTransitionFeeValidator<SR: StateRepositoryLike> {
    state_repository: Arc<SR>,
    asset_lock_transition_output_fetcher: AssetLockTransactionOutputFetcher<SR>,
}

impl<SR> StateTransitionFeeValidator<SR>
where
    SR: StateRepositoryLike,
{
    fn new(state_repository: Arc<SR>) -> Self {
        let asset_lock_transition_output_fetcher =
            AssetLockTransactionOutputFetcher::new(state_repository.clone());
        StateTransitionFeeValidator {
            state_repository,
            asset_lock_transition_output_fetcher,
        }
    }

    pub async fn validate(
        &self,
        state_transition: &StateTransition,
    ) -> Result<SimpleValidationResult, ProtocolError> {
        let mut result = SimpleValidationResult::default();

        let execution_context = state_transition.get_execution_context();
        let balance = match state_transition {
            StateTransition::IdentityCreate(st) => {
                let output = self
                    .asset_lock_transition_output_fetcher
                    .fetch(st.get_asset_lock_proof(), execution_context)
                    .await
                    .with_context(|| {
                        format!(
                            "unable to fetch asset lock transition output for: {:?}",
                            st.get_asset_lock_proof()
                        )
                    })?;
                convert_satoshi_to_credits(output.value)
            }
            StateTransition::IdentityTopUp(st) => {
                let output = self
                    .asset_lock_transition_output_fetcher
                    .fetch(st.get_asset_lock_proof(), execution_context)
                    .await
                    .with_context(|| {
                        format!(
                            "unable to fetch asset lock transition output for: {:?}",
                            st.get_asset_lock_proof()
                        )
                    })?;
                let balance = convert_satoshi_to_credits(output.value);
                let identity_id = st.get_owner_id();
                let identity: Identity = self
                    .state_repository
                    .fetch_identity(identity_id, execution_context)
                    .await?
                    .with_context(|| format!("identity with ID {}' doesn't exist", identity_id))?;
                balance + identity.get_balance()
            }
            StateTransition::DataContractCreate(st) => self.get_identity_owner_balance(st).await?,
            StateTransition::DataContractUpdate(st) => self.get_identity_owner_balance(st).await?,
            StateTransition::DocumentsBatch(st) => self.get_identity_owner_balance(st).await?,
            StateTransition::IdentityUpdate(st) => self.get_identity_owner_balance(st).await?,
            StateTransition::IdentityCreditWithdrawal(_) => {
                return Err(ProtocolError::InvalidStateTransitionTypeError);
            }
        };

        let fee = state_transition.calculate_fee();
        // ? make sure Fee cannot be negative and refunds are handled differently
        if (balance as i64) < fee {
            result.add_error(FeeError::BalanceIsNotEnoughError { balance, fee })
        }

        Ok(result)
    }

    async fn get_identity_owner_balance(
        &self,
        st: &impl StateTransitionIdentitySigned,
    ) -> Result<u64, ProtocolError> {
        let identity_id = st.get_owner_id();
        let identity: Identity = self
            .state_repository
            .fetch_identity(identity_id, st.get_execution_context())
            .await?
            .with_context(|| format!("identity with ID {}' doesn't exist", identity_id))?;

        Ok(identity.get_balance())
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use crate::state_transition::StateTransitionLike;
    use crate::tests::fixtures::identity_topup_transition_fixture_json;
    use crate::ProtocolError;
    use crate::{
        consensus::fee::FeeError,
        data_contract::state_transition::DataContractCreateTransition,
        document::{document_transition::Action, DocumentsBatchTransition},
        identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition,
        state_repository::MockStateRepositoryLike,
        state_transition::{
            fee::operations::{Operation, PreCalculatedOperation},
            state_transition_execution_context::StateTransitionExecutionContext,
        },
        tests::{
            fixtures::{
                get_data_contract_fixture, get_document_transitions_fixture,
                get_documents_fixture_with_owner_id_from_contract, identity_fixture,
            },
            utils::get_fee_error_from_result,
        },
    };

    use super::StateTransitionFeeValidator;

    fn execution_context_with_cost(
        storage_cost: i64,
        processing_cost: i64,
    ) -> StateTransitionExecutionContext {
        let mut ctx = StateTransitionExecutionContext::default();
        ctx.add_operation(Operation::PreCalculated(PreCalculatedOperation::new(
            storage_cost,
            processing_cost,
        )));
        ctx
    }

    #[tokio::test]
    async fn data_contract_crate_transition_invalid_result_if_balance_is_not_enough() {
        let mut identity = identity_fixture();
        let mut state_repository_mock = MockStateRepositoryLike::new();

        identity.balance = 1;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));

        let data_contract = get_data_contract_fixture(None);
        let data_contract_create_transition = DataContractCreateTransition {
            entropy: data_contract.entropy().to_owned(),
            data_contract,
            execution_context: execution_context_with_cost(40, 5),
            ..Default::default()
        };

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate(&data_contract_create_transition.into())
            .await
            .expect("the validation result should be returned");

        let fee_error = get_fee_error_from_result(&result, 0);
        assert!(
            matches!(fee_error, FeeError::BalanceIsNotEnoughError { balance, fee } if {
                *balance == 1 &&
                *fee == 90
            })
        );
    }

    #[tokio::test]
    async fn data_contract_crate_transition_should_return_valid_result() {
        let mut identity = identity_fixture();
        let mut state_repository_mock = MockStateRepositoryLike::new();

        identity.balance = 90;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));

        let data_contract = get_data_contract_fixture(None);
        let data_contract_create_transition = DataContractCreateTransition {
            entropy: data_contract.entropy().to_owned(),
            data_contract,
            execution_context: execution_context_with_cost(40, 5),
            ..Default::default()
        };

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate(&data_contract_create_transition.into())
            .await
            .expect("the validation result should be returned");
        assert!(result.is_valid())
    }

    #[tokio::test]
    async fn documents_batch_transition_invalid_result_if_balance_is_not_enough() {
        let mut identity = identity_fixture();
        let mut state_repository_mock = MockStateRepositoryLike::new();

        identity.balance = 1;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));

        let data_contract = get_data_contract_fixture(None);
        let documents =
            get_documents_fixture_with_owner_id_from_contract(data_contract.clone()).unwrap();
        let transitions = get_document_transitions_fixture([(Action::Create, documents)]);
        let documents_batch_transition = DocumentsBatchTransition {
            owner_id: data_contract.owner_id().to_owned(),
            transitions,
            execution_context: execution_context_with_cost(40, 5),
            ..Default::default()
        };

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate(&documents_batch_transition.into())
            .await
            .expect("the validation result should be returned");

        let fee_error = get_fee_error_from_result(&result, 0);
        assert!(
            matches!(fee_error, FeeError::BalanceIsNotEnoughError { balance, fee } if {
                *balance == 1 &&
                *fee == 90
            })
        );
    }

    #[tokio::test]
    async fn documents_batch_transition_should_return_valid_result() {
        let mut identity = identity_fixture();
        let mut state_repository_mock = MockStateRepositoryLike::new();

        identity.balance = 90;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));

        let data_contract = get_data_contract_fixture(None);
        let documents =
            get_documents_fixture_with_owner_id_from_contract(data_contract.clone()).unwrap();
        let transitions = get_document_transitions_fixture([(Action::Create, documents)]);
        let documents_batch_transition = DocumentsBatchTransition {
            owner_id: data_contract.owner_id().to_owned(),
            transitions,
            execution_context: execution_context_with_cost(40, 5),
            ..Default::default()
        };

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate(&documents_batch_transition.into())
            .await
            .expect("the validation result should be returned");
        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn identity_top_up_transition_should_return_invalid_result_if_balance_is_not_enough() {
        let mut identity = identity_fixture();
        let mut state_repository_mock = MockStateRepositoryLike::new();
        identity.balance = 1;
        state_repository_mock
            .expect_fetch_identity()
            .returning(move |_, _| Ok(Some(identity.clone())));

        let mut identity_topup_transition =
            IdentityTopUpTransition::new(identity_topup_transition_fixture_json(None)).unwrap();
        identity_topup_transition.set_execution_context(execution_context_with_cost(45000000, 5));

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate(&identity_topup_transition.into())
            .await
            .expect("the validation result should be returned");
        let fee_error = get_fee_error_from_result(&result, 0);
        assert!(
            matches!(fee_error, FeeError::BalanceIsNotEnoughError { balance, fee } if {
                *balance == 90000001 &&
                *fee == 90000010
            })
        );
    }

    #[tokio::test]
    async fn should_return_invalid_state_transition_type() {
        let transition = IdentityCreditWithdrawalTransition::default();
        let state_repository_mock = MockStateRepositoryLike::new();
        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));

        let result = validator
            .validate(&transition.into())
            .await
            .expect_err("error should be returned");
        assert!(matches!(
            result,
            ProtocolError::InvalidStateTransitionTypeError
        ))
    }
}
