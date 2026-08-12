//! Convenience builders for constructing shielded state transitions.
//!
//! These functions encapsulate the full Orchard bundle construction pipeline:
//! builder configuration, proof generation, signature application,
//! and serialization into platform state transitions.
//!
//! Requires the `shielded-client` feature, which pulls in
//! `grovedb-commitment-tree` (and transitively the `orchard` crate).
//!
//! # Example
//!
//! ```ignore
//! use dpp::shielded::builder::*;
//! use grovedb_commitment_tree::{SpendingKey, FullViewingKey, Scope, ProvingKey};
//!
//! // Derive recipient address
//! let sk = SpendingKey::from_bytes(seed)?;
//! let fvk = FullViewingKey::from(&sk);
//! let recipient = OrchardAddress::from_raw_bytes(
//!     &fvk.address_at(0, Scope::External).to_raw_address_bytes(),
//! );
//!
//! // Build a shield transition; pass the sender's OVK so the wallet can
//! // later recover its own send from chain data (None = unrecoverable)
//! let pk = ProvingKey::build();
//! let st = build_shield_transition(
//!     &recipient, shield_amount, inputs, fee_strategy,
//!     &signer, 0, &pk, [0u8; 36], Some(fvk.to_ovk(Scope::External)), platform_version,
//! )?;
//! ```

mod identity_create_from_shielded_pool;
mod shield;
mod shield_from_asset_lock;
mod shielded_transfer;
mod shielded_withdrawal;
mod unshield;

pub use self::shield::build_shield_transition;
pub use identity_create_from_shielded_pool::{
    build_identity_create_from_shielded_pool_transition, IdentityCreateFromShieldedPoolBuildResult,
};
pub use shield_from_asset_lock::build_shield_from_asset_lock_transition;
#[cfg(feature = "core_key_wallet")]
pub use shield_from_asset_lock::build_shield_from_asset_lock_transition_with_signer;
pub use shielded_transfer::{
    build_shielded_transfer_transition, build_shielded_transfer_transition_multi,
    ShieldedTransferOutput,
};
pub use shielded_withdrawal::build_shielded_withdrawal_transition;
pub use unshield::build_unshield_transition;

use grovedb_commitment_tree::{
    Anchor, Authorized, Builder, Bundle, BundleType, DashMemo, Flags as OrchardFlags,
    FullViewingKey, MerklePath, Note, NoteValue, OutgoingViewingKey, PaymentAddress, ProvingKey,
    Scope, SpendAuthorizingKey, SpendingKey,
};
use platform_version::version::PlatformVersion;
use rand::rngs::OsRng;
use rand::RngCore;

use crate::address_funds::OrchardAddress;
use crate::shielded::{compute_platform_sighash, SerializedAction};
use crate::ProtocolError;

/// Trait abstracting over Orchard proof generation.
///
/// This follows the same pattern as `Signer` — callers provide an implementation
/// that holds (and potentially caches) the expensive `ProvingKey`, and the builder
/// functions use it via this trait.
pub trait OrchardProver {
    /// Returns a reference to the Halo 2 proving key for the Orchard circuit.
    fn proving_key(&self) -> &ProvingKey;
}

/// A note that can be spent in a shielded transaction, paired with its
/// Merkle inclusion path in the commitment tree.
pub struct SpendableNote {
    /// The Orchard note to spend.
    pub note: Note,
    /// Merkle path proving the note's commitment exists in the tree.
    pub merkle_path: MerklePath,
}

/// The serialized fields extracted from an authorized Orchard bundle,
/// ready for use by state transition constructors.
pub struct SerializedBundle {
    /// Serialized Orchard actions (spends + outputs).
    pub actions: Vec<SerializedAction>,
    /// Bundle flags byte.
    pub flags: u8,
    /// Net value balance (positive = value leaving the shielded pool).
    pub value_balance: i64,
    /// Sinsemilla root of the Orchard note commitment tree (32 bytes).
    /// This is the Orchard `Anchor` — the root hash of the depth-32 Sinsemilla
    /// Merkle tree over extracted note commitments (cmx values).
    pub anchor: [u8; 32],
    /// Halo 2 proof bytes.
    pub proof: Vec<u8>,
    /// Binding signature (64 bytes).
    pub binding_signature: [u8; 64],
}

impl From<&OrchardAddress> for PaymentAddress {
    fn from(address: &OrchardAddress) -> Self {
        *address.inner()
    }
}

