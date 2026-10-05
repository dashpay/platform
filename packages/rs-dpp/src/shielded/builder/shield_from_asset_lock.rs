#[cfg(feature = "core_key_wallet")]
use dashcore::OutPoint;

use crate::address_funds::{OrchardAddress, PlatformAddress};
#[cfg(feature = "core_key_wallet")]
use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
use crate::prelude::AssetLockProof;
use crate::shielded::shield_from_asset_lock_extra_sighash_data;
use crate::state_transition::shield_from_asset_lock_transition::methods::ShieldFromAssetLockTransitionMethodsV0;
use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

use super::{
    build_output_only_bundle, serialize_authorized_bundle, OrchardProver, SerializedBundle,
};
#[cfg(feature = "core_key_wallet")]
use super::{prove_output_only_bundle, ProvedOutputOnlyBundle};

/// A `ShieldFromAssetLock` Orchard bundle that has been proved and bound to its funding asset
/// lock but not yet wrapped into a signed transition. Proved by [`Self::prove`]; wrapped (any
/// number of times) by [`Self::build_transition_with_signer`].
///
/// What each part of the finished transition commits to:
///
/// - The Halo 2 proof covers the circuit's public inputs: each action's nullifier, `rk`, `cmx`
///   and `cv_net`, the anchor (the empty tree) and the flags.
/// - The binding signature (which also enforces the value balance against the `cv_net`s) and
///   the padding spends' spend-authorization signatures are over the platform sighash: the
///   bundle commitment — the actions including their note ciphertexts (so the recipient note and
///   the sender OVK's out-ciphertext), the flags, the value balance and the anchor, without
///   signatures or proof — plus [`shield_from_asset_lock_extra_sighash_data`]: a kind tag and the
///   double SHA-256 of the locked **outpoint**. They do not cover how the lock is proven
///   (InstantSend or ChainLock), the surplus output, or anything else about the transition.
/// - The outer ECDSA signature by the asset-lock key covers the whole transition except itself:
///   the asset-lock proof, the bundle fields (including the signatures above) and the surplus
///   output. `ShieldFromAssetLock` has no `user_fee_increase`.
///
/// The sighash is fixed when the bundle is proved, so everything up to the bundle signatures
/// depends only on the locked outpoint and value, the recipient and the sender OVK. The
/// expensive part — the proof — can be produced as soon as the asset-lock transaction exists,
/// before its InstantSend lock arrives, and stays valid when the transition is later wrapped
/// around a ChainLock proof of the same outpoint. Everything that depends on the lock proof is
/// signed at assembly time.
///
/// Every assembly signs the proved bundle afresh. RedPallas signatures are randomized, so two
/// transitions assembled from one proved bundle never hash alike — a client resubmitting after a
/// rejection relies on that, as Tenderdash caches the hashes of rejected transitions and drops an
/// identical resubmission.
#[cfg(feature = "core_key_wallet")]
pub struct ProvedShieldFromAssetLockBundle {
    bundle: ProvedOutputOnlyBundle,
    /// The asset-lock binding the bundle's sighash committed to (see
    /// [`shield_from_asset_lock_extra_sighash_data`]); empty at protocol versions that bind
    /// nothing.
    extra_sighash_data: Vec<u8>,
}

