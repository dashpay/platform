use std::sync::Arc;

use anyhow::{anyhow, Result};
use itertools::Itertools;

use crate::identity::state_transition::asset_lock_proof::AssetLockTransactionOutputFetcher;
use crate::identity::state_transition::identity_create_transition::IdentityCreateTransition;
use crate::identity::{convert_satoshi_to_credits, Identity};
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::StateTransitionLike;
use crate::ProtocolError;

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
    pub async fn apply_identity_create_transition(
        &self,
        state_transition: &IdentityCreateTransition,
    ) -> Result<()> {
        let output = self
            .asset_lock_transaction_output_fetcher
            .fetch(state_transition.get_asset_lock_proof())
            .await?;

        let credits_amount = convert_satoshi_to_credits(output.value);

        let identity = Identity {
            protocol_version: state_transition.get_protocol_version(),
            id: state_transition.get_identity_id().clone(),
            public_keys: state_transition
                .get_public_keys()
                .iter()
                .cloned()
                .map(|mut pk| {
                    pk.set_signature(vec![]);
                    pk
                })
                .collect_vec(),
            balance: credits_amount,
            revision: 0,
            asset_lock_proof: None,
            metadata: None,
        };

        self.state_repository.create_identity(&identity).await?;

        let public_key_hashes = identity
            .get_public_keys()
            .iter()
            .map(|public_key| public_key.hash())
            .collect::<Result<Vec<Vec<u8>>, ProtocolError>>()?;

        self.state_repository
            .store_identity_public_key_hashes(identity.get_id(), public_key_hashes)
            .await?;

        let out_point = state_transition
            .get_asset_lock_proof()
            .out_point()
            .ok_or_else(|| anyhow!("Out point is missing from asset lock proof"))?;

        self.state_repository
            .mark_asset_lock_transaction_out_point_as_used(&out_point)
            .await?;

        Ok(())
    }
}