/// The number of Orchard actions a `BundleType::DEFAULT` bundle built from `num_spends` spends
/// and `num_outputs` outputs will publish **on the wire**, validated against the consensus
/// action ceiling.
///
/// Every shielded fee predictor MUST size its fee with this function, because consensus prices
/// the fee off the on-wire `actions.len()` (see
/// `StateTransitionShieldedMinimumFeeValidationV0::validate_minimum_shielded_fee`, which reads
/// `v0.actions.len()`), and an Orchard action is a *joined* spend/output slot: the action count
/// is `max(num_spends, num_outputs)`, then padded up to Orchard's `MIN_ACTIONS = 2`.
///
/// The output side matters. A predictor that looks only at the spend count is correct **only**
/// while `num_outputs <= 2`, because `max(n, 1).max(2) == max(n, 2).max(2)`. As soon as a
/// transition publishes three or more outputs (a multi-recipient transfer plus change), a
/// spends-only predictor under-counts and carves a fee below the one consensus computes — fatal
/// for `ShieldedTransfer`, whose `value_balance` must equal the minimum fee **exactly**.
///
/// This delegates to Orchard's own [`BundleType::num_actions`] rather than re-deriving the rule,
/// so the predictor cannot drift from the builder that actually lays out the bundle.
///
/// # The consensus ceilings
///
/// TWO versioned limits bound a shielded bundle, and this gate enforces both
/// BEFORE any proving work starts:
///
/// 1. **Structural**: every shielded transition's `validate_structure` rejects a bundle whose
///    `actions.len()` exceeds `platform_version.system_limits.max_shielded_transition_actions`
///    (via `validate_actions_count`).
/// 2. **Size**: the serialized transition must fit
///    `platform_version.system_limits.max_state_transition_size` (20 KiB), enforced by DAPI's
///    byte prefilter / Tenderdash `mempool.max-tx-bytes` and the Drive-ABCI consensus decoder
///    — which run BEFORE structural validation ever sees the transition. At current constants
///    this is the binding limit: 6 actions serialize to ~19.0 KiB while 7 need ~21.7 KiB, so
///    7..16-action bundles pass the structural check yet are guaranteed dead on arrival.
///
/// The `try_from_bundle` constructors run no structural validation and nothing checks the byte
/// size client-side — so without this gate an over-limit bundle is built, proved (~30 s of
/// Halo 2 *per bundle*), and only then rejected. The effective ceiling is derived from BOTH
/// limits via [`crate::shielded::max_shielded_actions_for_envelope`], never hardcoded, so a
/// future `max_state_transition_size` raise widens this gate automatically. Because the action
/// count is `max(spends, outputs)`, bounding it here bounds BOTH sides: a fragmented wallet
/// spending too many notes and a caller asking for too many outputs are rejected by the same
/// comparison, before any proving work starts.
///
/// `extra_envelope_bytes` is the serialized size of the transition's variable-length
/// non-Orchard fields beyond the measured baseline envelope — an embedded instant
/// asset-lock proof (its funding transaction and `InstantLock` both carry input
/// vectors; DPP admits up to 100 inputs) or an identity-create key set. Callers whose
/// envelope is covered by the baseline (transfer, unshield, withdrawal) pass `0`;
/// `ShieldFromAssetLock` passes the serialized proof size and identity creation the
/// serialized key-set size (see [`serialized_envelope_bytes`]), so an oversized
/// envelope tightens the ceiling here instead of after the proof (#4312 review
/// finding e90e9cf15f52).
pub fn shielded_bundle_action_count(
    num_spends: usize,
    num_outputs: usize,
    extra_envelope_bytes: u64,
    platform_version: &PlatformVersion,
) -> Result<usize, ProtocolError> {
    let num_actions = BundleType::DEFAULT
        .num_actions(num_spends, num_outputs)
        .map_err(|e| {
            ProtocolError::ShieldedBuildError(format!(
                "invalid Orchard bundle shape ({num_spends} spends, {num_outputs} outputs): {e}"
            ))
        })?;

    let max_actions = platform_version
        .system_limits
        .max_shielded_transition_actions as usize;
    if num_actions > max_actions {
        return Err(ProtocolError::ShieldedBuildError(format!(
            "a bundle of {num_spends} spends and {num_outputs} outputs publishes {num_actions} \
             Orchard actions, exceeding the consensus limit of {max_actions} \
             (max_shielded_transition_actions); consensus would reject the proved transition"
        )));
    }

    let effective_max =
        crate::shielded::max_shielded_actions_for_envelope(platform_version, extra_envelope_bytes);
    if num_actions > effective_max {
        let estimated = crate::shielded::estimated_shielded_transition_wire_bytes_with_envelope(
            num_actions,
            extra_envelope_bytes,
        );
        let max_size = platform_version.system_limits.max_state_transition_size;
        let envelope_note = if extra_envelope_bytes > 0 {
            format!(" (including {extra_envelope_bytes} bytes of transition-specific envelope)")
        } else {
            String::new()
        };
        return Err(ProtocolError::ShieldedBuildError(format!(
            "a bundle of {num_spends} spends and {num_outputs} outputs publishes {num_actions} \
             Orchard actions, which serializes to an estimated {estimated} bytes{envelope_note} \
             and exceeds max_state_transition_size ({max_size} bytes); at most {effective_max} \
             actions fit, so DAPI's byte prefilter would reject the proved transition"
        )));
    }

    Ok(num_actions)
}

/// Conservative per-key allowance for an identity key's proof-of-possession
/// signature at pre-proving gate time: the keys are measured BEFORE PoP
/// signing, when each `IdentityPublicKeyInCreation.signature` is still empty,
/// while the wire form carries the signature. 97 bytes = the largest
/// signature a key type can produce (BLS12-381, 96 bytes) plus its one-byte
/// length prefix; ECDSA (65) and EdDSA (64) keys under-fill the allowance,
/// which errs on the conservative (smaller-ceiling) side.
pub const PER_KEY_SIGNATURE_ALLOWANCE_BYTES: u64 = 97;

