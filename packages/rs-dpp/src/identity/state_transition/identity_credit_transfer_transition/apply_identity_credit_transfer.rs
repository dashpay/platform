use crate::identity::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use std::sync::Arc;

use anyhow::Result;

#[derive(Clone)]
pub struct ApplyIdentityCreditTransferTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: Arc<SR>,
}

impl<SR> ApplyIdentityCreditTransferTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: Arc<SR>) -> Self {
        Self { state_repository }
    }

    pub async fn apply(
        &self,
        state_transition: &IdentityCreditTransferTransition,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        let identity_id = state_transition.get_identity_id();
        let recipient_id = state_transition.get_recipient_id();

        self.state_repository
            .add_to_identity_balance(
                recipient_id,
                state_transition.amount,
                Some(execution_context),
            )
            .await?;

        self.state_repository
            .remove_from_identity_balance(
                identity_id,
                state_transition.amount,
                Some(execution_context),
            )
            .await?;

        Ok(())
    }
}
