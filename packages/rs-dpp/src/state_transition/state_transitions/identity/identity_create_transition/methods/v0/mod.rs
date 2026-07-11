#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::Identity;
#[cfg(feature = "state-transition-signing")]
use crate::identity::IdentityPublicKey;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::AssetLockProof;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::StateTransition;
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{BlsModule, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait IdentityCreateTransitionMethodsV0 {
    /// Build an `IdentityCreate` state transition that holds the
    /// asset-lock-proof private key in-process.
    ///
    /// The `signer` parameter signs each `IdentityPublicKey` witness (the
    /// per-key signatures in `public_keys[]`), while the asset-lock-proof
    /// signature on the outer state transition is produced from
    /// `asset_lock_proof_private_key` via [`StateTransition::sign_by_private_key`].
    ///
    /// Prefer [`Self::try_from_identity_with_signers`] when the asset-lock
    /// key lives outside Rust (Swift / hardware wallet / HSM): the
    /// `_with_signers` variant routes asset-lock signing through an external
    /// [`key_wallet::signer::Signer`] so the private key never crosses the FFI
    /// boundary as raw bytes.
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity_with_signer_and_private_key<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        signer: &S,
        bls: &impl BlsModule,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Build an `IdentityCreate` state transition where the asset-lock-proof
    /// signature is produced by an external [`key_wallet::signer::Signer`].
    ///
    /// `identity_signer` signs the per-key witnesses on `public_keys[]` (same
    /// as the legacy `try_from_identity_with_signer_and_private_key` path),
    /// while `asset_lock_signer` produces the outer state-transition ECDSA
    /// signature for the key at `asset_lock_proof_path` — atomically deriving,
    /// signing, and zeroising inside the signer's trust boundary. This is the
    /// signing path used by hosts that hold their private keys outside Rust
    /// (the iOS Swift SDK, hardware wallets, remote signers).
    #[cfg(all(feature = "state-transition-signing", feature = "core_key_wallet"))]
    #[allow(clippy::too_many_arguments)]
    async fn try_from_identity_with_signers<IS, AS>(
        identity: &Identity,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
        identity_signer: &IS,
        asset_lock_signer: &AS,
        bls: &impl BlsModule,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>
    where
        IS: Signer<IdentityPublicKey>,
        AS: ::key_wallet::signer::Signer;

    /// Get State Transition type
    fn get_type() -> StateTransitionType;
}
