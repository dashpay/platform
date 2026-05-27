#[cfg(feature = "state-transition-signing")]
use std::collections::BTreeMap;

#[cfg(feature = "state-transition-signing")]
use crate::address_funds::{AddressFundsFeeStrategy, PlatformAddress};
#[cfg(feature = "state-transition-signing")]
use crate::fee::Credits;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::{AddressNonce, AssetLockProof};
use crate::state_transition::StateTransitionType;
#[cfg(feature = "state-transition-signing")]
use crate::{prelude::UserFeeIncrease, state_transition::StateTransition, ProtocolError};
#[cfg(feature = "state-transition-signing")]
use platform_version::version::PlatformVersion;

pub trait AddressFundingFromAssetLockTransitionMethodsV0 {
    /// Build an `AddressFundingFromAssetLock` state transition where
    /// the asset-lock-proof signature is produced from a raw private
    /// key held in-process.
    ///
    /// `signer` signs each input's `AddressWitness` (one per
    /// `inputs.keys()`) over the transition's signable bytes; the
    /// outer state-transition signature is produced from
    /// `asset_lock_proof_private_key` via
    /// [`dashcore::signer::sign`].
    ///
    /// Prefer [`Self::try_from_asset_lock_with_signers`] when the
    /// asset-lock key lives outside Rust (Swift / hardware wallet /
    /// HSM): the `_with_signers` variant routes asset-lock signing
    /// through an external [`key_wallet::signer::Signer`] so the
    /// private key never crosses the FFI boundary as raw bytes.
    #[cfg(feature = "state-transition-signing")]
    #[allow(clippy::too_many_arguments)]
    async fn try_from_asset_lock_with_signer_and_private_key<S: Signer<PlatformAddress>>(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_private_key: &[u8],
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Option<Credits>>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>;

    /// Build an `AddressFundingFromAssetLock` state transition where
    /// the asset-lock-proof signature is produced by an external
    /// [`key_wallet::signer::Signer`].
    ///
    /// `signer` (`S: Signer<PlatformAddress>`) signs each input's
    /// `AddressWitness` (same as the legacy
    /// `try_from_asset_lock_with_signer_and_private_key` path), while
    /// `asset_lock_signer` (`AS: ::key_wallet::signer::Signer`)
    /// produces the outer state-transition ECDSA signature for the
    /// key at `asset_lock_proof_path` — atomically deriving, signing,
    /// and zeroising inside the signer's trust boundary. This is the
    /// signing path used by hosts that hold their private keys outside
    /// Rust (the iOS Swift SDK, hardware wallets, remote signers).
    #[cfg(all(feature = "state-transition-signing", feature = "core_key_wallet"))]
    #[allow(clippy::too_many_arguments)]
    async fn try_from_asset_lock_with_signers<S, AS>(
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
        inputs: BTreeMap<PlatformAddress, (AddressNonce, Credits)>,
        outputs: BTreeMap<PlatformAddress, Option<Credits>>,
        fee_strategy: AddressFundsFeeStrategy,
        signer: &S,
        asset_lock_signer: &AS,
        user_fee_increase: UserFeeIncrease,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>
    where
        S: Signer<PlatformAddress>,
        AS: ::key_wallet::signer::Signer;

    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::AddressFundingFromAssetLock
    }
}
