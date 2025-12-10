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
use std::collections::BTreeMap;

/// Funding sources supported when creating an identity.
///
/// For address-based funding, the caller must provide a signer that implements
/// `Signer<PlatformAddress>` separately via the trait methods.
pub enum IdentityFunding {
    AssetLock {
        asset_lock_proof: AssetLockProof,
        asset_lock_private_key: PrivateKey,
    },
    Addresses {
        inputs: BTreeMap<PlatformAddress, Credits>,
    },
    AddressesWithNonce {
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    },
}

/// A trait for putting an identity to platform
#[async_trait::async_trait]
pub trait PutIdentity<S: Signer<IdentityPublicKey>, A: Signer<PlatformAddress> + Send + Sync>:
    Waitable
{
    /// Sends a new identity to Platform using the provided funding source.
    ///
    /// For `IdentityFunding::Addresses` or `IdentityFunding::AddressesWithNonce`,
    /// an `address_signer` implementing `Signer<PlatformAddress>` must be provided.
    /// For `IdentityFunding::AssetLock`, `address_signer` can be `None`.
    async fn send_to_platform(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding,
        signer: &S,
        address_signer: Option<&A>,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error>;

    /// Sends the identity and waits for confirmation proof.
    async fn send_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding,
        signer: &S,
        address_signer: Option<&A>,
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
            None,
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
            None,
            settings,
        )
        .await
    }
}
#[async_trait::async_trait]
impl<S: Signer<IdentityPublicKey> + Send + Sync, A: Signer<PlatformAddress> + Send + Sync>
    PutIdentity<S, A> for Identity
{
    async fn send_to_platform(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding,
        signer: &S,
        address_signer: Option<&A>,
        settings: Option<PutSettings>,
    ) -> Result<StateTransition, Error> {
        send_to_identity_with_source(self, sdk, funding, signer, address_signer, settings).await
    }

    async fn send_to_platform_and_wait_for_response(
        &self,
        sdk: &Sdk,
        funding: IdentityFunding,
        signer: &S,
        address_signer: Option<&A>,
        settings: Option<PutSettings>,
    ) -> Result<Identity, Error> {
        let state_transition =
            send_to_identity_with_source(self, sdk, funding, signer, address_signer, settings)
                .await?;

        Self::wait_for_response(sdk, state_transition, settings).await
    }
}

async fn send_to_identity_with_source<S: Signer<IdentityPublicKey>, A: Signer<PlatformAddress>>(
    identity: &Identity,
    sdk: &Sdk,
    funding: IdentityFunding,
    signer: &S,
    address_signer: Option<&A>,
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
        IdentityFunding::Addresses { inputs } => {
            let address_signer = address_signer.ok_or_else(|| {
                Error::Generic("address_signer is required for address-based funding".to_string())
            })?;
            let inputs_with_nonce = nonce_inc(fetch_inputs_with_nonce(sdk, inputs).await?);
            send_identity_with_addresses(
                identity,
                sdk,
                inputs_with_nonce,
                signer,
                address_signer,
                settings,
            )
            .await
        }
        IdentityFunding::AddressesWithNonce { inputs } => {
            let address_signer = address_signer.ok_or_else(|| {
                Error::Generic("address_signer is required for address-based funding".to_string())
            })?;
            send_identity_with_addresses(
                identity,
                sdk,
                inputs.clone(),
                signer,
                address_signer,
                settings,
            )
            .await
        }
    }
}

async fn send_identity_with_addresses<S: Signer<IdentityPublicKey>, A: Signer<PlatformAddress>>(
    identity: &Identity,
    sdk: &Sdk,
    inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
    signer: &S,
    address_signer: &A,
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
        fee_strategy,
        signer,
        address_signer,
        user_fee_increase,
        sdk.version(),
    )?;
    ensure_valid_state_transition_structure(&state_transition, sdk.version())?;

    state_transition.broadcast(sdk, settings).await?;
    Ok(state_transition)
}