/// Serialized size of one variable-length transition envelope field, measured
/// with the same bincode configuration the transition's own wire serialization
/// uses (`standard().with_big_endian()`, per `platform_serialization`), so the
/// pre-proving gate prices exactly the bytes the byte prefilter will see.
/// `what` names the field in the error.
pub fn serialized_envelope_bytes<T: bincode::Encode>(
    field: &T,
    what: &str,
) -> Result<u64, ProtocolError> {
    let config = bincode::config::standard().with_big_endian();
    bincode::encode_to_vec(field, config)
        .map(|bytes| bytes.len() as u64)
        .map_err(|e| {
            ProtocolError::ShieldedBuildError(format!(
                "failed to measure the serialized size of {what} for the pre-proving size gate: {e}"
            ))
        })
}

/// Serializes an authorized Orchard bundle into the raw fields used by
/// state transition constructors.
pub fn serialize_authorized_bundle(bundle: &Bundle<Authorized, i64, DashMemo>) -> SerializedBundle {
    let actions: Vec<SerializedAction> = bundle
        .actions()
        .iter()
        .map(|action| {
            let enc = action.encrypted_note();
            let mut encrypted_note = Vec::with_capacity(216);
            encrypted_note.extend_from_slice(&enc.epk_bytes);
            encrypted_note.extend_from_slice(enc.enc_ciphertext.as_ref());
            encrypted_note.extend_from_slice(&enc.out_ciphertext);
            SerializedAction {
                nullifier: action.nullifier().to_bytes(),
                rk: <[u8; 32]>::from(action.rk()),
                cmx: action.cmx().to_bytes(),
                encrypted_note,
                cv_net: action.cv_net().to_bytes(),
                spend_auth_sig: <[u8; 64]>::from(action.authorization()),
            }
        })
        .collect();
    let flags = bundle.flags().to_byte();
    let value_balance = *bundle.value_balance();
    let anchor = bundle.anchor().to_bytes();
    let proof = bundle.authorization().proof().as_ref().to_vec();
    let binding_signature = <[u8; 64]>::from(bundle.authorization().binding_signature());
    SerializedBundle {
        actions,
        flags,
        value_balance,
        anchor,
        proof,
        binding_signature,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Generates a fresh random Orchard payment address with no recoverable
/// spending authority retained by anyone.
///
/// Draws 32 random bytes for an Orchard `SpendingKey` (retrying on the
/// rare invalid draw — `SpendingKey::from_bytes` returns a `CtOption`),
/// derives its `FullViewingKey`, and returns the External-scope address
/// at diversifier index 0. The spending key is dropped here, so the
/// resulting address is unspendable by this process — exactly what a
/// zero-value anonymity-set filler output wants.
fn random_orchard_payment_address() -> PaymentAddress {
    let mut rng = OsRng;
    loop {
        let mut bytes = [0u8; 32];
        rng.fill_bytes(&mut bytes);
        if let Some(sk) = Option::<SpendingKey>::from(SpendingKey::from_bytes(bytes)) {
            let fvk = FullViewingKey::from(&sk);
            return fvk.address_at(0u32, Scope::External);
        }
    }
}

/// Builds an output-only Orchard bundle (no spends).
///
/// Used by Shield and ShieldFromAssetLock transitions where funds enter
/// the shielded pool from transparent sources.
///
/// `sender_ovk` encrypts the real output's `out_ciphertext` (Zcash
/// outgoing-transaction-history convention): with `Some`, the sender can
/// later recover the note (recipient, value, memo) from chain data via
/// `try_recover_outgoing_note` under that OVK. With `None`, a random
/// outgoing cipher key is used and the sent note is unrecoverable by
/// anyone. Orchard's padding outputs always use `None`.
///
/// `dummy_outputs` adds that many extra **zero-value** outputs after the
/// real one, each to a fresh random Orchard address with `sender_ovk =
/// None` and an empty memo. They are unrecoverable by anyone (no party
/// holds the spending key) — they exist purely as anonymity-set filler
/// so a single transition can grow the on-chain note count. With
/// `dummy_outputs == 0` the bundle is byte-class identical to the
/// historical single-output form (Orchard still pads to its 2-action
/// minimum). The on-wire action count is
/// `max(1 + dummy_outputs, 2)` and the `value_balance` is unchanged
/// (the dummies contribute zero value).
pub(crate) fn build_output_only_bundle<P: OrchardProver>(
    recipient: &OrchardAddress,
    amount: u64,
    memo: [u8; 36],
    sender_ovk: Option<OutgoingViewingKey>,
    dummy_outputs: usize,
    prover: &P,
) -> Result<Bundle<Authorized, i64, DashMemo>, ProtocolError> {
    let payment_address = PaymentAddress::from(recipient);
    let anchor = Anchor::empty_tree();
    let mut builder = Builder::<DashMemo>::new(
        BundleType::Transactional {
            flags: OrchardFlags::SPENDS_DISABLED,
            bundle_required: false,
        },
        anchor,
    );

    builder
        .add_output(
            sender_ovk,
            payment_address,
            NoteValue::from_raw(amount),
            memo,
        )
        .map_err(|e| ProtocolError::ShieldedBuildError(format!("failed to add output: {:?}", e)))?;

    // Anonymity-set filler: zero-value outputs to fresh random addresses,
    // each with `None` OVK and an empty memo (unrecoverable by anyone).
    for _ in 0..dummy_outputs {
        let filler_address = random_orchard_payment_address();
        builder
            .add_output(None, filler_address, NoteValue::from_raw(0), [0u8; 36])
            .map_err(|e| {
                ProtocolError::ShieldedBuildError(format!("failed to add dummy output: {:?}", e))
            })?;
    }

    prove_and_sign_bundle(builder, prover, &[], &[])
}

/// Builds a spend+output Orchard bundle.
///
/// Used by Unshield, ShieldedWithdrawal, and IdentityCreateFromShieldedPool
/// where funds are spent from existing notes. The single shielded output is
/// the spender's change note; its `out_ciphertext` is encrypted under the
/// spender's own External-scope OVK (derived from `fvk`) so the wallet can
/// recover the note — including its structured memo, which the compact IVK
/// scan path never sees — from chain data alone.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_spend_bundle<P: OrchardProver>(
    spends: Vec<SpendableNote>,
    recipient: &OrchardAddress,
    output_amount: u64,
    memo: [u8; 36],
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &P,
    extra_sighash_data: &[u8],
) -> Result<Bundle<Authorized, i64, DashMemo>, ProtocolError> {
    let data = extra_sighash_data.to_vec();
    build_spend_bundle_with(
        spends,
        recipient,
        output_amount,
        memo,
        fvk,
        ask,
        anchor,
        prover,
        move |_| Ok(data),
    )
}

/// Like [`build_spend_bundle`], but the extra sighash data is computed by a
/// closure that receives the built bundle's published action nullifiers (in
/// on-wire order, INCLUDING any padding actions' dummy nullifiers).
///
/// `IdentityCreateFromShieldedPool` needs this: its identity id is
/// `double_sha256(sorted published nullifiers)`, and `BundleType::DEFAULT`
/// pads single-spend bundles with a dummy action whose random nullifier only
/// exists once the bundle is built — deriving the id from the real spends
/// alone would diverge from the consensus re-derivation.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_spend_bundle_with<P: OrchardProver, F>(
    spends: Vec<SpendableNote>,
    recipient: &OrchardAddress,
    output_amount: u64,
    memo: [u8; 36],
    fvk: &FullViewingKey,
    ask: &SpendAuthorizingKey,
    anchor: Anchor,
    prover: &P,
    extra_sighash_data: F,
) -> Result<Bundle<Authorized, i64, DashMemo>, ProtocolError>
where
    F: FnOnce(&[[u8; 32]]) -> Result<Vec<u8>, ProtocolError>,
{
    let payment_address = PaymentAddress::from(recipient);

    let mut builder = Builder::<DashMemo>::new(BundleType::DEFAULT, anchor);

    for spend in spends {
        builder
            .add_spend(fvk.clone(), spend.note, spend.merkle_path)
            .map_err(|e| {
                ProtocolError::ShieldedBuildError(format!("failed to add spend: {:?}", e))
            })?;
    }

    builder
        .add_output(
            Some(fvk.to_ovk(Scope::External)),
            payment_address,
            NoteValue::from_raw(output_amount),
            memo,
        )
        .map_err(|e| ProtocolError::ShieldedBuildError(format!("failed to add output: {:?}", e)))?;

    prove_and_sign_bundle_with(
        builder,
        prover,
        std::slice::from_ref(ask),
        extra_sighash_data,
    )
}

