use anyhow::{anyhow, Result};

use crate::{
    prelude::Identity, state_repository::StateRepositoryLike, state_transition::StateTransitionLike,
};

use super::IdentityCreditWithdrawalTransition;

pub struct ApplyIdentityCreditWithdrawalTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: SR,
}

impl<SR> ApplyIdentityCreditWithdrawalTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: SR) -> Self {
        Self { state_repository }
    }

    pub async fn apply_identity_credit_withdrawal_transition(
        &self,
        state_transition: &IdentityCreditWithdrawalTransition,
    ) -> Result<()> {
        let maybe_existing_identity: Option<Identity> = self
            .state_repository
            .fetch_identity(
                &state_transition.identity_id,
                state_transition.get_execution_context(),
            )
            .await?;

        let mut existing_identity =
            maybe_existing_identity.ok_or_else(|| anyhow!("Identity not found"))?;

        existing_identity = existing_identity.reduce_balance(state_transition.amount);

        self.state_repository
            .update_identity(&existing_identity, state_transition.get_execution_context())
            .await
    }
}
