//! Top up an existing identity's balance from the shielded pool.

use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::platform_value::Identifier;
use dpp::shielded::OrchardBundleParams;
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::methods::IdentityTopUpFromShieldedPoolTransitionMethodsV0;
use dpp::state_transition::identity_top_up_from_shielded_pool_transition::IdentityTopUpFromShieldedPoolTransition;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;

/// Spend shielded notes to top up an existing identity's balance (type 21 sibling of
/// `Unshield` with the identity as the output). The bundle must be an already proven
/// Orchard spend whose binding signature commits to `identity_id` and `top_up_amount`
/// (see `dpp::shielded::builder::build_identity_top_up_from_shielded_pool_transition`).
#[async_trait::async_trait]
pub trait IdentityTopUpFromShieldedPool {
    /// Build and structure-check the transition without broadcasting it.
    fn identity_top_up_from_shielded_pool_transition(
        &self,
        identity_id: Identifier,
        top_up_amount: u64,
        bundle: OrchardBundleParams,
    ) -> Result<StateTransition, Error>;

    /// Build, broadcast, and wait for proven execution. Returns the
    /// `VerifiedIdentityWithShieldedNullifiers` proof result (the credited identity and
    /// the spent nullifiers), so a wallet only marks notes spent once the top-up is
    /// cryptographically proven included.
    async fn identity_top_up_from_shielded_pool(
        &self,
        identity_id: Identifier,
        top_up_amount: u64,
        bundle: OrchardBundleParams,
        settings: Option<PutSettings>,
    ) -> Result<StateTransitionProofResult, Error>;
}

#[async_trait::async_trait]
impl IdentityTopUpFromShieldedPool for Sdk {
    fn identity_top_up_from_shielded_pool_transition(
        &self,
        identity_id: Identifier,
        top_up_amount: u64,
        bundle: OrchardBundleParams,
    ) -> Result<StateTransition, Error> {
        let OrchardBundleParams {
            actions,
            anchor,
            proof,
            binding_signature,
        } = bundle;

        let state_transition = IdentityTopUpFromShieldedPoolTransition::try_from_bundle(
            identity_id,
            actions,
            top_up_amount,
            anchor,
            proof,
            binding_signature,
            self.version(),
        )?;
        ensure_valid_state_transition_structure(&state_transition, self.version())?;

        Ok(state_transition)
    }

    async fn identity_top_up_from_shielded_pool(
        &self,
        identity_id: Identifier,
        top_up_amount: u64,
        bundle: OrchardBundleParams,
        settings: Option<PutSettings>,
    ) -> Result<StateTransitionProofResult, Error> {
        let state_transition =
            self.identity_top_up_from_shielded_pool_transition(identity_id, top_up_amount, bundle)?;
        let proof_result = state_transition
            .broadcast_and_wait_for_affected_state::<StateTransitionProofResult>(self, settings)
            .await?;
        Ok(proof_result)
    }
}