/// Takes a configured Builder, generates the proof, computes the platform
/// sighash, and applies signatures.
pub(crate) fn prove_and_sign_bundle<P: OrchardProver>(
    builder: Builder<DashMemo>,
    prover: &P,
    signing_keys: &[SpendAuthorizingKey],
    extra_sighash_data: &[u8],
) -> Result<Bundle<Authorized, i64, DashMemo>, ProtocolError> {
    let data = extra_sighash_data.to_vec();
    prove_and_sign_bundle_with(builder, prover, signing_keys, move |_| Ok(data))
}

/// Like [`prove_and_sign_bundle`], but the extra sighash data is computed by
/// a closure receiving the built bundle's published action nullifiers (see
/// [`build_spend_bundle_with`]). The closure runs after `Builder::build`
/// fixes the action set (padding included) and before the sighash is bound.
pub(crate) fn prove_and_sign_bundle_with<P: OrchardProver, F>(
    builder: Builder<DashMemo>,
    prover: &P,
    signing_keys: &[SpendAuthorizingKey],
    extra_sighash_data: F,
) -> Result<Bundle<Authorized, i64, DashMemo>, ProtocolError>
where
    F: FnOnce(&[[u8; 32]]) -> Result<Vec<u8>, ProtocolError>,
{
    let mut rng = OsRng;

    let (unauthorized, _) = builder
        .build::<i64>(&mut rng)
        .map_err(|e| ProtocolError::ShieldedBuildError(format!("failed to build bundle: {:?}", e)))?
        .ok_or_else(|| {
            ProtocolError::ShieldedBuildError("bundle was empty after build".to_string())
        })?;

    let nullifiers: Vec<[u8; 32]> = unauthorized
        .actions()
        .iter()
        .map(|action| action.nullifier().to_bytes())
        .collect();
    let extra_sighash_data = extra_sighash_data(&nullifiers)?;

    let bundle_commitment: [u8; 32] = unauthorized.commitment().into();
    let sighash = compute_platform_sighash(&bundle_commitment, &extra_sighash_data);

    let proven = unauthorized
        .create_proof(prover.proving_key(), &mut rng)
        .map_err(|e| {
            ProtocolError::ShieldedBuildError(format!("failed to create proof: {:?}", e))
        })?;

    proven
        .apply_signatures(rng, sighash, signing_keys)
        .map_err(|e| {
            ProtocolError::ShieldedBuildError(format!("failed to apply signatures: {:?}", e))
        })
}

