use std::convert::TryInto;
use std::sync::Arc;

use anyhow::Result;

use crate::consensus::signature::IdentityNotFoundError;
use crate::contracts::withdrawals_contract;
use crate::document::{generate_document_id, Document};
use crate::identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransitionAction;
use crate::system_data_contracts::load_system_data_contract;
use crate::{
    consensus::basic::identity::IdentityInsufficientBalanceError,
    identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition,
    state_repository::StateRepositoryLike, state_transition::StateTransitionLike,
    validation::ValidationResult, NonConsensusError, StateError,
};

pub struct IdentityCreditWithdrawalTransitionValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: Arc<SR>,
}

impl<SR> IdentityCreditWithdrawalTransitionValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: Arc<SR>) -> Self {
        Self { state_repository }
    }

    pub async fn validate_identity_credit_withdrawal_transition_state(
        &self,
        state_transition: &IdentityCreditWithdrawalTransition,
    ) -> Result<IdentityCreditWithdrawalTransitionAction, ValidationResult<()>> {
        let mut result: ValidationResult<()> = ValidationResult::default();

        // TODO: Use fetchIdentityBalance
        let maybe_existing_identity = self
            .state_repository
            .fetch_identity(
                &state_transition.identity_id,
                state_transition.get_execution_context(),
            )
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
            .map_err(|e| {
                NonConsensusError::StateRepositoryFetchError(format!(
                    "state repository fetch identity for credit withdrawal verification error: {}",
                    e.to_string()
                ))
            })?;

        let existing_identity = maybe_existing_identity
            .ok_or(IdentityNotFoundError::new(state_transition.identity_id).into())?;

        if existing_identity.get_balance() < state_transition.amount {
            let err = IdentityInsufficientBalanceError {
                identity_id: state_transition.identity_id,
                balance: existing_identity.balance,
            };

            result.add_error(err);

            return Err(result);
        }

        // Check revision
        if existing_identity.get_revision() != (state_transition.get_revision() - 1) {
            result.add_error(StateError::InvalidIdentityRevisionError {
                identity_id: existing_identity.get_id().to_owned(),
                current_revision: existing_identity.get_revision(),
            });

            return Err(result);
        }

        document_id = generate_document_id::generate_document_id(
            &withdrawals_contract::CONTRACT_ID,
            &state_transition.identity_id,
            &document_type,
            state_transition.output_script.as_bytes(),
        );

        let withdrawal_document = Document {
            id: Default::default(),
            owner_id: state_transition.identity_id,
            properties: Default::default(),
            revision: None,
            created_at: None,
            updated_at: None,
        };

        Ok(IdentityCreditWithdrawalTransitionAction {
            version: IdentityCreditWithdrawalTransitionAction::current_version(),
            identity_id: state_transition.identity_id,
            prepared_withdrawal_document: withdrawal_document,
        })
    }
}
