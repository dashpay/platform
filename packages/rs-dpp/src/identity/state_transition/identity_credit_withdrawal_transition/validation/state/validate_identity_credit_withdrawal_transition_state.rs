use std::sync::Arc;

use anyhow::Result;

use crate::{
    consensus::basic::{identity::IdentityInsufficientBalanceError, BasicError},
    identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition,
    prelude::Identity,
    state_repository::StateRepositoryLike,
    validation::ValidationResult,
    NonConsensusError,
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

        let maybe_existing_identity: Option<Identity> = self
            .state_repository
            .fetch_identity(&state_transition.identity_id)
            .await
            .map_err(|err| NonConsensusError::StateRepositoryFetchError(err.to_string()))?;

        let existing_identity = match maybe_existing_identity {
            None => {
                let err = BasicError::IdentityNotFoundError {
                    identity_id: state_transition.identity_id.clone(),
                };

                result.add_error(err);

                return Ok(result);
            }
            Some(identity) => identity,
        };

        if existing_identity.get_balance() < state_transition.amount {
            let err = IdentityInsufficientBalanceError {
                identity_id: state_transition.identity_id.clone(),
                balance: existing_identity.balance,
            };

            result.add_error(err);

            return Ok(result);
        }

        Ok(result)
    }
}
