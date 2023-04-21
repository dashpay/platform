use anyhow::anyhow;
use std::sync::Arc;

use crate::identity::IdentityPublicKey;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::{state_repository::StateRepositoryLike, ProtocolError};

use super::identity_update_transition::IdentityUpdateTransition;

#[derive(Clone)]
pub struct ApplyIdentityUpdateTransition<SR> {
    state_repository: Arc<SR>,
}

impl<SR> ApplyIdentityUpdateTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: Arc<SR>) -> Self {
        Self { state_repository }
    }

    pub async fn apply(
        &self,
        state_transition: &IdentityUpdateTransition,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<(), ProtocolError> {
        apply_identity_update_transition(
            self.state_repository.as_ref(),
            state_transition,
            execution_context,
        )
        .await
    }
}

/// Apply Identity Update state transition
pub async fn apply_identity_update_transition(
    state_repository: &impl StateRepositoryLike,
    state_transition: &IdentityUpdateTransition,
    execution_context: &StateTransitionExecutionContext,
) -> Result<(), ProtocolError> {
    state_repository
        .update_identity_revision(
            &state_transition.identity_id,
            state_transition.revision,
            Some(execution_context),
        )
        .await?;

    if !state_transition.get_public_key_ids_to_disable().is_empty() {
        let disabled_at = state_transition
            .get_public_keys_disabled_at()
            .ok_or_else(|| anyhow!("disabled_at must be present"))?;

        state_repository
            .disable_identity_keys(
                &state_transition.identity_id,
                state_transition.get_public_key_ids_to_disable(),
                disabled_at,
                Some(execution_context),
            )
            .await?;
    }

    if !state_transition.get_public_keys_to_add().is_empty() {
        let keys_to_add = state_transition
            .get_public_keys_to_add()
            .iter()
            .cloned()
            .map(|pk| pk.to_identity_public_key())
            .collect::<Vec<IdentityPublicKey>>();

        state_repository
            .add_keys_to_identity(
                &state_transition.identity_id,
                &keys_to_add,
                Some(execution_context),
            )
            .await?;
    }

    Ok(())
}
