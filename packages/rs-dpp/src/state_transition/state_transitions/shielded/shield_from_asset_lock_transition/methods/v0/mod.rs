#[cfg(feature = "state-transition-signing")]
use crate::address_funds::PlatformAddress;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::AssetLockProof;
#[cfg(feature = "state-transition-signing")]
use crate::shielded::SerializedAction;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait ShieldFromAssetLockTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    fn try_from_asset_lock_with_bundle(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        actions: Vec<SerializedAction>,
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        surplus_output: Option<PlatformAddress>,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Build a `ShieldFromAssetLock` state transition where the
    /// asset-lock-proof signature is produced by an external
    /// [`key_wallet::signer::Signer`].
    ///
    /// `asset_lock_signer` produces the outer state-transition ECDSA
    /// signature for the key at `asset_lock_proof_path` — atomically
    /// deriving, signing, and zeroising inside the signer's trust
    /// boundary. This is the signing path used by hosts that hold
    /// their private keys outside Rust (the iOS Swift SDK, hardware
    /// wallets, remote signers); the raw key never crosses the FFI
    /// boundary.
    #[cfg(all(feature = "state-transition-signing", feature = "core_key_wallet"))]
    #[allow(clippy::too_many_arguments)]
    async fn try_from_asset_lock_with_bundle_and_signer<AS>(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        actions: Vec<SerializedAction>,
        value_balance: u64,
        anchor: [u8; 32],
        proof: Vec<u8>,
        binding_signature: [u8; 64],
        surplus_output: Option<PlatformAddress>,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>
    where
        AS: ::key_wallet::signer::Signer;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::ShieldFromAssetLock
    }
}
