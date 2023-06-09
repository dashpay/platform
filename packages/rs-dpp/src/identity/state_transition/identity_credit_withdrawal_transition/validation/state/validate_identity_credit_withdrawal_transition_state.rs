use std::convert::TryInto;
use std::sync::Arc;

use anyhow::Result;
use dashcore::{consensus, BlockHeader};
use platform_value::platform_value;

use crate::consensus::signature::IdentityNotFoundError;

use crate::consensus::state::identity::invalid_identity_revision_error::InvalidIdentityRevisionError;
use crate::consensus::state::identity::IdentityInsufficientBalanceError;
use crate::consensus::state::state_error::StateError;
use crate::contracts::withdrawals_contract;
use crate::document::{generate_document_id, Document};
use crate::identity::state_transition::identity_credit_withdrawal_transition::{
    IdentityCreditWithdrawalTransitionAction, Pooling,
};
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::validation::ConsensusValidationResult;
use crate::{
    identity::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition,
    state_repository::StateRepositoryLike, NonConsensusError, ProtocolError,
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

    pub fn validate_identity_credit_withdrawal_transition_state(
        &self,
        state_transition: &IdentityCreditWithdrawalTransition,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<ConsensusValidationResult<IdentityCreditWithdrawalTransitionAction>, ProtocolError>
    {
        let mut result = ConsensusValidationResult::default();

        // TODO: Use fetchIdentityBalance
        let maybe_existing_identity = self
            .state_repository
            .fetch_identity(&state_transition.identity_id, Some(execution_context))?
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
            result.add_error(StateError::InvalidIdentityRevisionError(
                InvalidIdentityRevisionError::new(
                    existing_identity.get_id().to_owned(),
                    existing_identity.get_revision(),
                ),
            ));

            return Ok(result);
        }

        let last_block_time_ms: u64 = self.state_repository.fetch_latest_platform_block_time()?;

        Ok(
            IdentityCreditWithdrawalTransitionAction::from_identity_credit_withdrawal(
                state_transition,
                last_block_time_ms,
            )
            .into(),
        )
    }
}
