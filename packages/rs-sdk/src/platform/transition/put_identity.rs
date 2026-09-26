use crate::platform::transition::address_inputs::{fetch_inputs_with_nonce, nonce_inc};
use crate::platform::transition::broadcast_identity::BroadcastRequestForNewIdentity;
use crate::platform::transition::{
    address_inputs::collect_address_infos_from_proof, broadcast::BroadcastStateTransition,
};
use crate::{Error, Sdk};

use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::dashcore::PrivateKey;
use dpp::fee::Credits;
use dpp::identity::accessors::IdentityGettersV0;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::{AddressNonce, AssetLockProof, Identity};
use dpp::state_transition::identity_create_from_addresses_transition::methods::IdentityCreateFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::identity_id_from_input_addresses;
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive_proof_verifier::types::AddressInfos;
use std::collections::{BTreeMap, BTreeSet};
use tracing::debug;

/// Trait for creating identities on the platform.
#[async_trait::async_trait]
pub trait PutIdentity<IS: Signer<IdentityPublicKey>>: Waitable {
    /// Creates an identity using an asset lock proof whose private key
    /// is held in-process.
    ///
    /// Prefer [`Self::put_to_platform_with_signer`] when the asset-lock
    /// private key lives outside Rust (Swift / hardware wallet / HSM):
    /// the `_with_signer` variant routes asset-lock signing through an
    /// external [`key_wallet::signer::Signer`] so the private key never
    /// crosses the FFI boundary as raw bytes.
    async fn put_to_platform_with_private_key(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &IS,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Creates an identity using an asset lock and waits for confirmation.
    ///
    /// A broadcast answered with `AlreadyExists` (this exact transition is
    /// already in the mempool or on chain) is not an error: the result wait
    /// that follows resolves it.
    ///
    /// In-process private-key counterpart to
    /// [`Self::put_to_platform_and_wait_for_response_with_signer`].
    async fn put_to_platform_and_wait_for_response_with_private_key(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &IS,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>
    where
        Self: Sized;

    /// Creates an identity using an asset lock proof whose private key
    /// is held by an external [`key_wallet::signer::Signer`] (Swift,
    /// hardware wallet, HSM).
    ///
    /// `identity_signer` signs the per-key witnesses on `public_keys[]`
    /// (same as [`Self::put_to_platform_with_private_key`]), while
    /// `asset_lock_signer` produces the outer state-transition ECDSA
    /// signature for the key at `asset_lock_proof_path` — atomically
    /// deriving, signing, and zeroising inside the signer's trust
    /// boundary.
    #[cfg(feature = "core_key_wallet")]
    async fn put_to_platform_with_signer<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        identity_signer: &IS,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync;

    /// Creates an identity using an asset-lock signer and waits for
    /// confirmation. `AlreadyExists` from the broadcast is resolved by the
    /// wait, as in the private-key variant.
    ///
    /// Signer-driven counterpart to
    /// [`Self::put_to_platform_and_wait_for_response_with_private_key`].
    #[cfg(feature = "core_key_wallet")]
    async fn put_to_platform_and_wait_for_response_with_signer<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        identity_signer: &IS,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>
    where
        Self: Sized,
        AS: dpp::key_wallet::signer::Signer + Send + Sync;

    /// Creates an identity funded by Platform addresses using explicit nonces.
    ///
    /// Use [Identity::new_with_input_addresses_and_keys](dpp::identity::Identity::new_with_input_addresses_and_keys)
    /// to create an identity. Then use this method to put it to the platform.
    ///
    /// This is a preferred method, as you need to use the same nonces when creating the identity.
    async fn put_with_address_funding<AS: Signer<PlatformAddress> + Send + Sync>(
        &self,
        sdk: &Sdk,
        inputs_with_nonce: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        output: Option<(PlatformAddress, Credits)>,
        identity_signer: &IS,
        input_address_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(Identity, AddressInfos, u64), Error>;

    /// Creates an identity funded by Platform addresses, fetching the
    /// current address nonces from Platform automatically.
    ///
    /// Mirrors the auto-fetching pattern in `withdraw_address_funds` /
    /// `transfer_address_funds` — the caller supplies only
    /// `(address, credits)` pairs and we look up each address's
    /// on-chain nonce via `AddressInfo::fetch_many`, increment by 1,
    /// then hand off to [`Self::put_with_address_funding`].
    ///
    /// Prefer this variant when the caller doesn't already have a
    /// trusted nonce source; reaching for
    /// [`Self::put_with_address_funding`] directly otherwise lets you
    /// submit with cached / externally-supplied nonces in one round
    /// trip.
    async fn put_with_address_funding_fetching_nonces<AS: Signer<PlatformAddress> + Send + Sync>(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        output: Option<(PlatformAddress, Credits)>,
        identity_signer: &IS,
        input_address_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(Identity, AddressInfos, u64), Error>;
}

#[async_trait::async_trait]
impl<IS: Signer<IdentityPublicKey>> PutIdentity<IS> for Identity {
    async fn put_to_platform_with_private_key(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &IS,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let state_transition = identity_create_with_asset_lock_and_private_key(
            self,
            sdk,
            asset_lock_proof,
            asset_lock_proof_private_key,
            signer,
            settings,
        )
        .await?;
        state_transition.broadcast(sdk, settings).await?;
        Ok(state_transition)
    }

    async fn put_to_platform_and_wait_for_response_with_private_key(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &IS,
        settings: Option<PutSettings>,
    ) -> Result<Identity, Error> {
        let state_transition = identity_create_with_asset_lock_and_private_key(
            self,
            sdk,
            asset_lock_proof,
            asset_lock_proof_private_key,
            signer,
            settings,
        )
        .await?;
        already_known_is_delivered(state_transition.broadcast(sdk, settings).await)?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }

    #[cfg(feature = "core_key_wallet")]
    async fn put_to_platform_with_signer<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        identity_signer: &IS,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync,
    {
        let state_transition = identity_create_with_asset_lock_and_signer(
            self,
            sdk,
            asset_lock_proof,
            asset_lock_proof_path,
            asset_lock_signer,
            identity_signer,
            settings,
        )
        .await?;
        state_transition.broadcast(sdk, settings).await?;
        Ok(state_transition)
    }

    #[cfg(feature = "core_key_wallet")]
    async fn put_to_platform_and_wait_for_response_with_signer<AS>(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        identity_signer: &IS,
        settings: Option<PutSettings>,
    ) -> Result<Identity, Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync,
    {
        let state_transition = identity_create_with_asset_lock_and_signer(
            self,
            sdk,
            asset_lock_proof,
            asset_lock_proof_path,
            asset_lock_signer,
            identity_signer,
            settings,
        )
        .await?;
        already_known_is_delivered(state_transition.broadcast(sdk, settings).await)?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }

    async fn put_with_address_funding<AS: Signer<PlatformAddress> + Send + Sync>(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        output: Option<(PlatformAddress, Credits)>,
        identity_signer: &IS,
        input_address_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(Identity, AddressInfos, u64), Error> {
        put_identity_with_address_funding::<IS, AS>(
            self,
            sdk,
            inputs,
            output,
            identity_signer,
            input_address_signer,
            settings,
        )
        .await
    }

    async fn put_with_address_funding_fetching_nonces<AS: Signer<PlatformAddress> + Send + Sync>(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        output: Option<(PlatformAddress, Credits)>,
        identity_signer: &IS,
        input_address_signer: &AS,
        settings: Option<PutSettings>,
    ) -> Result<(Identity, AddressInfos, u64), Error> {
        // Platform's convention: transitions submit `last_used + 1`.
        // `fetch_inputs_with_nonce` reads the on-chain "last used",
        // `nonce_inc` bumps by 1 — same helpers used by
        // `withdraw_address_funds` / `transfer_address_funds`.
        let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(sdk, &inputs).await?);
        self.put_with_address_funding(
            sdk,
            inputs_with_nonce,
            output,
            identity_signer,
            input_address_signer,
            settings,
        )
        .await
    }
}

/// A broadcast whose result is waited for next: `AlreadyExists` counts as
/// delivered. DAPI returns it only when this exact transition is already in
/// the mempool or on chain (a broadcast that timed out after the node took it,
/// retried on another node), so the wait resolves it; failing here would
/// report an identity create that lands as a failure.
fn already_known_is_delivered(broadcast: Result<(), Error>) -> Result<(), Error> {
    match broadcast {
        Err(Error::AlreadyExists(message)) => {
            debug!(%message, "identity create is already known to Platform; waiting for its result");
            Ok(())
        }
        other => other,
    }
}

/// Build and structurally validate the identity create, without broadcasting.
async fn identity_create_with_asset_lock_and_private_key<S: Signer<IdentityPublicKey>>(
    identity: &Identity,
    sdk: &Sdk,
    asset_lock_proof: AssetLockProof,
    asset_lock_proof_private_key: &PrivateKey,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    // `broadcast_request_for_new_identity_with_private_key` reads
    // `PutSettings::user_fee_increase` internally; threading
    // `settings` straight through honours that knob without the
    // pre-extraction dance. The CL-height retry path in
    // `platform-wallet` relies on `user_fee_increase` to change the
    // ST's signable bytes (Tenderdash's 24h invalid-tx hash cache
    // would silently drop identical-bytes resubmits).
    let (state_transition, _) = identity
        .broadcast_request_for_new_identity_with_private_key(
            asset_lock_proof,
            asset_lock_proof_private_key,
            signer,
            sdk.version(),
            settings,
        )
        .await?;
    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
    Ok(state_transition)
}

/// Signer-driven counterpart to
/// [`identity_create_with_asset_lock_and_private_key`].
#[cfg(feature = "core_key_wallet")]
#[allow(clippy::too_many_arguments)]
async fn identity_create_with_asset_lock_and_signer<IS, AS>(
    identity: &Identity,
    sdk: &Sdk,
    asset_lock_proof: AssetLockProof,
    asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
    asset_lock_signer: &AS,
    identity_signer: &IS,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error>
where
    IS: Signer<IdentityPublicKey>,
    AS: dpp::key_wallet::signer::Signer + Send + Sync,
{
    // `broadcast_request_for_new_identity_with_signer` reads
    // `PutSettings::user_fee_increase` internally; thread `settings`
    // through to honour the CL-height retry's hash-bumping mechanism
    // (see the matching block in `identity_create_with_asset_lock_and_private_key`).
    let (state_transition, _) = identity
        .broadcast_request_for_new_identity_with_signer(
            asset_lock_proof,
            asset_lock_proof_path,
            asset_lock_signer,
            identity_signer,
            sdk.version(),
            settings,
        )
        .await?;
    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
    Ok(state_transition)
}

async fn put_identity_with_address_funding<
    IS: Signer<IdentityPublicKey>,
    AS: Signer<PlatformAddress>,
>(
    identity: &Identity,
    sdk: &Sdk,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    output: Option<(PlatformAddress, Credits)>,
    identity_signer: &IS,
    input_signer: &AS,
    settings: Option<PutSettings>,
) -> Result<(Identity, AddressInfos, u64), Error> {
    let expected_addresses: BTreeSet<PlatformAddress> =
        inputs.keys().copied().collect::<BTreeSet<_>>();

    let fee_strategy: AddressFundsFeeStrategy =
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

    let user_fee_increase = settings
        .as_ref()
        .and_then(|settings| settings.user_fee_increase)
        .unwrap_or_default();

    // Compute the expected identity ID deterministically from the input addresses
    // and nonces BEFORE they're moved into try_from_inputs_with_signer. This must
    // NOT use identity.id(), which may be a caller-supplied placeholder that doesn't
    // match the platform-computed ID. See https://github.com/dashpay/platform/issues/3095
    let expected_identity_id = identity_id_from_input_addresses(&inputs)?;

    let state_transition = IdentityCreateFromAddressesTransition::try_from_inputs_with_signer(
        identity,
        inputs,
        output,
        fee_strategy,
        identity_signer,
        input_signer,
        user_fee_increase,
        sdk.version(),
    )
    .await?;

    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

    // `metadata.height` is the proof's committed block — the height
    // pin for these absolutes (`AddressFunds::as_of_height`).
    let (st_result, metadata) = state_transition
        .broadcast_and_wait_with_metadata::<StateTransitionProofResult>(sdk, settings)
        .await?;
    match st_result {
        StateTransitionProofResult::VerifiedIdentityFullWithAddressInfos(
            proved_identity,
            address_infos_map,
        ) => {
            let proved_identity_id = proved_identity.id();
            if proved_identity_id != expected_identity_id {
                return Err(Error::InvalidProvedResponse(format!(
                    "proof returned identity {} but {} was expected (derived from input addresses)",
                    proved_identity_id, expected_identity_id
                )));
            }

            let address_infos =
                collect_address_infos_from_proof(address_infos_map, &expected_addresses)?;

            Ok((proved_identity, address_infos, metadata.height))
        }
        other => Err(Error::InvalidProvedResponse(format!(
            "identity proof was expected but not returned: {:?}",
            other
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_treat_an_already_known_broadcast_as_delivered() {
        let broadcast = Err(Error::AlreadyExists(
            "state transition already in mempool".to_string(),
        ));
        assert!(already_known_is_delivered(broadcast).is_ok());
    }

    #[test]
    fn should_keep_every_other_broadcast_outcome() {
        assert!(already_known_is_delivered(Ok(())).is_ok());
        let rejected = already_known_is_delivered(Err(Error::Generic("rejected".to_string())));
        assert!(matches!(rejected, Err(Error::Generic(_))));
    }
}
