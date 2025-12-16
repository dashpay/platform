use crate::platform::transition::address_inputs::nonce_inc;
use crate::platform::transition::broadcast_identity::BroadcastRequestForNewIdentity;
use crate::{Error, Sdk};

use super::address_inputs::fetch_inputs_with_nonce;
use super::broadcast::BroadcastStateTransition;
use super::put_settings::PutSettings;
use super::validation::ensure_valid_state_transition_structure;
use super::waitable::Waitable;
use dpp::address_funds::{AddressFundsFeeStrategy, AddressFundsFeeStrategyStep, PlatformAddress};
use dpp::dashcore::PrivateKey;
use dpp::fee::Credits;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::prelude::{AddressNonce, AssetLockProof, Identity};
use dpp::state_transition::identity_create_from_addresses_transition::methods::IdentityCreateFromAddressesTransitionMethodsV0;
use dpp::state_transition::identity_create_from_addresses_transition::IdentityCreateFromAddressesTransition;
use dpp::state_transition::StateTransition;
use simple_signer::SimpleAddressSigner;
use std::collections::BTreeMap;

/// Funding sources supported when creating an identity.
pub enum IdentityFunding<S: Signer<PlatformAddress>> {
    AssetLock {
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
    },
    AddressesWithPrivateKeys {
        inputs: BTreeMap<PlatformAddress, Credits>,
        change_output: Option<(PlatformAddress, Credits)>,
        input_private_keys: Vec<[u8; 32]>,
    },
    AddressesWithPrivateKeysAndNonce {
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        change_output: Option<(PlatformAddress, Credits)>,
        input_private_keys: Vec<[u8; 32]>,
    },
    AddressesWithSigner {
        inputs: BTreeMap<PlatformAddress, Credits>,
        change_output: Option<(PlatformAddress, Credits)>,
        address_signer: S,
    },
    AddressesWithSignerAndNonce {
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        change_output: Option<(PlatformAddress, Credits)>,
        address_signer: S,
    },
}

/// A trait for putting an identity to platform
#[async_trait::async_trait]
pub trait PutIdentity<S: Signer<IdentityPublicKey>, WS: Signer<PlatformAddress>>: Waitable {
    /// Puts an identity on platform.
    ///
    /// TODO: Discuss if it should not actually consume self, since it is no longer valid (eg. identity id is changed)
    async fn send_to_platform(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding<WS>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Sends the identity and waits for confirmation proof.
    async fn send_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding<WS>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Self, Error>
    where
        Self: Sized;

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
            IdentityFunding::AssetLock {
                asset_lock_proof,
                asset_lock_private_key: *asset_lock_proof_private_key,
            },
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
            IdentityFunding::AssetLock {
                asset_lock_proof,
                asset_lock_private_key: *asset_lock_proof_private_key,
            },
            signer,
            settings,
        )
        .await
    }
}

// TODO: This should require addresses + identity, not only identity
#[async_trait::async_trait]
impl<S: Signer<IdentityPublicKey>, WS: Signer<PlatformAddress>> PutIdentity<S, WS> for Identity {
    async fn send_to_platform(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding<WS>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        send_to_identity_with_source(self, sdk, funding, signer, settings).await
    }

    async fn send_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding<WS>,
        signer: &S,
        settings: Option<PutSettings>,
    ) -> Result<Identity, Error> {
        let state_transition =
            send_to_identity_with_source(self, sdk, funding, signer, settings).await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}

async fn send_to_identity_with_source<S: Signer<IdentityPublicKey>, WS: Signer<PlatformAddress>>(
    identity: &Identity,
    sdk: &Sdk,
    funding: IdentityFunding<WS>,
    signer: &S,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    match &funding {
        IdentityFunding::AssetLock {
            asset_lock_proof,
            asset_lock_private_key,
        } => {
            let (state_transition, _) = identity.broadcast_request_for_new_identity(
                asset_lock_proof.to_owned(),
                asset_lock_private_key,
                signer,
                sdk.version(),
            )?;
            ensure_valid_state_transition_structure(&state_transition, sdk.version())?;
            state_transition.broadcast(sdk, settings).await?;
            Ok(state_transition)
        }
        IdentityFunding::AddressesWithPrivateKeys {
            inputs,
            change_output,
            input_private_keys,
        } => {
            if input_private_keys.is_empty() {
                return Err(Error::Generic(
                    "input_private_keys must contain at least one key".to_string(),
                ));
            }
            // Create address signer from inputs and private keys
            let addresses: Vec<PlatformAddress> = inputs.keys().cloned().collect();
            let address_signer =
                SimpleAddressSigner::from_addresses_and_keys(&addresses, input_private_keys)?;
            let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(sdk, &inputs).await?);
            send_identity_with_addresses(
                identity,
                sdk,
                inputs_with_nonce,
                *change_output,
                signer,
                &address_signer,
                settings,
            )
            .await
        }
        IdentityFunding::AddressesWithPrivateKeysAndNonce {
            inputs,
            change_output,
            input_private_keys,
        } => {
            if input_private_keys.is_empty() {
                return Err(Error::Generic(
                    "input_private_keys must contain at least one key".to_string(),
                ));
            }
            // Create address signer from inputs and private keys
            let addresses: Vec<PlatformAddress> = inputs.keys().cloned().collect();
            let address_signer =
                SimpleAddressSigner::from_addresses_and_keys(&addresses, input_private_keys)?;
            send_identity_with_addresses(
                identity,
                sdk,
                inputs.clone(),
                *change_output,
                signer,
                &address_signer,
                settings,
            )
            .await
        }
        IdentityFunding::AddressesWithSigner {
            inputs,
            change_output,
            address_signer,
        } => {
            let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(sdk, &inputs).await?);
            send_identity_with_addresses(
                identity,
                sdk,
                inputs_with_nonce,
                *change_output,
                signer,
                address_signer,
                settings,
            )
            .await
        }
        IdentityFunding::AddressesWithSignerAndNonce {
            inputs,
            change_output,
            address_signer,
        } => {
            send_identity_with_addresses(
                identity,
                sdk,
                inputs.clone(),
                *change_output,
                signer,
                address_signer,
                settings,
            )
            .await
        }
    }
}

async fn send_identity_with_addresses<S: Signer<IdentityPublicKey>, WS: Signer<PlatformAddress>>(
    identity: &Identity,
    sdk: &Sdk,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    output: Option<(PlatformAddress, Credits)>,
    signer: &S,
    address_signer: &WS,
    settings: Option<PutSettings>,
) -> Result<StateTransition, Error> {
    // Default fee strategy: deduct from first input
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
        signer,
        &address_signer,
        user_fee_increase,
        sdk.version(),
    )?;
    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

    state_transition.broadcast(sdk, settings).await?;
    Ok(state_transition)
}
