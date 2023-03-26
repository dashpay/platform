use crate::identity::state_transition::identity_topup_transition::{
    IdentityTopUpTransition, IdentityTopUpTransitionAction,
};
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::validation::{
    AsyncStateTransitionDataValidator, SimpleValidationResult, ValidationResult,
};
use crate::{NonConsensusError, ProtocolError};
use async_trait::async_trait;

pub struct IdentityTopUpTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

#[async_trait(?Send)]
impl<SR> AsyncStateTransitionDataValidator for IdentityTopUpTransitionStateValidator<SR>
where
    SR: StateRepositoryLike,
{
    type StateTransition = IdentityTopUpTransition;
    type StateTransitionAction = IdentityTopUpTransitionAction;

    async fn validate(
        &self,
        data: &IdentityTopUpTransition,
    ) -> Result<IdentityTopUpTransitionAction, SimpleValidationResult> {
        validate_identity_topup_transition_state(data, &self.state_repository)
            .await
            .map(|result| result.into())
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
    state_transition: &IdentityTopUpTransition,
    state_repository: &impl StateRepositoryLike,
    execution_context: &StateTransitionExecutionContext,
) -> Result<IdentityTopUpTransitionAction, ValidationResult<()>> {
    //todo: I think we need to validate that the identity actually exists
    let top_up_balance_amount = state_transition
        .asset_lock_proof
        .fetch_asset_lock_transaction_output(state_repository, execution_context)
        .await?;
    Ok(IdentityTopUpTransitionAction::from_borrowed(
        state_transition,
        top_up_balance_amount.value,
    ))
}
