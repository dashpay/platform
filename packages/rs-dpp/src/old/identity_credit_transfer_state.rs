use std::convert::TryInto;

use anyhow::Result;

use crate::consensus::signature::IdentityNotFoundError;

use crate::consensus::state::identity::IdentityInsufficientBalanceError;

use crate::identity::state_transition::identity_credit_transfer_transition::action::IdentityCreditTransferTransitionAction;
use crate::identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use crate::identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransitionAction;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::validation::ConsensusValidationResult;
use crate::{state_repository::StateRepositoryLike, NonConsensusError, ProtocolError};

pub struct IdentityCreditTransferTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

impl<SR> IdentityCreditTransferTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> IdentityCreditTransferTransitionStateValidator<SR>
    where
        SR: StateRepositoryLike,
    {
        IdentityCreditTransferTransitionStateValidator { state_repository }
    }

    pub async fn validate(
        &self,
        state_transition: &IdentityCreditTransferTransition,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<IdentityCreditTransferTransitionAction>, ProtocolError>
    {
        let mut result = ConsensusValidationResult::default();

        let maybe_existing_identity = self
            .state_repository
            .fetch_identity(&state_transition.identity_id, Some(execution_context))
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
            .map_err(|e| {
                NonConsensusError::StateRepositoryFetchError(format!(
                    "state repository fetch identity for transfer verification error: {}",
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

        let maybe_existing_recipient = self
            .state_repository
            .fetch_identity(&state_transition.recipient_id, Some(execution_context))
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)
            .map_err(|e| {
                NonConsensusError::StateRepositoryFetchError(format!(
                    "state repository fetch identity for transfer verification error: {}",
                    e
                ))
            })?;

        if maybe_existing_recipient.is_none() {
            let err = IdentityNotFoundError::new(state_transition.recipient_id);

            result.add_error(err);

            return Ok(result);
        };

        Ok(IdentityCreditTransferTransitionAction {
            version: IdentityCreditWithdrawalTransitionAction::current_version(),
            identity_id: state_transition.identity_id,
            transfer_amount: state_transition.amount,
            recipient_id: state_transition.recipient_id,
        }
        .into())
    }
}
