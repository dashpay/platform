//! Broadcast trait representing the action of broadcasting a new identity state transition to Platform.
//!
//! The [BroadcastRequestForNewIdentity] trait is designed for the creation and broadcasting of new identity state transitions.
//! This involves the generation of a state transition object, signing it, and then broadcasting it to Platform.
//!
//! This trait is expected to be implemented by objects that encapsulate the necessary data and logic to perform
//! these operations, including the handling of asset lock proof and signing operations.
use std::fmt::Debug;

use dapi_grpc::platform::v0::{self as proto, BroadcastStateTransitionRequest};
use dpp::dashcore::PrivateKey;
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::native_bls::NativeBlsModule;
use dpp::prelude::{AssetLockProof, Identity};

use super::put_settings::PutSettings;
use dpp::state_transition::identity_create_transition::methods::IdentityCreateTransitionMethodsV0;
use dpp::state_transition::identity_create_transition::IdentityCreateTransition;
use dpp::state_transition::StateTransition;
use dpp::version::PlatformVersion;
use rs_dapi_client::transport::TransportRequest;

use super::broadcast_request::BroadcastRequestForStateTransition;
use super::validation::ensure_valid_state_transition_structure;
use crate::error::Error;

/// Trait implemented by objects that can be used to broadcast new identity state transitions.
///
/// [BroadcastRequestForNewIdentity] trait is used when a new identity needs to be created and broadcasted on Platform.
/// It encapsulates the data, the signing process, and the logic required to perform the broadcast operation.
///
/// Implementors of this trait will typically be responsible for creating an identity state transition,
/// signing it with the provided private key and signer, and preparing it for transport to Platform.
///
/// ## Example
///
/// To broadcast a new [Identity](dpp::prelude::Identity) state transition, you would typically
/// create an [IdentityCreateTransition](dpp::state_transition::identity_create_transition::IdentityCreateTransition),
/// sign it, and use the `broadcast_new_identity` method provided by this trait:
///
/// ```rust, ignore
///
/// use dash_sdk::{Sdk, platform::{BroadcastNewIdentity, IdentityCreateTransition}};
/// use dpp::identity::signer::Signer;
/// use dpp::prelude::{AssetLockProof, PrivateKey};
/// use dpp::version::PlatformVersion;
///
/// let mut sdk = Sdk::new_mock();
/// let asset_lock_proof = AssetLockProof::new(/* parameters for the asset lock proof */);
/// let private_key = PrivateKey::from(/* private key data */);
/// let signer = /* implementation of Signer trait */;
/// let platform_version = PlatformVersion::latest();
///
/// let identity_transition = IdentityCreateTransition::new(/* parameters for the transition */);
/// let result = identity_transition.broadcast_new_identity(asset_lock_proof, private_key, &signer, &platform_version);
///
/// match result {
///     Ok(transport_request) => {
///         // The transport_request can now be sent to Platform to broadcast the new identity.
///     }
///     Err(e) => {
///         // Handle the error
///     }
/// }
/// ```
///
/// As [BroadcastRequestForNewIdentity] is a trait, it can be implemented for any type that represents
/// a new identity creation operation, allowing for flexibility in how new identities are broadcasted.
pub(crate) trait BroadcastRequestForNewIdentity<T: TransportRequest, S: Signer<IdentityPublicKey>>:
    Send + Debug + Clone
{
    /// Converts the current instance into an instance of the `TransportRequest` type, ready for broadcasting.
    ///
    /// This method takes ownership of the instance upon which it's called (hence `self`), and attempts to perform the conversion,
    /// including signing the state transition with the provided private key and signer.
    ///
    /// # Arguments
    ///
    /// * `asset_lock_proof` - The proof that locks the asset which is used to create the identity.
    /// * `asset_lock_proof_private_key` - The private key associated with the asset lock proof.
    /// * `signer` - The signer to be used for signing the state transition.
    /// * `platform_version` - The version of Platform for which the state transition is intended.
    ///
    /// # Returns
    /// On success, this method yields an instance of the `TransportRequest` type (`T`), which can be used to broadcast the new identity state transition to Platform.
    /// On failure, it yields an [`Error`].
    ///
    /// # Error Handling
    /// This method propagates any errors encountered during the signing or conversion process.
    /// These are returned as [`Error`] instances.
    ///
    /// Prefer [`Self::broadcast_request_for_new_identity_with_signer`] when
    /// the asset-lock private key lives outside Rust (Swift / hardware wallet
    /// / HSM): the `_with_signer` variant routes asset-lock signing through
    /// an external [`key_wallet::signer::Signer`] so the private key never
    /// crosses the FFI boundary as raw bytes.
    #[allow(async_fn_in_trait)]
    async fn broadcast_request_for_new_identity_with_private_key(
        &self,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        platform_version: &PlatformVersion,
        settings: Option<PutSettings>,
    ) -> Result<(StateTransition, BroadcastStateTransitionRequest), Error>;

    /// Signer-driven counterpart to
    /// [`Self::broadcast_request_for_new_identity_with_private_key`].
    ///
    /// `identity_signer` signs the per-key witnesses on `public_keys[]`,
    /// while `asset_lock_signer` produces the outer state-transition ECDSA
    /// signature for the key at `asset_lock_proof_path` — atomically
    /// deriving, signing, and zeroising inside the signer's trust boundary.
    ///
    /// `settings.user_fee_increase` is the percentage multiplier the
    /// caller wants applied to the ST's processing fee. Threading it
    /// through the builder is load-bearing: it both affects fee
    /// accounting AND changes the ST's signable bytes, which the
    /// upstream CL-height retry path in `platform-wallet` relies on
    /// to bypass Tenderdash's invalid-tx hash cache
    /// (`keep-invalid-txs-in-cache = true` in dashmate's
    /// mainnet/testnet templates). `None` / unset = unaltered fees.
    #[cfg(feature = "core_key_wallet")]
    #[allow(async_fn_in_trait)]
    async fn broadcast_request_for_new_identity_with_signer<AS>(
        &self,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        identity_signer: &S,
        platform_version: &PlatformVersion,
        settings: Option<PutSettings>,
    ) -> Result<(StateTransition, BroadcastStateTransitionRequest), Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync;
}