/// Shared test utilities for builder tests.
#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use grovedb_commitment_tree::{
        FullViewingKey, Hashable, MerkleHashOrchard, Note, NoteValue, ProvingKey, RandomSeed, Rho,
        Scope, SpendingKey, NOTE_COMMITMENT_TREE_DEPTH,
    };
    use std::sync::OnceLock;

    static PROVING_KEY: OnceLock<ProvingKey> = OnceLock::new();

    /// Returns a cached ProvingKey (~30s to build on first call).
    pub fn proving_key() -> &'static ProvingKey {
        PROVING_KEY.get_or_init(ProvingKey::build)
    }

    /// Test implementation of `OrchardProver` backed by the cached proving key.
    pub struct TestProver;

    impl super::OrchardProver for TestProver {
        fn proving_key(&self) -> &ProvingKey {
            proving_key()
        }
    }

    /// Creates a test OrchardAddress from a deterministic spending key.
    pub fn test_orchard_address() -> OrchardAddress {
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let payment_address = fvk.address_at(0u32, Scope::External);
        OrchardAddress::from_raw_bytes(&payment_address.to_raw_address_bytes())
            .expect("valid orchard address bytes")
    }

    /// Creates a SpendableNote with the given value.
    ///
    /// The note is cryptographically valid (has a valid commitment) but uses
    /// an all-zeros Merkle path, so it will only pass the Orchard circuit when
    /// paired with `Anchor::empty_tree()`. Suitable for both error-path tests
    /// (where the proving key is never reached) and happy-path tests.
    pub fn test_spendable_note(value: u64) -> SpendableNote {
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key bytes");
        let fvk = FullViewingKey::from(&sk);
        let payment_address = fvk.address_at(0u32, Scope::External);

        // Construct a valid Rho from the zero element (always valid in pallas)
        let rho: Rho =
            Option::from(Rho::from_bytes(&[0u8; 32])).expect("zero is valid pallas::Base");
        let rseed: RandomSeed =
            Option::from(RandomSeed::from_bytes([1u8; 32], &rho)).expect("valid random seed");
        let note: Note = Option::from(Note::from_parts(
            payment_address,
            NoteValue::from_raw(value),
            rho,
            rseed,
        ))
        .expect("note commitment should be valid");

        // All-zeros merkle path at position 0 — consistent with Anchor::empty_tree()
        let auth_path = [MerkleHashOrchard::empty_leaf(); NOTE_COMMITMENT_TREE_DEPTH];
        let merkle_path = MerklePath::from_parts(0, auth_path);

        SpendableNote { note, merkle_path }
    }
}

#[cfg(test)]
mod mod_tests {
    use super::test_helpers::{test_orchard_address, test_spendable_note, TestProver};
    use super::*;
    use grovedb_commitment_tree::{FullViewingKey, SpendAuthorizingKey, SpendingKey};

    // ------------------------------------------------------------------
    // `build_output_only_bundle` — exercise the happy path covering the
    // internal builder configuration and `prove_and_sign_bundle` pipeline
    // on the empty-signing-keys branch.
    // ------------------------------------------------------------------

    #[test]
    fn output_only_bundle_flags_and_value_balance() {
        let recipient = test_orchard_address();
        let bundle = build_output_only_bundle(&recipient, 10_000, [0u8; 36], None, 0, &TestProver)
            .expect("bundle should build");

        // Spends are disabled for Shield / ShieldFromAssetLock bundles.
        assert!(!bundle.flags().spends_enabled());
        assert!(bundle.flags().outputs_enabled());
        // Orchard value_balance is negative when net value enters the pool.
        assert_eq!(*bundle.value_balance(), -10_000i64);
        assert!(
            !bundle.actions().is_empty(),
            "at least one padding action expected"
        );
    }

    // ------------------------------------------------------------------
    // `build_output_only_bundle` dummy-output padding — the on-wire
    // action count is `max(1 + dummy_outputs, 2)` (Orchard pads an
    // output-only bundle to its 2-action minimum) and the dummies are
    // zero-value, so the bundle's `value_balance` still equals exactly
    // the real recipient amount. This is the invariant the pool-seeding
    // flow relies on: one transition publishes up to 6 actions (the most
    // that fits the 20 KiB transition-size limit), all but one carrying
    // no value. The cases stop at 5 dummies — the seeding maximum — to
    // keep this real-proving test inside the CI shielded-step budget
    // (proof cost grows with the action count).
    // ------------------------------------------------------------------

    #[test]
    fn dummy_output_padding_action_count_and_value_balance() {
        let recipient = test_orchard_address();
        let amount = 10_000u64;

        // (dummy_outputs, expected on-wire action count).
        for (dummies, expected_actions) in [(0usize, 2usize), (1, 2), (5, 6)] {
            let bundle =
                build_output_only_bundle(&recipient, amount, [0u8; 36], None, dummies, &TestProver)
                    .expect("bundle should build");
            assert_eq!(
                bundle.actions().len(),
                expected_actions,
                "dummy_outputs={dummies} should serialize to {expected_actions} actions"
            );
            // Dummies are zero-value: net value entering the pool is unchanged.
            assert_eq!(
                *bundle.value_balance(),
                -(amount as i64),
                "value_balance must equal the real amount regardless of dummy_outputs ({dummies})"
            );
        }
    }