#[cfg(feature = "core_key_wallet")]
impl ProvedShieldFromAssetLockBundle {
    /// Proves the outputs-only bundle of a `ShieldFromAssetLock` that will be funded by the asset
    /// lock at `asset_lock_out_point`.
    ///
    /// The CPU-heavy half of [`build_shield_from_asset_lock_transition`] (Halo 2
    /// proof generation, seconds of work); it does no I/O and no signing, so it can run on a
    /// blocking thread while the caller waits for the lock's InstantSend lock. Parameters are as
    /// for [`build_shield_from_asset_lock_transition`].
    #[allow(clippy::too_many_arguments)]
    pub fn prove<P: OrchardProver>(
        recipient: &OrchardAddress,
        shield_amount: u64,
        asset_lock_out_point: OutPoint,
        prover: &P,
        memo: [u8; 36],
        sender_ovk: Option<grovedb_commitment_tree::OutgoingViewingKey>,
        dummy_outputs: usize,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        // The binding is a function of the locked outpoint alone: `AssetLockProof::create_identifier`
        // is the double SHA-256 of the outpoint for an InstantSend and a ChainLock proof alike
        // (pinned by `sighash`'s
        // `should_bind_the_whole_outpoint_of_the_asset_lock_not_only_its_transaction`). A chain
        // proof of the outpoint therefore stands in for the proof that does not exist yet; its
        // height is not part of the binding. Assembly re-derives the binding from the real proof
        // and refuses a mismatch.
        let lock_placeholder = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 0,
            out_point: asset_lock_out_point,
        });
        let extra_sighash_data =
            shield_from_asset_lock_extra_sighash_data(&lock_placeholder, platform_version)?;
        let bundle = prove_output_only_bundle(
            recipient,
            shield_amount,
            memo,
            sender_ovk,
            dummy_outputs,
            &extra_sighash_data,
            prover,
        )?;
        Ok(Self {
            bundle,
            extra_sighash_data,
        })
    }

    /// Whether this bundle binds exactly what consensus will re-derive for a transition carrying
    /// `asset_lock_proof` at `platform_version`: the same locked outpoint under the same binding
    /// rules. The kind of proof (InstantSend or ChainLock) does not matter. A bundle proved for
    /// another outpoint, or at a protocol version with a different binding, must be proved again
    /// — except across protocol versions that bind nothing (`credit_pool_bundle_binding` unset),
    /// where any lock matches, as it does for consensus.
    pub fn is_bound_for(
        &self,
        asset_lock_proof: &AssetLockProof,
        platform_version: &PlatformVersion,
    ) -> Result<bool, ProtocolError> {
        Ok(
            shield_from_asset_lock_extra_sighash_data(asset_lock_proof, platform_version)?
                == self.extra_sighash_data,
        )
    }

    /// Wraps the proved bundle into a `ShieldFromAssetLock` transition funded by
    /// `asset_lock_proof`: signs the bundle afresh (see the type docs) and has
    /// `asset_lock_signer` produce the outer ECDSA signature over the transition. Does not consume
    /// the proved bundle, so a rejected transition can be re-assembled (with a new asset-lock
    /// proof, or the same one) without proving again.
    ///
    /// Errors instead of producing a transition consensus would reject when the bundle is not
    /// bound to `asset_lock_proof` at `platform_version` (see [`Self::is_bound_for`]).
    pub async fn build_transition_with_signer<AS>(
        &self,
        asset_lock_proof: AssetLockProof,
        asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
        asset_lock_signer: &AS,
        surplus_output: Option<PlatformAddress>,
        platform_version: &PlatformVersion,
    ) -> Result<StateTransition, ProtocolError>
    where
        AS: ::key_wallet::signer::Signer,
    {
        if !self.is_bound_for(&asset_lock_proof, platform_version)? {
            return Err(ProtocolError::ShieldedBuildError(
                "shield_from_asset_lock: the asset lock proof does not match the binding the \
                 bundle was proved with (different outpoint or protocol version)"
                    .to_string(),
            ));
        }
        let sb = serialize_authorized_bundle(&self.bundle.authorize()?);
        let value_balance = pool_inflow(&sb)?;

        ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle_and_signer(
            asset_lock_proof,
            asset_lock_proof_path,
            asset_lock_signer,
            sb.actions,
            value_balance,
            sb.anchor,
            sb.proof,
            sb.binding_signature,
            surplus_output,
            platform_version,
        )
        .await
    }
}

