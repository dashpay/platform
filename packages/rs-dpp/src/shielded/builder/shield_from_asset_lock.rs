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
use super::{output_only_bundle_builder, prove_bundle_with, SignBundleFn};

/// A `ShieldFromAssetLock` Orchard bundle, proved for a locked outpoint but not yet wrapped into
/// a transition. Proved by [`Self::prove`]; wrapped any number of times by
/// [`Self::build_transition_with_signer`].
///
/// The bundle's sighash binds the locked outpoint (see
/// [`shield_from_asset_lock_extra_sighash_data`]), not the proof that presents it, so the Halo 2
/// proof can be made before the InstantSend lock arrives and reused under a ChainLock proof of the
/// same outpoint. Only the outer ECDSA signature covers the asset-lock proof.
///
/// Each assembly re-signs the bundle with fresh randomness, so a resubmission never repeats a
/// rejected transition's hash (Tenderdash caches those and drops repeats). To allow that, the value
/// holds the bundle's signing secrets, with which its holder could re-bind the proof to another
/// asset lock and fund the same note twice: keep it in memory, and drop it once the transition has
/// landed or been abandoned.
#[cfg(feature = "core_key_wallet")]
pub struct ProvedShieldFromAssetLockBundle {
    /// Signs the proved bundle (see `prove_bundle_with`). The proved-but-unsigned bundle type is
    /// not re-exported by `grovedb-commitment-tree`, so it lives inside this closure.
    sign: Box<SignBundleFn>,
    /// The outpoint the bundle was proved for. Checked on its own, because at versions that bind
    /// nothing the sighash would accept a proof of any outpoint.
    asset_lock_out_point: OutPoint,
    /// The asset-lock binding the sighash committed to; empty at versions that bind nothing.
    extra_sighash_data: Vec<u8>,
}

#[cfg(feature = "core_key_wallet")]
impl ProvedShieldFromAssetLockBundle {
    /// Proves the outputs-only bundle of a `ShieldFromAssetLock` that the asset lock at
    /// `asset_lock_out_point` will fund: the CPU-heavy half (seconds of Halo 2 proving) of
    /// [`build_shield_from_asset_lock_transition_with_signer`], whose parameters it shares. It does
    /// no I/O and no signing, so it can run on a blocking thread while the lock's InstantSend lock
    /// is awaited.
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
        // The binding depends on the locked outpoint alone, never on the proof kind or the
        // chain-lock height, so a chain proof of the outpoint stands in for the proof that does
        // not exist yet. Assembly re-derives the binding from the real proof.
        let lock_placeholder = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 0,
            out_point: asset_lock_out_point,
        });
        let extra_sighash_data =
            shield_from_asset_lock_extra_sighash_data(&lock_placeholder, platform_version)?;
        let builder =
            output_only_bundle_builder(recipient, shield_amount, memo, sender_ovk, dummy_outputs)?;
        let bound = extra_sighash_data.clone();
        let sign = prove_bundle_with(builder, prover, move |_| Ok(bound))?;
        Ok(Self {
            sign,
            asset_lock_out_point,
            extra_sighash_data,
        })
    }

    /// Wraps the proved bundle into a `ShieldFromAssetLock` transition funded by
    /// `asset_lock_proof`: signs the bundle afresh and has `asset_lock_signer` produce the outer
    /// ECDSA signature. Does not consume the proved bundle, so a rejected transition can be
    /// re-assembled, with a new asset-lock proof or the same one, without proving again.
    ///
    /// Errors when `asset_lock_proof` is for another outpoint, even at versions whose sighash
    /// binds nothing (one proved bundle around two locks would fund the same note twice), and,
    /// rather than produce a transition consensus would reject, when `platform_version` binds
    /// differently from the version the bundle was proved at.
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
        if asset_lock_proof.out_point() != Some(self.asset_lock_out_point)
            || shield_from_asset_lock_extra_sighash_data(&asset_lock_proof, platform_version)?
                != self.extra_sighash_data
        {
            return Err(ProtocolError::ShieldedBuildError(
                "shield_from_asset_lock: the asset lock proof does not match the binding the \
                 bundle was proved with (different outpoint or protocol version)"
                    .to_string(),
            ));
        }
        let sb = serialize_authorized_bundle(&(self.sign)(&[])?);
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

    #[cfg(feature = "core_key_wallet")]
    #[test]
    fn proved_bundle_signs_afresh_and_verifies_against_a_real_lock_proof_binding() {
        // Consensus derives the sighash from the transition's own lock proof, not from what the
        // builder stored: verify against that, for a proof `prove` never saw, after each of two
        // signings (`signing_tests` checks they share the proof but not the signatures).
        use super::ProvedShieldFromAssetLockBundle;
        use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
        use crate::prelude::AssetLockProof;
        use crate::shielded::{
            compute_platform_sighash, shield_from_asset_lock_extra_sighash_data,
        };
        use dashcore::OutPoint;
        use grovedb_commitment_tree::{BatchValidator, VerifyingKey};
        use platform_version::version::PlatformVersion;
        use rand::rngs::OsRng;

        let platform_version = PlatformVersion::latest();
        let out_point = OutPoint::from([0x55; 36]);
        let proved = ProvedShieldFromAssetLockBundle::prove(
            &test_orchard_address(),
            50_000,
            out_point,
            &TestProver,
            [0u8; 36],
            None,
            0,
            platform_version,
        )
        .expect("bundle should prove");
        let first = (proved.sign)(&[]).expect("bundle should sign");
        let second = (proved.sign)(&[]).expect("bundle should sign again");

        let lock = AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 100,
            out_point,
        });
        let extra =
            shield_from_asset_lock_extra_sighash_data(&lock, platform_version).expect("binding");
        assert!(!extra.is_empty(), "the latest version binds the asset lock");
        let commitment: [u8; 32] = first.commitment().into();
        let vk = VerifyingKey::build();
        let verifies = |bundle, extra: &[u8]| {
            let mut batch = BatchValidator::new();
            batch.add_bundle(bundle, compute_platform_sighash(&commitment, extra));
            batch.validate(&vk, OsRng)
        };
        assert!(verifies(&first, &extra));
        assert!(verifies(&second, &extra));
        assert!(
            !verifies(&first, &extra[..1]),
            "signatures bind the proving-time sighash"
        );
    }
}