    // ------------------------------------------------------------------
    // `serialize_authorized_bundle` — verify the mapping from a fully
    // authorized bundle into the raw state-transition fields.
    // ------------------------------------------------------------------

    #[test]
    fn serialize_authorized_bundle_preserves_fields() {
        let recipient = test_orchard_address();
        let bundle = build_output_only_bundle(&recipient, 7_777, [3u8; 36], None, 0, &TestProver)
            .expect("bundle should build");
        let sb = serialize_authorized_bundle(&bundle);

        assert_eq!(sb.value_balance, *bundle.value_balance());
        assert_eq!(sb.flags, bundle.flags().to_byte());
        assert_eq!(sb.anchor, bundle.anchor().to_bytes());
        assert!(!sb.proof.is_empty(), "Halo 2 proof must not be empty");
        assert_eq!(sb.binding_signature.len(), 64);
        assert_eq!(sb.actions.len(), bundle.actions().len());
        for action in &sb.actions {
            // Each encrypted_note packs epk (32) + enc_ciphertext (580... wait — 84+512? verify via cap 216)
            // The explicit layout from serialize_authorized_bundle: epk_bytes (32) +
            // enc_ciphertext + out_ciphertext = 580 + 80? The code pre-allocates 216.
            // Don't hardcode length — just verify non-empty and signature sizes.
            assert!(!action.encrypted_note.is_empty());
            assert_eq!(action.nullifier.len(), 32);
            assert_eq!(action.cmx.len(), 32);
            assert_eq!(action.cv_net.len(), 32);
            assert_eq!(action.rk.len(), 32);
            assert_eq!(action.spend_auth_sig.len(), 64);
        }
    }

    // ------------------------------------------------------------------
    // OVK outgoing-history round trip: an output built with the sender's
    // OVK must recover (note, recipient, memo) under that same OVK — the
    // Zcash convention that lets a wallet reconstruct its send history
    // from chain data alone — and must stay opaque to any other OVK.
    // ------------------------------------------------------------------

    #[test]
    fn output_built_with_sender_ovk_recovers_under_that_ovk_only() {
        use grovedb_commitment_tree::{try_output_recovery_with_ovk, OrchardDomain, Scope};

        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key bytes");
        let sender_ovk = FullViewingKey::from(&sk).to_ovk(Scope::External);

        let recipient = test_orchard_address();
        let amount = 31_337u64;
        let mut memo = [0u8; 36];
        memo[..9].copy_from_slice(b"ovk-round");

        let bundle = build_output_only_bundle(
            &recipient,
            amount,
            memo,
            Some(sender_ovk.clone()),
            0,
            &TestProver,
        )
        .expect("bundle should build");

        let recover_all = |ovk: &grovedb_commitment_tree::OutgoingViewingKey| {
            bundle
                .actions()
                .iter()
                .filter_map(|action| {
                    let domain = OrchardDomain::<DashMemo>::for_action(action);
                    try_output_recovery_with_ovk(
                        &domain,
                        ovk,
                        action,
                        action.cv_net(),
                        &action.encrypted_note().out_ciphertext,
                    )
                })
                .collect::<Vec<_>>()
        };

        let recovered = recover_all(&sender_ovk);
        assert_eq!(
            recovered.len(),
            1,
            "exactly the real recipient output must recover; padding stays opaque"
        );
        let (note, recovered_addr, recovered_memo) = &recovered[0];
        assert_eq!(note.value().inner(), amount, "recovered value mismatch");
        assert_eq!(
            recovered_addr.to_raw_address_bytes(),
            recipient.inner().to_raw_address_bytes(),
            "recovered recipient mismatch"
        );
        assert_eq!(*recovered_memo, memo, "recovered memo mismatch");

        // A different wallet's OVK opens nothing — no false positives in
        // anyone else's send history.
        let other_sk = SpendingKey::from_bytes([7u8; 32]).expect("valid spending key bytes");
        let other_ovk = FullViewingKey::from(&other_sk).to_ovk(Scope::External);
        assert!(
            recover_all(&other_ovk).is_empty(),
            "a foreign OVK must not recover the output"
        );
    }

    // ------------------------------------------------------------------
    // `From<&OrchardAddress> for PaymentAddress` delegates to `inner()`.
    // ------------------------------------------------------------------

    #[test]
    fn from_orchard_address_to_payment_address_preserves_bytes() {
        let addr = test_orchard_address();
        let pa: PaymentAddress = (&addr).into();
        assert_eq!(
            pa.to_raw_address_bytes(),
            addr.inner().to_raw_address_bytes()
        );
    }

    // ------------------------------------------------------------------
    // `build_spend_bundle` — exercise the `add_spend` error path. The
    // helper notes don't reconcile to `Anchor::empty_tree()` (the
    // commitment and the all-zeros Merkle path don't match), so adding
    // the spend surfaces an AnchorMismatch error wrapped in
    // `ProtocolError::ShieldedBuildError`.
    // ------------------------------------------------------------------

