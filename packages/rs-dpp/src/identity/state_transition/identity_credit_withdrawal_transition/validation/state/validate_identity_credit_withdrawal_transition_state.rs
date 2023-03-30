use std::convert::TryInto;
use std::sync::Arc;

use anyhow::Result;

use crate::consensus::signature::IdentityNotFoundError;
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
    ) -> Result<ValidationResult<()>, NonConsensusError> {
        let mut result: ValidationResult<()> = ValidationResult::default();

        // TODO: Use fetchIdentityBalance
        let maybe_existing_identity = self
            .state_repository
            .fetch_identity(
                &state_transition.identity_id,
                Some(state_transition.get_execution_context()),
            )
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
            .map_err(|e| {
                NonConsensusError::StateRepositoryFetchError(format!(
                    "state repository fetch identity for credit withdrawal verification error: {}",
                    e
                ))
            })?;

        let Some(existing_identity) = maybe_existing_identity else {
            let err = IdentityNotFoundError::new(state_transition.identity_id);

            result.add_error(err);

            return Ok(result);
        };

        if existing_identity.get_balance() < state_transition.amount {
            let err = IdentityInsufficientBalanceError {
                identity_id: state_transition.identity_id,
                balance: existing_identity.balance,
            };

            result.add_error(err);

            return Ok(result);
        }

        // Check revision
        if existing_identity.get_revision() != (state_transition.get_revision() - 1) {
            result.add_error(StateError::InvalidIdentityRevisionError {
                identity_id: existing_identity.get_id().to_owned(),
                current_revision: existing_identity.get_revision(),
            });

            return Ok(result);
        }

        Ok(result)
    }
}
