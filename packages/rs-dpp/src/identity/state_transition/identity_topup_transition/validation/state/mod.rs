use crate::identity::state_transition::identity_topup_transition::{
    IdentityTopUpTransition, IdentityTopUpTransitionAction,
};
use crate::state_repository::StateRepositoryLike;

use crate::validation::{AsyncDataValidator, ValidationResult};
use crate::{NonConsensusError, ProtocolError};
use async_trait::async_trait;

pub struct IdentityTopUpTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

#[async_trait(?Send)]
impl<SR> AsyncDataValidator for IdentityTopUpTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    type Item = IdentityTopUpTransition;
    type ResultItem = IdentityTopUpTransitionAction;

    async fn validate(
        &self,
        data: &IdentityTopUpTransition,
    ) -> Result<ValidationResult<IdentityTopUpTransitionAction>, ProtocolError> {
        validate_identity_topup_transition_state(&self.state_repository, data)
            .await
            .map_err(|err| err.into())
    }
}

impl<SR> IdentityTopUpTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> IdentityTopUpTransitionStateValidator<SR>
    where
        SR: StateRepositoryLike,
    {
        IdentityTopUpTransitionStateValidator { state_repository }
    }
}

/// Validate that identity exists
///
/// Do we need to check that key ids are incremental?
///
/// For later versions:
/// 1. We need to check that outpoint exists (not now)
/// 2. Verify ownership proof signature, as it requires special transaction to be implemented
pub async fn validate_identity_topup_transition_state(
    state_repository: &impl StateRepositoryLike,
    state_transition: &IdentityTopUpTransition,
) -> Result<ValidationResult<IdentityTopUpTransitionAction>, NonConsensusError> {
    //todo: I think we need to validate that the identity actually exists
    let top_up_balance_amount = state_transition
        .asset_lock_proof
        .fetch_asset_lock_transaction_output(state_repository, &state_transition.execution_context)
        .await
        .map_err(Into::<NonConsensusError>::into)?;
    Ok(
        IdentityTopUpTransitionAction::from_borrowed(state_transition, top_up_balance_amount.value)
            .into(),
    )
}