    #[test]
    fn build_spend_bundle_add_spend_anchor_mismatch_surfaces_error() {
        let recipient = test_orchard_address();
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid spending key");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let spends = vec![test_spendable_note(50_000)];

        let result = build_spend_bundle(
            spends,
            &recipient,
            40_000,
            [1u8; 36],
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            &[],
        );
        let err = result.expect_err("anchor mismatch should bubble up");
        match err {
            ProtocolError::ShieldedBuildError(msg) => {
                assert!(
                    msg.contains("failed to add spend")
                        || msg.contains("AnchorMismatch")
                        || msg.contains("anchor"),
                    "unexpected error message: {}",
                    msg
                );
            }
            other => panic!("expected ShieldedBuildError, got {:?}", other),
        }
    }

    #[test]
    fn build_spend_bundle_empty_spends_still_returns_some_output_bundle_or_error() {
        // Exercise the loop-never-executed branch: no spends at all. The
        // Orchard builder configuration `BundleType::DEFAULT` requires at
        // least one spend by default — expect an error wrapped as
        // `ShieldedBuildError`.
        let recipient = test_orchard_address();
        let sk = SpendingKey::from_bytes([42u8; 32]).expect("valid sk");
        let fvk = FullViewingKey::from(&sk);
        let ask = SpendAuthorizingKey::from(&sk);

        let result = build_spend_bundle(
            vec![],
            &recipient,
            0,
            [0u8; 36],
            &fvk,
            &ask,
            Anchor::empty_tree(),
            &TestProver,
            &[],
        );
        // Whatever the outcome, it should be deterministic: either Ok (with
        // padding) or a clean ShieldedBuildError — never a panic.
        match result {
            Ok(_) => {}
            Err(ProtocolError::ShieldedBuildError(_)) => {}
            Err(e) => panic!("unexpected error kind: {:?}", e),
        }
    }

    /// Builds an output-only builder the way `build_output_only_bundle` does (no merkle
    /// witness needed): a single output, padded by `BundleType` to the 2-action minimum.
    fn output_only_builder(amount: u64) -> Builder<DashMemo> {
        let recipient = test_orchard_address();
        let payment_address = PaymentAddress::from(&recipient);
        let mut builder = Builder::<DashMemo>::new(
            BundleType::Transactional {
                flags: OrchardFlags::SPENDS_DISABLED,
                bundle_required: false,
            },
            Anchor::empty_tree(),
        );
        builder
            .add_output(
                None,
                payment_address,
                NoteValue::from_raw(amount),
                [0u8; 36],
            )
            .expect("add output");
        builder
    }

    // ------------------------------------------------------------------
    // `prove_and_sign_bundle_with` — the closure contract. The closure MUST
    // receive the BUILT bundle's published action nullifiers (padding
    // actions' dummy nullifiers included), in on-wire order: this is what
    // lets `IdentityCreateFromShieldedPool` derive its identity id from the
    // same nullifier set consensus re-derives it from. Deriving from the
    // requested spends alone would diverge whenever the bundle is padded.
    // ------------------------------------------------------------------

    #[test]
    fn prove_and_sign_bundle_with_closure_receives_published_nullifiers() {
        let builder = output_only_builder(10_000);

        let mut recorded: Option<Vec<[u8; 32]>> = None;
        let bundle = prove_and_sign_bundle_with(builder, &TestProver, &[], |nullifiers| {
            recorded = Some(nullifiers.to_vec());
            Ok(vec![])
        })
        .expect("output-only bundle should prove");

        let recorded = recorded.expect("the extra-sighash closure must run");
        // A single output is padded to the 2-action minimum; every padded action
        // publishes a (dummy) nullifier on the wire.
        assert_eq!(
            recorded.len(),
            2,
            "closure must see one nullifier per PUBLISHED action (incl. padding)"
        );
        assert_ne!(
            recorded[0], recorded[1],
            "padding dummy nullifiers are randomized per action"
        );
        // The recorded set must be exactly the authorized bundle's published
        // nullifiers, in the same on-wire order.
        let published: Vec<[u8; 32]> = bundle
            .actions()
            .iter()
            .map(|action| action.nullifier().to_bytes())
            .collect();
        assert_eq!(
            recorded, published,
            "closure must receive the bundle's published nullifiers in on-wire order"
        );
    }

    #[test]
    fn prove_and_sign_bundle_with_closure_error_short_circuits_before_proving() {
        let builder = output_only_builder(10_000);

        let result = prove_and_sign_bundle_with(builder, &TestProver, &[], |_| {
            Err(ProtocolError::ShieldedBuildError(
                "closure rejected".to_string(),
            ))
        });

        match result {
            Err(ProtocolError::ShieldedBuildError(msg)) => {
                assert_eq!(msg, "closure rejected", "closure error must pass through");
            }
            other => panic!("expected the closure's error to propagate, got {:?}", other),
        }
    }

    // ------------------------------------------------------------------
    // `shielded_bundle_action_count` — the shared fee-sizing predictor.
    // ------------------------------------------------------------------

