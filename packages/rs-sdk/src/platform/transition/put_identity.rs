use crate::platform::transfer::TransferInput;
use crate::platform::transition::broadcast_identity::BroadcastRequestForNewIdentity;
use crate::{Error, Sdk};

use super::address_inputs::fetch_inputs_with_nonce;
use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::waitable::Waitable;
use dpp::address_funds::PlatformAddress;
use dpp::dashcore::PrivateKey;
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::native_bls::NativeBlsModule;
use dpp::prelude::{AddressNonce, AssetLockProof, Identity};
use dpp::state_transition::identity_create_from_addresses_transition::methods::IdentityCreateFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::StateTransition;
use std::collections::BTreeMap;

/// A trait for putting an identity to platform
#[async_trait::async_trait]
pub trait PutIdentity<S: Signer<IdentityPublicKey>>: Waitable {
    /// Sends a new identity to Platform using the provided funding source.
    async fn send_to_platform<F>(
        &self,
        sdk: &Sdk,
        funding: F,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>
    where
        F: TryInto<TransferInput> + Send,
        <F as TryInto<TransferInput>>::Error: ToString;

    /// Sends the identity and waits for confirmation proof.
    async fn send_to_platform_and_wait_for_response<F>(
        &self,
        sdk: &Sdk,
        funding: F,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>
    where
        F: TryInto<TransferInput> + Send,
        <F as TryInto<TransferInput>>::Error: ToString;

    /// Deprecated alias for [`send_to_platform`].
    #[deprecated(note = "use send_to_platform instead")]
    async fn put_to_platform(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        self.send_to_platform(
            sdk,
            (asset_lock_proof, *asset_lock_proof_private_key),
            signer,
            settings,
        )
        .await
    }

    /// Deprecated alias for [`send_to_platform_and_wait_for_response`].
    #[deprecated(note = "use send_to_platform_and_wait_for_response instead")]
    async fn put_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>
    where
        Self: Sized,
    {
        self.send_to_platform_and_wait_for_response(
            sdk,
            (asset_lock_proof, *asset_lock_proof_private_key),
            signer,
            settings,
        )
        .await
    }
}
#[async_trait::async_trait]
impl<S: Signer<IdentityPublicKey>> PutIdentity<S> for Identity {
    async fn send_to_platform<F>(
        &self,
        sdk: &Sdk,
        funding: F,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>
    where
        F: TryInto<TransferInput> + Send,
        <F as TryInto<TransferInput>>::Error: ToString,
    {
        let funding_source = funding
            .try_into()
            .map_err(|e| Error::Generic(e.to_string()))?;
        send_identity_with_source(self, sdk, funding_source, signer, settings).await
    }

    async fn send_to_platform_and_wait_for_response<F>(
        &self,
        sdk: &Sdk,
        funding: F,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Identity, Error>
    where
        F: TryInto<TransferInput> + Send,
        <F as TryInto<TransferInput>>::Error: ToString,
    {
        let funding_source = funding
            .try_into()
            .map_err(|e| Error::Generic(e.to_string()))?;
        let state_transition =
            send_identity_with_source(self, sdk, funding_source, signer, settings).await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}

async fn send_identity_with_source<S: Signer<IdentityPublicKey>>(
    identity: &Identity,
    sdk: &Sdk,
    funding: TransferInput,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    match &funding {
        TransferInput::AssetLock {
            asset_lock_proof,
            asset_lock_private_key,
        } => {
            let (state_transition, _) = identity.broadcast_request_for_new_identity(
                asset_lock_proof.to_owned(),
                &asset_lock_private_key,
                signer,
                sdk.version(),
            )?;
            state_transition.broadcast(sdk, settings).await?;
            Ok(state_transition)
        }
        TransferInput::Addresses {
            inputs,
            input_private_keys,
        } => {
            let inputs_with_nonce = fetch_inputs_with_nonce(sdk, &inputs).await?;
            send_identity_with_addresses(
                identity,
                sdk,
                inputs_with_nonce,
                input_private_keys,
                signer,
                settings,
            )
            .await
        }
        TransferInput::AddressesWithNonce {
            inputs,
            input_private_keys,
        } => {
            send_identity_with_addresses(
                identity,
                sdk,
                inputs.clone(),
                input_private_keys,
                signer,
                settings,
            )
            .await
        }
    }
}

async fn send_identity_with_addresses<S: Signer<IdentityPublicKey>>(
    identity: &Identity,
    sdk: &Sdk,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    input_private_keys: &Vec<Vec<u8>>,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    if input_private_keys.is_empty() {
        return Err(Error::Generic(
            "input_private_keys must contain at least one key".to_string(),
        ));
    }
    let key_refs: Vec<&[u8]> = input_private_keys
        .iter()
        .map(|key| key.as_slice())
        .collect();

    let user_fee_increase = settings
        .as_ref()
        .and_then(|settings| settings.user_fee_increase)
        .unwrap_or_default();

    let state_transition = IdentityCreateFromAddressesTransition::try_from_inputs_with_signer(
        identity,
        inputs,
        key_refs,
        signer,
        &NativeBlsModule,
        user_fee_increase,
        sdk.version(),
    )?;

    state_transition.broadcast(sdk, settings).await?;
    Ok(state_transition)
}
