#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{AssetLockProof, UserFeeIncrease};
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::ProtocolError;
#[cfg(feature = "state-transition-signing")]
use platform_version::version::{FeatureVersion, PlatformVersion};

pub trait IdentityTopUpTransitionMethodsV0 {
    /// Build an `IdentityTopUp` state transition whose asset-lock-proof
    /// signature is produced from a raw `asset_lock_proof_private_key` held
    /// in-process.
    ///
    /// Prefer [`Self::try_from_identity_with_signer`] when the asset-lock key
    /// lives outside Rust (Swift / hardware wallet / HSM): the `_with_signer`
    /// variant routes asset-lock signing through an external
    /// [`key_wallet::signer::Signer`] so the private key never crosses the
    /// FFI boundary as raw bytes.
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_private_key(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>;

    /// Build an `IdentityTopUp` state transition whose asset-lock-proof
    /// signature is produced by an external [`key_wallet::signer::Signer`].
    ///
    /// The signer atomically derives, signs, and zeroises the key at
    /// `asset_lock_proof_path` inside its own trust boundary — the host only
    /// sees a 32-byte digest and the resulting Core-ECDSA signature.
    #[cfg(all(feature = "state-transition-signing", feature = "core_key_wallet"))]
    async fn try_from_identity_with_signer<AS>(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
        version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError>
    where
        AS: ::key_wallet::signer::Signer;

    /// Get State Transition type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityTopUp
    }
}
