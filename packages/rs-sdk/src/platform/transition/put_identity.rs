use crate::platform::transition::broadcast_identity::BroadcastRequestForNewIdentity;
use crate::platform::transition::{
    address_inputs::{collect_address_infos_from_proof, fetch_inputs_with_nonce, nonce_inc},
    broadcast::BroadcastStateTransition,
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
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive_proof_verifier::types::AddressInfos;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

/// Trait for creating identities on the platform.
#[async_trait::async_trait]
pub trait PutIdentity<S: Signer<IdentityPublicKey>>: Waitable {
    /// Creates an identity using an asset lock proof.
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Creates an identity using an asset lock and waits for confirmation.
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>
    where
        Self: Sized;

    /// Creates an identity funded by Platform addresses (nonces fetched automatically).
    async fn put_with_address_funding<
        WS: Signer<PlatformAddress> + Send + Sync,
        K: Into<WS> + Send + Sync,
    >(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        output: Option<(PlatformAddress, Credits)>,
        identity_signer: &S,
        input_address_signer: K,
        settings: Option<PutSettings>,
    ) -> Result<(Identity, AddressInfos), Error>;

    /// Creates an identity funded by Platform addresses using explicit nonces.
    async fn put_with_address_funding_with_nonce<
        WS: Signer<PlatformAddress> + Send + Sync,
        K: Into<WS> + Send + Sync,
    >(
        &self,
        sdk: &Sdk,
        inputs_with_nonce: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        output: Option<(PlatformAddress, Credits)>,
        identity_signer: &S,
        input_address_signer: K,
        settings: Option<PutSettings>,
    ) -> Result<(Identity, AddressInfos), Error>;
}

#[async_trait::async_trait]
impl<S: Signer<IdentityPublicKey>> PutIdentity<S> for Identity {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        put_identity_with_asset_lock(
            self,
            sdk,
            asset_lock_proof,
            asset_lock_proof_private_key,
            signer,
            settings,
        )
        .await
    }

    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Identity, Error> {
        let state_transition = self
            .put_to_platform(
                sdk,
                asset_lock_proof,
                asset_lock_proof_private_key,
                signer,
                settings.clone(),
            )
            .await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }

    async fn put_with_address_funding<
        WS: Signer<PlatformAddress> + Send + Sync,
        K: Into<WS> + Send + Sync,
    >(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        output: Option<(PlatformAddress, Credits)>,
        identity_signer: &S,
        input_address_signer: K,
        settings: Option<PutSettings>,
    ) -> Result<(Identity, AddressInfos), Error> {
        let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(sdk, &inputs).await?);
        self.put_with_address_funding_with_nonce(
            sdk,
            inputs_with_nonce,
            output,
            identity_signer,
            input_address_signer,
            settings,
        )
        .await
    }

    async fn put_with_address_funding_with_nonce<
        WS: Signer<PlatformAddress> + Send + Sync,
        K: Into<WS> + Send + Sync,
    >(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        output: Option<(PlatformAddress, Credits)>,
        identity_signer: &S,
        input_address_signer: K,
        settings: Option<PutSettings>,
    ) -> Result<(Identity, AddressInfos), Error> {
        let input_signer: WS = input_address_signer.into();
        put_identity_with_address_funding::<S, WS>(
            self,
            sdk,
            inputs,
            output,
            identity_signer,
            &input_signer,
            settings,
        )
        .await
    }
}

async fn put_identity_with_asset_lock<S: Signer<IdentityPublicKey>>(
    identity: &Identity,
    sdk: &Sdk,
    asset_lock_proof: AssetLockProof,
    asset_lock_proof_private_key: &PrivateKey,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    let (state_transition, _) = identity.broadcast_request_for_new_identity(
        asset_lock_proof,
        asset_lock_proof_private_key,
        signer,
        sdk.version(),
    )?;
    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
    state_transition.broadcast(sdk, settings).await?;
    Ok(state_transition)
}

async fn put_identity_with_address_funding<
    S: Signer<IdentityPublicKey>,
    WS: Signer<PlatformAddress>,
>(
    identity: &Identity,
    sdk: &Sdk,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    output: Option<(PlatformAddress, Credits)>,
    identity_signer: &S,
    input_signer: &WS,
    settings: Option<PutSettings>,
) -> Result<(Identity, AddressInfos), Error> {
    let expected_addresses: BTreeSet<PlatformAddress> =
        inputs.keys().copied().collect::<BTreeSet<_>>();

    let fee_strategy: AddressFundsFeeStrategy =
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

    let user_fee_increase = settings
        .as_ref()
        .and_then(|settings| settings.user_fee_increase)
        .unwrap_or_default();

    let state_transition = IdentityCreateFromAddressesTransition::try_from_inputs_with_signer(
        identity,
        inputs,
        output,
        fee_strategy,
        identity_signer,
        input_signer,
        user_fee_increase,
        sdk.version(),
    )?;
    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

    match state_transition
        .broadcast_and_wait::<StateTransitionProofResult>(sdk, settings)
        .await?
    {
        StateTransitionProofResult::VerifiedIdentityFullWithAddressInfos(
            proved_identity,
            address_infos_map,
        ) => {
            let proved_identity_id = proved_identity.id();
            if proved_identity_id != identity.id() {
                return Err(Error::InvalidProvedResponse(format!(
                    "proof returned identity {} but {} was created",
                    proved_identity_id,
                    identity.id()
                )));
            }

            let address_infos =
                collect_address_infos_from_proof(address_infos_map, &expected_addresses)?;

            Ok((proved_identity, address_infos))
        }
        other => Err(Error::InvalidProvedResponse(format!(
            "identity proof was expected but not returned: {:?}",
            other
        ))),
    }
}
