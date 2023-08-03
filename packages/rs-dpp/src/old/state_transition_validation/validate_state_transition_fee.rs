use anyhow::Context;

use crate::consensus::basic::state_transition::InvalidStateTransitionTypeError;
use crate::consensus::fee::balance_is_not_enough_error::BalanceIsNotEnoughError;
use crate::consensus::fee::fee_error::FeeError;
use crate::data_contract::errors::IdentityNotPresentError;
use crate::fee::Credits;
use crate::state_transition::fee::calculate_state_transition_fee_factory::calculate_state_transition_fee;
use crate::state_transition::fee::FeeResult;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::state_transition::StateTransitionType;
use crate::NonConsensusError;
use crate::{
    identity::{
        convert_duffs_to_credits,
        state_transition::asset_lock_proof::AssetLockTransactionOutputFetcher,
    },
    state_repository::StateRepositoryLike,
    state_transition::{StateTransition, StateTransitionIdentitySignedV0},
    validation::SimpleConsensusValidationResult,
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
    pub fn new(state_repository: Arc<SR>) -> Self {
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
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        self.validate_with_custom_calculator(
            state_transition,
            calculate_state_transition_fee,
            execution_context,
        )
        .await
    }

    async fn validate_with_custom_calculator(
        &self,
        state_transition: &StateTransition,
        calculate_state_transition_fee_fn: impl Fn(
            &StateTransition,
            &StateTransitionExecutionContext,
        ) -> Result<FeeResult, NonConsensusError>,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = SimpleConsensusValidationResult::default();
        let required_fee = calculate_state_transition_fee_fn(state_transition, execution_context)?;

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
                convert_duffs_to_credits(output.value)?
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
                let balance = convert_duffs_to_credits(output.value)?;
                let identity_id = st.get_owner_id();
                let identity_balance: i64 = self
                    .state_repository
                    .fetch_identity_balance_with_debt(identity_id, Some(execution_context))
                    .await?
                    .ok_or_else(|| {
                        ProtocolError::IdentityNotPresentError(IdentityNotPresentError::new(
                            *identity_id,
                        ))
                    })?;

                if execution_context.is_dry_run() {
                    return Ok(result);
                }

                if identity_balance.is_negative() && identity_balance.unsigned_abs() > balance {
                    result.add_error(BalanceIsNotEnoughError::new(0, required_fee.desired_amount));

                    return Ok(result);
                }

                if identity_balance.is_negative() {
                    balance.checked_sub(identity_balance.unsigned_abs()).ok_or(
                        ProtocolError::Overflow(
                            "can't subtract identity balance from the state transition balance",
                        ),
                    )?
                } else {
                    balance.checked_add(identity_balance as Credits).ok_or(
                        ProtocolError::Overflow(
                            "can't add identity balance to state transition balance",
                        ),
                    )?
                }
            }
            StateTransition::DataContractCreate(st) => {
                let balance = self
                    .get_identity_owner_balance(st, execution_context)
                    .await?;
                if execution_context.is_dry_run() {
                    return Ok(result);
                }
                balance
            }
            StateTransition::DataContractUpdate(st) => {
                let balance = self
                    .get_identity_owner_balance(st, execution_context)
                    .await?;
                if execution_context.is_dry_run() {
                    return Ok(result);
                }
                balance
            }
            StateTransition::DocumentsBatch(st) => {
                let balance = self
                    .get_identity_owner_balance(st, execution_context)
                    .await?;
                if execution_context.is_dry_run() {
                    return Ok(result);
                }
                balance
            }

            StateTransition::IdentityUpdate(st) => {
                let balance = self
                    .get_identity_owner_balance(st, execution_context)
                    .await?;
                if execution_context.is_dry_run() {
                    return Ok(result);
                }
                balance
            }
            StateTransition::IdentityCreditWithdrawal(_) => {
                // TODO: make it as a top-up, but in reverse
                return Err(ProtocolError::InvalidStateTransitionTypeError(
                    InvalidStateTransitionTypeError::new(
                        StateTransitionType::IdentityCreditWithdrawal as u8,
                    ),
                ));
            }

            StateTransition::IdentityCreditTransfer(st) => {
                let balance = self
                    .get_identity_owner_balance(st, execution_context)
                    .await?;
                if execution_context.is_dry_run() {
                    return Ok(result);
                }
                balance
            }
        };

        if execution_context.is_dry_run() {
            return Ok(result);
        }

        // ? make sure Fee cannot be negative and refunds are handled differently
        if balance < required_fee.desired_amount {
            result.add_error(FeeError::BalanceIsNotEnoughError(
                BalanceIsNotEnoughError::new(balance, required_fee.desired_amount),
            ))
        }

        Ok(result)
    }

    async fn get_identity_owner_balance(
        &self,
        st: &impl StateTransitionIdentitySignedV0,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<u64, ProtocolError> {
        let identity_id = st.get_owner_id();
        let maybe_balance = self
            .state_repository
            .fetch_identity_balance(identity_id, Some(execution_context))
            .await
            .map_err(ProtocolError::from)?;

        if let Some(balance) = maybe_balance {
            Ok(balance)
        } else {
            Err(ProtocolError::IdentityNotPresentError(
                IdentityNotPresentError::new(*identity_id),
            ))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::tests::fixtures::{
        identity_create_transition_fixture, identity_topup_transition_fixture,
    };
    use std::sync::Arc;

    use crate::data_contract::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
    use crate::fee::Credits;
    use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
    use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
    use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
    use crate::identity::RATIO;
    use crate::state_transition::fee::FeeResult;
    use crate::state_transition::StateTransition;
    use crate::ProtocolError;
    use crate::{
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
                get_extended_documents_fixture_with_owner_id_from_contract, identity_fixture,
            },
            utils::get_fee_error_from_result,
        },
    };

    use super::StateTransitionFeeValidator;

    fn execution_context_with_cost(
        storage_cost: Credits,
        processing_cost: Credits,
    ) -> StateTransitionExecutionContext {
        let ctx = StateTransitionExecutionContext::default();
        ctx.add_operation(Operation::PreCalculated(PreCalculatedOperation::new(
            storage_cost,
            processing_cost,
            vec![],
        )));
        ctx
    }

    macro_rules! get_output_amount_from_identity_transition {
        ($transition:ident) => {
            if let AssetLockProof::Instant(lock_proof) = $transition.get_asset_lock_proof() {
                let satoshis = lock_proof
                    .output()
                    .expect("output must be present in instant lock proof")
                    .value;
                satoshis * RATIO
            } else {
                panic!("identity must have an instant lock proof")
            }
        };
    }

    #[tokio::test]
    async fn data_contract_crate_transition_invalid_result_if_balance_is_not_enough() {
        let mut identity = identity_fixture();
        let mut state_repository_mock = MockStateRepositoryLike::new();

        identity.balance = 1;
        state_repository_mock
            .expect_fetch_identity_balance()
            .returning(move |_, _| Ok(Some(identity.get_balance())));

        let created_data_contract = get_data_contract_fixture(None);
        let execution_context = execution_context_with_cost(40, 5);
        let data_contract_create_transition = DataContractCreateTransitionV0 {
            entropy: created_data_contract.entropy_used,
            data_contract: created_data_contract.data_contract,
            ..Default::default()
        };

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate(&data_contract_create_transition.into(), &execution_context)
            .await
            .expect("the validation result should be returned");

        let fee_error = get_fee_error_from_result(&result, 0);
        assert!(matches!(fee_error, FeeError::BalanceIsNotEnoughError(e) if e.balance() == 1));
    }

    #[tokio::test]
    async fn data_contract_crate_transition_should_return_valid_result() {
        let mut identity = identity_fixture();
        let mut state_repository_mock = MockStateRepositoryLike::new();

        identity.balance = 52;
        state_repository_mock
            .expect_fetch_identity_balance()
            .returning(move |_, _| Ok(Some(identity.get_balance())));

        let created_data_contract = get_data_contract_fixture(None);
        let execution_context = execution_context_with_cost(40, 5);
        let data_contract_create_transition = DataContractCreateTransitionV0 {
            entropy: created_data_contract.entropy_used,
            data_contract: created_data_contract.data_contract,
            ..Default::default()
        };

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate(&data_contract_create_transition.into(), &execution_context)
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
            .expect_fetch_identity_balance()
            .returning(move |_, _| Ok(Some(identity.get_balance())));

        let data_contract = get_data_contract_fixture(None).data_contract;
        let documents =
            get_extended_documents_fixture_with_owner_id_from_contract(data_contract.clone())
                .unwrap();
        let transitions = get_document_transitions_fixture([(DocumentTransitionActionType::Create, documents)]);
        let execution_context = execution_context_with_cost(40, 5);
        let documents_batch_transition = DocumentsBatchTransition {
            owner_id: data_contract.owner_id,
            transitions,
            ..Default::default()
        };

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate(&documents_batch_transition.into(), &execution_context)
            .await
            .expect("the validation result should be returned");

        let fee_error = get_fee_error_from_result(&result, 0);
        assert!(matches!(fee_error, FeeError::BalanceIsNotEnoughError(e) if e.balance() == 1));
    }

    #[tokio::test]
    async fn documents_batch_transition_should_return_valid_result() {
        let mut identity = identity_fixture();
        let mut state_repository_mock = MockStateRepositoryLike::new();

        identity.balance = 90;
        state_repository_mock
            .expect_fetch_identity_balance()
            .returning(move |_, _| Ok(Some(identity.get_balance())));

        let data_contract = get_data_contract_fixture(None).data_contract;
        let documents =
            get_extended_documents_fixture_with_owner_id_from_contract(data_contract.clone())
                .unwrap();
        let transitions = get_document_transitions_fixture([(DocumentTransitionActionType::Create, documents)]);
        let execution_context = execution_context_with_cost(40, 5);
        let documents_batch_transition = DocumentsBatchTransition {
            owner_id: data_contract.owner_id,
            transitions,
            ..Default::default()
        };

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate(&documents_batch_transition.into(), &execution_context)
            .await
            .expect("the validation result should be returned");
        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn documents_batch_transition_should_not_increase_balance_on_dry_run() {
        let mut identity = identity_fixture();
        let mut state_repository_mock = MockStateRepositoryLike::new();

        identity.balance = 1;
        state_repository_mock
            .expect_fetch_identity_balance()
            .returning(move |_, _| Ok(Some(identity.get_balance())));

        let data_contract = get_data_contract_fixture(None).data_contract;
        let documents =
            get_extended_documents_fixture_with_owner_id_from_contract(data_contract.clone())
                .unwrap();
        let transitions = get_document_transitions_fixture([(DocumentTransitionActionType::Create, documents)]);
        let execution_context = execution_context_with_cost(40, 5);
        execution_context.enable_dry_run();

        let documents_batch_transition = DocumentsBatchTransition {
            owner_id: data_contract.owner_id,
            transitions,
            ..Default::default()
        };

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate(&documents_batch_transition.into(), &execution_context)
            .await
            .expect("the validation result should be returned");
        assert!(result.is_valid());
    }

    #[tokio::test]
    async fn identity_create_transition_should_return_invalid_result_if_asset_lock_output_amount_is_not_enough(
    ) {
        let identity_create_transition =
            IdentityCreateTransition::from_object(identity_create_transition_fixture(None))
                .unwrap();
        let output_amount = get_output_amount_from_identity_transition!(identity_create_transition);
        let state_repository_mock = MockStateRepositoryLike::new();
        let calculate_state_transition_fee_mock =
            |_: &StateTransition, _: &StateTransitionExecutionContext| {
                Ok(FeeResult {
                    desired_amount: output_amount + 1,
                    ..Default::default()
                })
            };

        let execution_context = StateTransitionExecutionContext::default();

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate_with_custom_calculator(
                &identity_create_transition.into(),
                calculate_state_transition_fee_mock,
                &execution_context,
            )
            .await
            .expect("the validation result should be returned");
        let fee_error = get_fee_error_from_result(&result, 0);

        assert!(
            matches!(fee_error, FeeError::BalanceIsNotEnoughError(e) if e.balance() == output_amount)
        );
    }

    #[tokio::test]
    async fn identity_create_transition_should_return_valid_result() {
        let identity_create_transition =
            IdentityCreateTransition::from_object(identity_create_transition_fixture(None))
                .unwrap();
        let output_amount = get_output_amount_from_identity_transition!(identity_create_transition);
        let state_repository_mock = MockStateRepositoryLike::new();
        let calculate_state_transition_fee_mock =
            |_: &StateTransition, _: &StateTransitionExecutionContext| {
                Ok(FeeResult {
                    desired_amount: output_amount,
                    ..Default::default()
                })
            };
        let execution_context = StateTransitionExecutionContext::default();

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate_with_custom_calculator(
                &identity_create_transition.into(),
                calculate_state_transition_fee_mock,
                &execution_context,
            )
            .await
            .expect("the validation result should be returned");
        assert!(result.is_valid())
    }

    #[tokio::test]
    async fn identity_top_up_transition_should_return_invalid_result_if_balance_is_not_enough() {
        let mut state_repository_mock = MockStateRepositoryLike::new();
        state_repository_mock
            .expect_fetch_identity_balance_with_debt()
            .returning(move |_, _| Ok(Some(1)));

        let identity_topup_transition =
            IdentityTopUpTransition::new(identity_topup_transition_fixture(None)).unwrap();
        let output_amount = get_output_amount_from_identity_transition!(identity_topup_transition);

        let calculate_state_transition_fee_mock =
            |_: &StateTransition, _: &StateTransitionExecutionContext| {
                Ok(FeeResult {
                    desired_amount: output_amount + 2,
                    ..Default::default()
                })
            };

        let execution_context = StateTransitionExecutionContext::default();

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate_with_custom_calculator(
                &identity_topup_transition.into(),
                calculate_state_transition_fee_mock,
                &execution_context,
            )
            .await
            .expect("the validation result should be returned");

        let fee_error = get_fee_error_from_result(&result, 0);

        assert!(
            matches!(fee_error, FeeError::BalanceIsNotEnoughError(e) if e.balance() == output_amount + 1)
        );
    }

    #[tokio::test]
    async fn identity_top_up_transition_should_return_valid_result() {
        let mut state_repository_mock = MockStateRepositoryLike::new();
        state_repository_mock
            .expect_fetch_identity_balance_with_debt()
            .returning(move |_, _| Ok(Some(41)));

        let identity_topup_transition =
            IdentityTopUpTransition::new(identity_topup_transition_fixture(None)).unwrap();
        let output_amount = get_output_amount_from_identity_transition!(identity_topup_transition);

        let calculation_mock = |_: &StateTransition, _: &StateTransitionExecutionContext| {
            Ok(FeeResult {
                desired_amount: output_amount - 1,
                ..Default::default()
            })
        };

        let execution_context = StateTransitionExecutionContext::default();

        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let result = validator
            .validate_with_custom_calculator(
                &identity_topup_transition.into(),
                calculation_mock,
                &execution_context,
            )
            .await
            .expect("the validation result should be returned");

        assert!(result.is_valid())
    }

    #[tokio::test]
    async fn should_return_invalid_state_transition_type() {
        let transition = IdentityCreditWithdrawalTransition::default();
        let state_repository_mock = MockStateRepositoryLike::new();
        let validator = StateTransitionFeeValidator::new(Arc::new(state_repository_mock));
        let execution_context = StateTransitionExecutionContext::default();

        let result = validator
            .validate(&transition.into(), &execution_context)
            .await
            .expect_err("error should be returned");

        match result {
            ProtocolError::InvalidStateTransitionTypeError(err) => {
                assert_eq!(err.transition_type(), 6);
            }
            _ => panic!("expected InvalidStateTransitionTypeError, got {}", result),
        }
    }
}
