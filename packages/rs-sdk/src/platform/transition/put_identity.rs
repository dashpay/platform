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
use dpp::state_transition::proof_result::StateTransitionProofResult;
use dpp::state_transition::StateTransition;
use drive_proof_verifier::types::AddressInfos;
use std::collections::{BTreeMap, BTreeSet};

/// Trait for creating identities on the platform.
#[async_trait::async_trait]
pub trait PutIdentity<IS: Signer<IdentityPublicKey>>: Waitable {
    /// Creates an identity using an asset lock proof.
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &IS,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Creates an identity using an asset lock and waits for confirmation.
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &IS,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>
    where
        Self: Sized;

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
    ) -> Result<(Identity, AddressInfos), Error>;
}

#[async_trait::async_trait]
impl<IS: Signer<IdentityPublicKey>> PutIdentity<IS> for Identity {
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &IS,
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
        signer: &IS,
        settings: Option<PutSettings>,
    ) -> Result<Identity, Error> {
        let state_transition = self
            .put_to_platform(
                sdk,
                asset_lock_proof,
                asset_lock_proof_private_key,
                signer,
                settings,
            )
            .await?;

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
    ) -> Result<(Identity, AddressInfos), Error> {
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
