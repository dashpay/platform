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
use simple_signer::SimpleAddressSigner;
use std::collections::{BTreeMap, BTreeSet};

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
    async fn put_with_address_funding(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits), Error>;

    /// Creates an identity funded by Platform addresses using explicit nonces.
    async fn put_with_address_funding_with_nonce(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits), Error>;
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

    async fn put_with_address_funding(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits), Error> {
        let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(sdk, &inputs).await?);
        self.put_with_address_funding_with_nonce(
            sdk,
            inputs_with_nonce,
            input_private_keys,
            signer,
            settings,
        )
        .await
    }

    async fn put_with_address_funding_with_nonce(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<(AddressInfos, Credits), Error> {
        put_identity_with_address_funding(self, sdk, inputs, input_private_keys, signer, settings)
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

async fn put_identity_with_address_funding<S: Signer<IdentityPublicKey>>(
    identity: &Identity,
    sdk: &Sdk,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    input_private_keys: Vec<Vec<u8>>,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<(AddressInfos, Credits), Error> {
    if input_private_keys.is_empty() {
        return Err(Error::InvalidCreditTransfer(
            "input_private_keys must contain at least one key".to_string(),
        ));
    }

    let expected_addresses: BTreeSet<PlatformAddress> =
        inputs.keys().copied().collect::<BTreeSet<_>>();
    let signer_addresses: Vec<PlatformAddress> = expected_addresses.iter().copied().collect();
    let address_signer =
        SimpleAddressSigner::from_addresses_and_keys(&signer_addresses, &input_private_keys)?;

    let fee_strategy: AddressFundsFeeStrategy =
        vec![AddressFundsFeeStrategyStep::DeductFromInput(0)];

    let user_fee_increase = settings
        .as_ref()
        .and_then(|settings| settings.user_fee_increase)
        .unwrap_or_default();

    let state_transition = IdentityCreateFromAddressesTransition::try_from_inputs_with_signer(
        identity,
        inputs,
        fee_strategy,
        signer,
        &address_signer,
        user_fee_increase,
        sdk.version(),
    )?;
    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

    match state_transition
        .broadcast_and_wait::<StateTransitionProofResult>(sdk, settings)
        .await?
    {
        StateTransitionProofResult::VerifiedIdentityWithAddressInfos(
            partial_identity,
            address_infos_map,
        ) => {
            if partial_identity.id != identity.id() {
                return Err(Error::InvalidProvedResponse(format!(
                    "proof returned identity {} but {} was created",
                    partial_identity.id,
                    identity.id()
                )));
            }

            let address_infos =
                collect_address_infos_from_proof(address_infos_map, &expected_addresses)?;

            let balance = partial_identity.balance.ok_or_else(|| {
                Error::InvalidProvedResponse(
                    "identity proof did not include updated balance".to_string(),
                )
            })?;

            Ok((address_infos, balance))
        }
        other => Err(Error::InvalidProvedResponse(format!(
            "identity proof was expected but not returned: {:?}",
            other
        ))),
    }
}
