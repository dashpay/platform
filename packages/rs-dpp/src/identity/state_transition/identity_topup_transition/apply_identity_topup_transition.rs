use std::convert::TryInto;
use std::sync::Arc;

use anyhow::{anyhow, Result};

use crate::identity::state_transition::asset_lock_proof::AssetLockTransactionOutputFetcher;
use crate::identity::state_transition::identity_topup_transition::IdentityTopUpTransition;
use crate::identity::{convert_satoshi_to_credits, get_biggest_possible_identity};
use crate::state_repository::StateRepositoryLike;
use crate::state_transition::StateTransitionLike;

pub struct ApplyIdentityTopUpTransition<SR>
where
    SR: StateRepositoryLike,
{
    state_repository: Arc<SR>,
    asset_lock_transaction_output_fetcher: Arc<AssetLockTransactionOutputFetcher<SR>>,
}

impl<SR> ApplyIdentityTopUpTransition<SR>
where
    SR: StateRepositoryLike,
{
    pub fn new(
        state_repository: Arc<SR>,
        asset_lock_transaction_output_fetcher: Arc<AssetLockTransactionOutputFetcher<SR>>,
    ) -> Self {
        Self {
            state_repository,
            asset_lock_transaction_output_fetcher,
        }
    }

    pub async fn apply(&self, state_transition: &IdentityTopUpTransition) -> Result<()> {
        let is_dry_run = state_transition.get_execution_context().is_dry_run();
        let output = self
            .asset_lock_transaction_output_fetcher
            .fetch(
                state_transition.get_asset_lock_proof(),
                state_transition.get_execution_context(),
            )
            .await?;

        let credits_amount = convert_satoshi_to_credits(output.value);

        let out_point = state_transition
            .get_asset_lock_proof()
            .out_point()
            .ok_or_else(|| anyhow!("Out point is missing from asset lock proof"))?;
        let identity_id = state_transition.get_identity_id();
        let mut maybe_identity = self
            .state_repository
            .fetch_identity(identity_id, state_transition.get_execution_context())
            .await?
            .map(TryInto::try_into)
            .transpose()
            .map_err(Into::into)?;

        if is_dry_run {
            maybe_identity = Some(get_biggest_possible_identity())
        }

        if let Some(mut identity) = maybe_identity {
            if !is_dry_run {
                identity.increase_balance(credits_amount);
            };

            self.state_repository
                .update_identity(&identity, state_transition.get_execution_context())
                .await?;

            self.state_repository
                .mark_asset_lock_transaction_out_point_as_used(&out_point)
                .await?;

            Ok(())
        } else {
            Err(anyhow!("Identity not found"))
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::{
        identity::state_transition::{
            asset_lock_proof::AssetLockTransactionOutputFetcher,
            identity_topup_transition::IdentityTopUpTransition,
        },
        state_repository::MockStateRepositoryLike,
        state_transition::StateTransitionLike,
        tests::fixtures::identity_topup_transition_fixture_json,
    };

    use super::ApplyIdentityTopUpTransition;

    #[tokio::test]
    async fn should_store_biggest_possible_identity_if_on_dry_run() {
        let raw_transition = identity_topup_transition_fixture_json(None);
        let transition = IdentityTopUpTransition::new(raw_transition).unwrap();

        let mut state_repository_for_apply = MockStateRepositoryLike::new();
        let state_repository_for_fetcher = MockStateRepositoryLike::new();

        state_repository_for_apply
            .expect_fetch_identity()
            .return_once(|_, _| Ok(None));
        state_repository_for_apply
            .expect_update_identity()
            .return_once(|_, _| Ok(()));
        state_repository_for_apply
            .expect_mark_asset_lock_transaction_out_point_as_used()
            .return_once(|_| Ok(()));

        let asset_lock_transaction_fetcher =
            AssetLockTransactionOutputFetcher::new(Arc::new(state_repository_for_fetcher));
        let apply_identity_topup_transition = ApplyIdentityTopUpTransition::new(
            Arc::new(state_repository_for_apply),
            Arc::new(asset_lock_transaction_fetcher),
        );

        transition.get_execution_context().enable_dry_run();

        let result = apply_identity_topup_transition.apply(&transition).await;
        assert!(result.is_ok())
    }
}
