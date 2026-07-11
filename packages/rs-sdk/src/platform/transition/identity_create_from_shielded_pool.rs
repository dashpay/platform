use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use crate::{Error, Sdk};
use dpp::address_funds::PlatformAddress;
use dpp::shielded::OrchardBundleParams;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::methods::IdentityCreateFromShieldedPoolTransitionMethodsV0;
use dpp::state_transition::state_transitions::shielded::identity_create_from_shielded_pool_transition::IdentityCreateFromShieldedPoolTransition;
use dpp::state_transition::StateTransition;

/// Helper trait to create a brand-new Platform identity funded directly from the shielded pool.
#[async_trait::async_trait]
pub trait IdentityCreateFromShieldedPool {
    /// Build (and structurally validate) the Type-20 transition from PoP-signed keys + bundle
    /// params, **without** broadcasting it.
    ///
    /// Splitting the build off `identity_create_from_shielded_pool` lets a caller stage the
    /// broadcast and the result-wait separately (via [`BroadcastStateTransition::broadcast`] then
    /// [`BroadcastStateTransition::wait_for_response`]). That separation is what lets the wallet
    /// distinguish a *broadcast-time* rejection (notes safe to release) from a *post-broadcast*
    /// confirmation failure where the transition may well have executed on chain (notes must stay
    /// reserved) — a single `broadcast_and_wait` `Result` collapses the two and misattributes the
    /// latter, which is exactly the orphaned-identity hazard this split fixes.
    ///
    /// `public_keys` MUST already carry their per-key proof-of-possession signatures over the
    /// transition's signable bytes (the wallet/builder fills them before broadcast); the bundle's
    /// binding signature commits the derived id + denomination + full key set. No platform identity
    /// signature is involved.
    fn identity_create_from_shielded_pool_transition(
        &self,
        public_keys: Vec<IdentityPublicKeyInCreation>,
        denomination: u64,
        send_to_address_on_creation_failure: PlatformAddress,
        bundle: OrchardBundleParams,
    ) -> Result<StateTransition, Error>;

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
        send_to_address_on_creation_failure: PlatformAddress,
        bundle: OrchardBundleParams,
        settings: Option<PutSettings>,
    ) -> Result<StateTransitionProofResult, Error>;
}

#[async_trait::async_trait]
impl IdentityCreateFromShieldedPool for Sdk {
    fn identity_create_from_shielded_pool_transition(
        &self,
        public_keys: Vec<IdentityPublicKeyInCreation>,
        denomination: u64,
        send_to_address_on_creation_failure: PlatformAddress,
        bundle: OrchardBundleParams,
    ) -> Result<StateTransition, Error> {
        let OrchardBundleParams {
            actions,
            anchor,
            proof,
            binding_signature,
        } = bundle;

        let state_transition = IdentityCreateFromShieldedPoolTransition::try_from_bundle(
            public_keys,
            denomination,
            send_to_address_on_creation_failure,
            actions,
            anchor,
            proof,
            binding_signature,
            self.version(),
        )?;
        ensure_valid_state_transition_structure(&state_transition, self.version())?;

        Ok(state_transition)
    }

    async fn identity_create_from_shielded_pool(
        &self,
        public_keys: Vec<IdentityPublicKeyInCreation>,
        denomination: u64,
        send_to_address_on_creation_failure: PlatformAddress,
        bundle: OrchardBundleParams,
        settings: Option<PutSettings>,
    ) -> Result<StateTransitionProofResult, Error> {
        // Build + structurally validate via the shared builder, then wait for proven inclusion
        // (parity with `unshield`/`shielded_transfer`/`withdraw`), so the wallet's post-broadcast
        // `finalize_pending` only runs once the spend is proven — a Platform-level rejection after
        // relay then correctly triggers the `cancel_pending` fallback. Callers that need to
        // distinguish broadcast-time from post-broadcast failures should instead drive the two
        // `BroadcastStateTransition` stages themselves off the transition returned by
        // `identity_create_from_shielded_pool_transition`.
        let state_transition = self.identity_create_from_shielded_pool_transition(
            public_keys,
            denomination,
            send_to_address_on_creation_failure,
            bundle,
        )?;
        let proof_result = state_transition
            .broadcast_and_wait::<StateTransitionProofResult>(self, settings)
            .await?;
        Ok(proof_result)
    }
}