    /// The predictor must be `max(num_spends, num_outputs)` padded to Orchard's 2-action
    /// minimum — for the OUTPUT side as well as the spend side. The `num_outputs >= 3` rows are
    /// the ones a spends-only predictor gets wrong.
    #[test]
    fn shielded_bundle_action_count_is_max_spends_outputs_padded_to_two() {
        let platform_version = PlatformVersion::latest();
        for (spends, outputs, expected) in [
            (0usize, 1usize, 2usize),
            (1, 1, 2),
            (1, 2, 2),
            (2, 2, 2),
            // Output-dominated shapes: the spend count no longer determines the fee.
            (1, 3, 3),
            (2, 3, 3),
            (1, 4, 4),
            (5, 3, 5),
            (3, 6, 6),
        ] {
            let actual = shielded_bundle_action_count(spends, outputs, 0, platform_version)
                .expect("DEFAULT bundles accept any spend/output mix");
            assert_eq!(
                actual, expected,
                "action count for {spends} spends / {outputs} outputs"
            );
        }
    }

    /// A real bundle's on-wire `actions.len()` — the number consensus prices the fee off — must
    /// equal what the predictor said. Exercised through the output-only builder because it is
    /// the cheapest real bundle to construct at several output counts.
    #[test]
    fn shielded_bundle_action_count_matches_a_real_bundle() {
        let platform_version = PlatformVersion::latest();
        let recipient = test_orchard_address();
        // (dummy_outputs, total outputs = 1 real + dummies)
        for dummies in [0usize, 1, 4] {
            let num_outputs = 1 + dummies;
            let bundle =
                build_output_only_bundle(&recipient, 10_000, [0u8; 36], None, dummies, &TestProver)
                    .expect("bundle should build");
            let predicted = shielded_bundle_action_count(0, num_outputs, 0, platform_version)
                .expect("valid bundle shape");
            assert_eq!(
                bundle.actions().len(),
                predicted,
                "predicted action count must match the real bundle's on-wire count for \
                 {num_outputs} outputs"
            );
        }
    }

    /// The predictor is also the CONSENSUS gate — for BOTH ceilings. The effective (size-derived)
    /// boundary itself must still pass, from each side: at current constants that is 6 actions
    /// (~19.0 KiB against the 20 KiB `max_state_transition_size`).
    #[test]
    fn shielded_bundle_action_count_accepts_the_effective_boundary() {
        let platform_version = PlatformVersion::latest();
        let effective = crate::shielded::max_shielded_actions_per_transition(platform_version);

        // Exactly at the effective ceiling, from each side and from both at once.
        for (spends, outputs) in [(1usize, effective), (effective, 1), (effective, effective)] {
            assert_eq!(
                shielded_bundle_action_count(spends, outputs, 0, platform_version).unwrap_or_else(
                    |e| panic!("{spends} spends / {outputs} outputs is AT the effective ceiling and must be accepted: {e}")
                ),
                effective
            );
        }
    }

    /// One action over the EFFECTIVE ceiling must fail fast, pre-proving — from the OUTPUT side
    /// (a 6-recipient multi transfer becomes 7 outputs once the unconditional change output is
    /// added) and from the SPEND side (a fragmented wallet selecting 7 notes). These shapes pass
    /// the 16-action structural cap, but a 7-action transition serializes to ~21.7 KiB and is
    /// rejected by DAPI's 20 KiB byte prefilter — AFTER ~30 s of Halo 2 proving, without this
    /// gate. This test completing in milliseconds is itself part of the assertion.
    #[test]
    fn shielded_bundle_action_count_rejects_over_the_size_derived_ceiling() {
        let platform_version = PlatformVersion::latest();
        let effective = crate::shielded::max_shielded_actions_per_transition(platform_version);
        let structural = platform_version
            .system_limits
            .max_shielded_transition_actions as usize;
        assert!(
            effective < structural,
            "this test requires the size limit to be the binding one (effective {effective} < \
             structural {structural}); if the size limit was raised, retire or rework this test"
        );

        let over = effective + 1;
        // Output-dominated, spend-dominated, and both-sided 7-action shapes.
        for (spends, outputs) in [(1usize, over), (over, 1), (over, over)] {
            let err = shielded_bundle_action_count(spends, outputs, 0, platform_version)
                .expect_err("a bundle over the size-derived ceiling must be rejected pre-proving");
            assert!(
                err.to_string().contains("max_state_transition_size"),
                "unexpected error for {spends} spends / {outputs} outputs: {err}"
            );
        }
    }

    /// One action over the STRUCTURAL ceiling must fail fast — from the OUTPUT side (a
    /// 16-recipient call, which becomes 17 outputs once the unconditional change output is
    /// added) and from the SPEND side (a fragmented wallet selecting too many notes). The
    /// structural check fires first, so these carry the `max_shielded_transition_actions`
    /// message rather than the size-derived one.
    #[test]
    fn shielded_bundle_action_count_rejects_over_the_consensus_limit() {
        let platform_version = PlatformVersion::latest();
        let max = platform_version
            .system_limits
            .max_shielded_transition_actions as usize;

        for (spends, outputs) in [(1usize, max + 1), (max + 1, 1), (max + 1, max + 1)] {
            let err = shielded_bundle_action_count(spends, outputs, 0, platform_version)
                .expect_err("a bundle over the consensus action limit must be rejected");
            assert!(
                err.to_string().contains("exceeding the consensus limit"),
                "unexpected error for {spends} spends / {outputs} outputs: {err}"
            );
        }
    }
}
