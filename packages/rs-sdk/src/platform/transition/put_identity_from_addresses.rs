use std::collections::BTreeMap;

use super::address_inputs::fetch_inputs_with_nonce;
use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::waitable::Waitable;
use crate::{Error, Sdk};
use dpp::address_funds::PlatformAddress;
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::identity::{Identity, IdentityPublicKey};
use dpp::native_bls::NativeBlsModule;
use dpp::prelude::AddressNonce;
use dpp::state_transition::identity_create_from_addresses_transition::methods::IdentityCreateFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::StateTransition;

/// Helper trait to put an identity to Platform using address balances instead of an asset lock.
#[async_trait::async_trait]
pub trait PutIdentityFromAddresses<S: Signer<IdentityPublicKey>>: Waitable {
    /// Build and broadcast identity creation funded by address inputs, automatically fetching address nonces.
    async fn put_from_addresses(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Build and broadcast identity creation with explicitly provided address nonces.
    /// Build and broadcast identity creation with explicitly provided address nonces.
    ///
    /// Inputs are not pre-validated client-side (Drive will perform authoritative checks).
    async fn put_from_addresses_with_nonce(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Broadcast identity creation (nonce lookup) and wait for the proof, returning the created identity.
    async fn put_from_addresses_and_wait(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>
    where
        Self: Sized;

    /// Broadcast identity creation with provided address nonces and wait for the proof.
    async fn put_from_addresses_with_nonce_and_wait(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>
    where
        Self: Sized;
}

#[async_trait::async_trait]
impl<S: Signer<IdentityPublicKey>> PutIdentityFromAddresses<S> for Identity {
    async fn put_from_addresses(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let inputs_with_nonce = fetch_inputs_with_nonce(sdk, &inputs).await?;

        self.put_from_addresses_with_nonce(
            sdk,
            inputs_with_nonce,
            input_private_keys,
            signer,
            settings,
        )
        .await
    }

    async fn put_from_addresses_with_nonce(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        let user_fee_increase = settings
            .as_ref()
            .and_then(|settings| settings.user_fee_increase)
            .unwrap_or_default();

        let key_refs: Vec<&[u8]> = input_private_keys
            .iter()
            .map(|key| key.as_slice())
            .collect();

        let state_transition = IdentityCreateFromAddressesTransition::try_from_inputs_with_signer(
            self,
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

    async fn put_from_addresses_and_wait(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, Credits>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error> {
        let state_transition = self
            .put_from_addresses(sdk, inputs, input_private_keys, signer, settings)
            .await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }

    async fn put_from_addresses_with_nonce_and_wait(
        &self,
        sdk: &Sdk,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        input_private_keys: Vec<Vec<u8>>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error> {
        let state_transition = self
            .put_from_addresses_with_nonce(sdk, inputs, input_private_keys, signer, settings)
            .await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}