/// Builds a ShieldFromAssetLock state transition (core asset lock -> shielded pool).
///
/// Like Shield, constructs an output-only Orchard bundle. The funds come from
/// a core asset lock proof rather than platform address inputs.
///
/// # Parameters
/// - `recipient` - Orchard address to receive the shielded note
/// - `shield_amount` - Amount of credits to shield (from the asset lock)
/// - `asset_lock_proof` - Proof that funds are locked on core chain
/// - `asset_lock_private_key` - Private key for the asset lock (signs the transition)
/// - `prover` - Orchard prover (holds the Halo 2 proving key)
/// - `memo` - 36-byte structured memo for the recipient (4-byte type tag + 32-byte payload)
/// - `sender_ovk` - The sender's outgoing viewing key (External scope). With `Some`, the
///   recipient output's `out_ciphertext` is encrypted under it so the sender can later
///   recover the sent note (recipient, value, memo) from chain data via OVK recovery —
///   the Zcash outgoing-transaction-history convention. With `None`, a random outgoing
///   cipher key is used and the sent note is unrecoverable by anyone.
/// - `surplus_output` - Optional platform address that receives the asset-lock surplus
///   (`asset_lock_value − shield_amount − fee`); when `None`, the surplus is added to the fee
///   pools, capped at `shielded_implicit_fee_cap`
/// - `dummy_outputs` - Number of extra zero-value anonymity-set filler outputs to append after
///   the real recipient output (unrecoverable random addresses, `None` OVK, empty memo). `0`
///   reproduces the historical single-output bundle exactly. The on-wire action count becomes
///   `max(1 + dummy_outputs, 2)`, which consensus prices the fee from — see the pool-seeding flow.
/// - `platform_version` - Protocol version
#[allow(clippy::too_many_arguments)]
pub fn build_shield_from_asset_lock_transition<P: OrchardProver>(
    recipient: &OrchardAddress,
    shield_amount: u64,
    asset_lock_proof: AssetLockProof,
    asset_lock_private_key: &[u8],
    prover: &P,
    memo: [u8; 36],
    sender_ovk: Option<grovedb_commitment_tree::OutgoingViewingKey>,
    surplus_output: Option<PlatformAddress>,
    dummy_outputs: usize,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError> {
    let (sb, value_balance) = build_bound_bundle(
        recipient,
        shield_amount,
        &asset_lock_proof,
        prover,
        memo,
        sender_ovk,
        dummy_outputs,
        platform_version,
    )?;

    ShieldFromAssetLockTransition::try_from_asset_lock_with_bundle(
        asset_lock_proof,
        asset_lock_private_key,
        sb.actions,
        value_balance,
        sb.anchor,
        sb.proof,
        sb.binding_signature,
        surplus_output,
        platform_version,
    )
}

/// Builds a ShieldFromAssetLock state transition where the
/// asset-lock-proof signature is produced by an external
/// [`key_wallet::signer::Signer`] (Swift / hardware-wallet / HSM
/// flow). The raw private key never crosses the FFI boundary;
/// derive + sign + zeroise happen inside the signer.
///
/// # Parameters
/// - `recipient` - Orchard address to receive the shielded note
/// - `shield_amount` - Amount of credits to shield (from the asset lock)
/// - `asset_lock_proof` - Proof that funds are locked on core chain
/// - `asset_lock_proof_path` - BIP32 path to the asset-lock key inside `asset_lock_signer`
/// - `asset_lock_signer` - External signer that produces the outer ECDSA signature
/// - `prover` - Orchard prover (holds the Halo 2 proving key)
/// - `memo` - 36-byte structured memo for the recipient (4-byte type tag + 32-byte payload)
/// - `sender_ovk` - The sender's outgoing viewing key (External scope). With `Some`, the
///   recipient output's `out_ciphertext` is encrypted under it so the sender can later
///   recover the sent note (recipient, value, memo) from chain data via OVK recovery —
///   the Zcash outgoing-transaction-history convention. With `None`, a random outgoing
///   cipher key is used and the sent note is unrecoverable by anyone.
/// - `surplus_output` - Optional platform address that receives the asset-lock surplus
///   (`asset_lock_value − shield_amount − fee`); when `None`, the surplus is added to the fee
///   pools, capped at `shielded_implicit_fee_cap`
/// - `dummy_outputs` - Number of extra zero-value anonymity-set filler outputs to append after
///   the real recipient output (unrecoverable random addresses, `None` OVK, empty memo). `0`
///   reproduces the historical single-output bundle exactly. The on-wire action count becomes
///   `max(1 + dummy_outputs, 2)`, which consensus prices the fee from — see the pool-seeding flow.
/// - `platform_version` - Protocol version
#[cfg(feature = "core_key_wallet")]
#[allow(clippy::too_many_arguments)]
pub async fn build_shield_from_asset_lock_transition_with_signer<P, AS>(
    recipient: &OrchardAddress,
    shield_amount: u64,
    asset_lock_proof: AssetLockProof,
    asset_lock_proof_path: &::key_wallet::bip32::DerivationPath,
    asset_lock_signer: &AS,
    prover: &P,
    memo: [u8; 36],
    sender_ovk: Option<grovedb_commitment_tree::OutgoingViewingKey>,
    surplus_output: Option<PlatformAddress>,
    dummy_outputs: usize,
    platform_version: &PlatformVersion,
) -> Result<StateTransition, ProtocolError>
where
    P: OrchardProver,
    AS: ::key_wallet::signer::Signer,
{
    // The same proof-then-assembly a client proving ahead of the lock proof goes through.
    let asset_lock_out_point = asset_lock_proof.out_point().ok_or_else(|| {
        ProtocolError::IdentifierError(String::from("No output at a given index"))
    })?;
    ProvedShieldFromAssetLockBundle::prove(
        recipient,
        shield_amount,
        asset_lock_out_point,
        prover,
        memo,
        sender_ovk,
        dummy_outputs,
        platform_version,
    )?
    .build_transition_with_signer(
        asset_lock_proof,
        asset_lock_proof_path,
        asset_lock_signer,
        surplus_output,
        platform_version,
    )
    .await
}

/// Proves and signs the outputs-only bundle of a `ShieldFromAssetLock` funded by
/// `asset_lock_proof` and returns it with the amount it moves into the pool, for the raw-key
/// builder. It binds the same preimage as [`ProvedShieldFromAssetLockBundle::prove`], which the
/// external-signer builder goes through.
#[allow(clippy::too_many_arguments)]
fn build_bound_bundle<P: OrchardProver>(
    recipient: &OrchardAddress,
    shield_amount: u64,
    asset_lock_proof: &AssetLockProof,
    prover: &P,
    memo: [u8; 36],
    sender_ovk: Option<grovedb_commitment_tree::OutgoingViewingKey>,
    dummy_outputs: usize,
    platform_version: &PlatformVersion,
) -> Result<(SerializedBundle, u64), ProtocolError> {
    // Bound to the funding asset lock, so nobody can re-wrap the proved bundle around a lock
    // of their own; empty at protocol versions whose verifier predates the binding.
    let extra_sighash_data =
        shield_from_asset_lock_extra_sighash_data(asset_lock_proof, platform_version)?;
    let bundle = build_output_only_bundle(
        recipient,
        shield_amount,
        memo,
        sender_ovk,
        dummy_outputs,
        &extra_sighash_data,
        prover,
    )?;
    let sb = serialize_authorized_bundle(&bundle);
    let value_balance = pool_inflow(&sb)?;
    Ok((sb, value_balance))
}

/// The amount an outputs-only bundle moves into the pool. Its Orchard value_balance is negative
/// (value flowing in); this is the absolute amount, as a `u64`.
fn pool_inflow(sb: &SerializedBundle) -> Result<u64, ProtocolError> {
    sb.value_balance
        .checked_neg()
        .and_then(|v| u64::try_from(v).ok())
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError(
                "shield_from_asset_lock: bundle value_balance is not negative".to_string(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::super::{build_output_only_bundle, serialize_authorized_bundle};
    use crate::shielded::builder::test_helpers::{test_orchard_address, TestProver};

    /// Verifies that an output-only bundle produces a negative value_balance
    /// (value flowing into the pool), which is the precondition for
    /// shield_from_asset_lock's value_balance conversion.
    #[test]
    fn test_output_only_bundle_value_balance_is_negative() {
        let recipient = test_orchard_address();
        let amount = 50_000u64;

        let bundle =
            build_output_only_bundle(&recipient, amount, [0u8; 36], None, 0, &[], &TestProver)
                .expect("bundle should build successfully");
        let sb = serialize_authorized_bundle(&bundle);

        // Output-only bundles have negative value_balance (value entering the pool)
        assert!(
            sb.value_balance < 0,
            "expected negative value_balance, got {}",
            sb.value_balance
        );

        // The absolute value should match the shield amount
        let abs_balance = sb
            .value_balance
            .checked_neg()
            .and_then(|v| u64::try_from(v).ok())
            .expect("value_balance should be safely negatable");
        assert_eq!(abs_balance, amount);
    }

    /// Consensus prices the shielded fee from the on-wire `actions.len()`, and the wallet reserves
    /// the fee for exactly 2 actions (Orchard's `MIN_ACTIONS`). A single-output, spends-disabled
    /// bundle must therefore serialize to exactly 2 on-wire actions. If a future Orchard or builder
    /// change alters that padding, the hardcoded wallet reservation would diverge from what consensus
    /// charges (a valid client tx would be rejected); this test fails loudly if that invariant breaks.
    #[test]
    fn test_output_only_bundle_serializes_to_min_actions() {
        let recipient = test_orchard_address();
        let bundle =
            build_output_only_bundle(&recipient, 50_000u64, [0u8; 36], None, 0, &[], &TestProver)
                .expect("bundle should build");
        let sb = serialize_authorized_bundle(&bundle);
        assert_eq!(
            sb.actions.len(),
            2,
            "single-output shield bundle must pad to exactly 2 on-wire actions"
        );
    }

    // -------------------------------------------------------------
    // Arithmetic edge cases on the value_balance conversion branch
    // (the `checked_neg().and_then(u64::try_from)` chain).
    // -------------------------------------------------------------

    #[test]
    fn test_value_balance_positive_would_fail_conversion() {
        // This is a regression-guard: if a *positive* value_balance ever
        // reached the conversion path, `checked_neg` on i64::MIN would
        // overflow and the `.try_from::<u64>` on a negative value would
        // fail. We simulate by constructing a hypothetical value_balance
        // scenario rather than calling the high-level builder (which
        // requires a real AssetLockProof).
        let positive: i64 = 123;
        let converted = positive.checked_neg().and_then(|v| u64::try_from(v).ok());
        assert!(converted.is_none(), "negative result cannot be u64");

        let zero: i64 = 0;
        let converted_zero = zero.checked_neg().and_then(|v| u64::try_from(v).ok());
        assert_eq!(converted_zero, Some(0));

        let negative: i64 = -42;
        let converted_neg = negative.checked_neg().and_then(|v| u64::try_from(v).ok());
        assert_eq!(converted_neg, Some(42));
    }

    #[test]
    fn test_output_only_various_amounts_negative_balance() {
        // Try several amounts to ensure the helper consistently produces a
        // negative value_balance equal in magnitude to the requested amount.
        for amount in [1u64, 100, 1_000_000, u32::MAX as u64] {
            let recipient = test_orchard_address();
            let bundle =
                build_output_only_bundle(&recipient, amount, [0u8; 36], None, 0, &[], &TestProver)
                    .expect("bundle should build");
            let sb = serialize_authorized_bundle(&bundle);
            assert_eq!(
                sb.value_balance,
                -(amount as i64),
                "value_balance mismatch for amount {}",
                amount
            );
        }
    }
}
