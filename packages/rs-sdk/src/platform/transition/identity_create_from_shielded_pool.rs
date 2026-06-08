use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::shielded::OrchardBundleParams;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::methods::IdentityCreateFromShieldedPoolTransitionMethodsV0;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;

/// Helper trait to create a brand-new Platform identity funded directly from the shielded pool.
#[async_trait::async_trait]
pub trait IdentityCreateFromShieldedPool {
    /// Create a new identity funded by spending shielded-pool notes.
    ///
    /// The exit amount is a fixed `denomination` (a member of the versioned denomination set), and
    /// authorization is 100% the Orchard proof + per-action spend-auth signatures + binding
    /// signature (no platform identity signature). The new identity's id is derived from the spend
    /// nullifiers and is bound — together with the denomination and the full public-key set — into
    /// the Orchard sighash, so the bundle cannot be redirected.
    ///
    /// `public_keys` MUST already carry their per-key proof-of-possession signatures over the
    /// transition's signable bytes (the wallet/builder fills them before broadcast). The new
    /// identity is created holding `denomination - total_fee`.
    ///
    /// Like the other shielded spends, this **waits for proven execution** (not just relay-ACK) and
    /// returns the `StateTransitionProofResult` (a `VerifiedIdentityWithShieldedNullifiers`), so a
    /// caller's post-broadcast bookkeeping (e.g. the wallet marking notes spent) only runs after the
    /// transition is cryptographically proven included.
    async fn identity_create_from_shielded_pool(
        &self,
        public_keys: Vec<IdentityPublicKeyInCreation>,
        denomination: u64,
        bundle: OrchardBundleParams,
        settings: Option<PutSettings>,
    ) -> Result<StateTransitionProofResult, Error>;
}

#[async_trait::async_trait]
impl IdentityCreateFromShieldedPool for Sdk {
    async fn identity_create_from_shielded_pool(
        &self,
        public_keys: Vec<IdentityPublicKeyInCreation>,
        denomination: u64,
        bundle: OrchardBundleParams,
        settings: Option<PutSettings>,
    ) -> Result<StateTransitionProofResult, Error> {
        let OrchardBundleParams {
            actions,
            anchor,
            proof,
            binding_signature,
        } = bundle;

        let state_transition = IdentityCreateFromShieldedPoolTransition::try_from_bundle(
            public_keys,
            denomination,
            actions,
            anchor,
            proof,
            binding_signature,
            self.version(),
        )?;
        ensure_valid_state_transition_structure(&state_transition, self.version())?;

        // Wait for proven inclusion (parity with `unshield`/`shielded_transfer`/`withdraw`), so the
        // wallet's post-broadcast `finalize_pending` only runs once the spend is proven — a
        // Platform-level rejection after relay then correctly triggers the `cancel_pending` fallback.
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, settings)
            .await?;
        Ok(proof_result)
    }
}