impl<S: Signer<IdentityPublicKey>>
    BroadcastRequestForNewIdentity<proto::BroadcastStateTransitionRequest, S> for Identity
{
    async fn broadcast_request_for_new_identity_with_private_key(
        &self,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &PrivateKey,
        signer: &S,
        platform_version: &PlatformVersion,
        settings: Option<PutSettings>,
    ) -> Result<(StateTransition, BroadcastStateTransitionRequest), Error> {
        let user_fee_increase = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();
        let identity_create_transition =
            IdentityCreateTransition::try_from_identity_with_signer_and_private_key(
                self,
                asset_lock_proof,
                asset_lock_proof_private_key.inner.as_ref(),
                signer,
                &NativeBlsModule,
                user_fee_increase,
                platform_version,
            )
            .await?;
        ensure_valid_state_transition_structure(&identity_create_transition, platform_version)?;
        let request = identity_create_transition.broadcast_request_for_state_transition()?;
        Ok((identity_create_transition, request))
    }

    #[cfg(feature = "core_key_wallet")]
    async fn broadcast_request_for_new_identity_with_signer<AS>(
        &self,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &dpp::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        identity_signer: &S,
        platform_version: &PlatformVersion,
        settings: Option<PutSettings>,
    ) -> Result<(StateTransition, BroadcastStateTransitionRequest), Error>
    where
        AS: dpp::key_wallet::signer::Signer + Send + Sync,
    {
        let user_fee_increase = settings
            .and_then(|s| s.user_fee_increase)
            .unwrap_or_default();
        let identity_create_transition = IdentityCreateTransition::try_from_identity_with_signers(
            self,
            asset_lock_proof,
            asset_lock_proof_path,
            identity_signer,
            asset_lock_signer,
            &NativeBlsModule,
            user_fee_increase,
            platform_version,
        )
        .await?;
        ensure_valid_state_transition_structure(&identity_create_transition, platform_version)?;
        let request = identity_create_transition.broadcast_request_for_state_transition()?;
        Ok((identity_create_transition, request))
    }
}
