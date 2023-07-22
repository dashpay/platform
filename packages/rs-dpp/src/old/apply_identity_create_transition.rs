use std::sync::Arc;

use anyhow::{anyhow, Result};

use crate::identity::state_transition::asset_lock_proof::AssetLockTransactionOutputFetcher;
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::identity::{convert_duffs_to_credits, Identity};
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::state_transition_execution_context::StateTransitionExecutionContext;
use crate::state_transition::StateTransitionLike;

#[derive(Clone)]
pub struct ApplyIdentityCreateTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: Arc<SR>,
    asset_lock_transaction_output_fetcher: Arc<AssetLockTransactionOutputFetcher<SR>>,
}

impl<SR> ApplyIdentityCreateTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(state_repository: Arc<SR>) -> Self {
        let asset_lock_transaction_output_fetcher = Arc::new(
            AssetLockTransactionOutputFetcher::new(state_repository.clone()),
        );

        Self {
            state_repository,
            asset_lock_transaction_output_fetcher,
        }
    }

    pub async fn apply_identity_create_transition(
        &self,
        state_transition: &IdentityCreateTransition,
        execution_context: &StateTransitionExecutionContext,
    ) -> Result<()> {
        let output = self
            .asset_lock_transaction_output_fetcher
            .fetch(state_transition.get_asset_lock_proof(), execution_context)
            .await?;

        let credits_amount = convert_duffs_to_credits(output.value)?;

        let identity = Identity {
            feature_version: state_transition.state_transition_protocol_version(),
            id: *state_transition.get_identity_id(),
            public_keys: state_transition
                .public_keys()
                .iter()
                .cloned()
                .map(|pk| (pk.id, pk.to_identity_public_key()))
                .collect(),
            balance: credits_amount,
            revision: 0,
            asset_lock_proof: None,
            metadata: None,
        };

        self.state_repository
            .create_identity(&identity, Some(execution_context))
            .await?;

        self.state_repository
            .add_to_system_credits(credits_amount, Some(execution_context))
            .await?;

        let out_point = state_transition
            .get_asset_lock_proof()
            .out_point()
            .ok_or_else(|| anyhow!("Out point is missing from asset lock proof"))?;

        self.state_repository
            .mark_asset_lock_transaction_out_point_as_used(&out_point, Some(execution_context))
            .await?;

        Ok(())
    }
}
