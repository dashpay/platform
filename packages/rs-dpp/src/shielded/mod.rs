#[cfg(feature = "shielded-client")]
pub mod builder;

mod compute_minimum_shielded_fee;
pub mod memo;
mod sighash;

#[cfg(all(test, feature = "shielded-client"))]
mod wire_cost_measured_tests;

pub use memo::{ShieldedMemo, MEMO_PAYLOAD_SIZE, MEMO_SIZE};

use bincode::{Decode, Encode};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

// Re-exported so the public path stays `dpp::shielded::compute_minimum_shielded_fee` (the
// module and the function share a name but live in different namespaces).
pub use compute_minimum_shielded_fee::{
    compute_minimum_shielded_fee, compute_shielded_identity_create_fee,
    compute_shielded_unshield_fee, compute_shielded_verification_fee,
    compute_shielded_withdrawal_fee,
};

// Re-exported so the public paths stay `dpp::shielded::<name>` after moving the sighash preimage
// builders into their own file. Both the version-dispatching wrappers and their `_v0` impls are
// re-exported (callers use the wrappers; byte-layout tests use the `_v0` impls).
pub use sighash::{
    compute_platform_sighash, identity_create_from_shielded_extra_sighash_data,
    identity_create_from_shielded_extra_sighash_data_v0, shielded_withdrawal_extra_sighash_data,
    shielded_withdrawal_extra_sighash_data_v0, unshield_extra_sighash_data,
    unshield_extra_sighash_data_v0,
};

/// On-wire serialized size of one [`SerializedAction`]: 408 bytes.
///
/// `nullifier` (32) + `rk` (32) + `cmx` (32) + `encrypted_note` (216) +
/// `cv_net` (32) + `spend_auth_sig` (64). This is the per-action cost in the
/// transition's `actions` vector, EXCLUDING the Halo 2 proof's per-action
/// growth (see [`SHIELDED_PROOF_WIRE_BYTES_PER_ACTION`]).
pub(crate) const SHIELDED_ACTION_WIRE_BYTES: u64 = 408;

/// On-wire growth of the Halo 2 proof per additional Orchard action: 2,273 bytes.
///
/// The proof over the Orchard circuit grows linearly with the number of action
/// instances. Measured on real proved transitions (see the
/// `seed_pool_batch_fits_max_state_transition_size` signing test in
/// `shield_from_asset_lock_transition/signing_tests.rs`): 2 actions → 8,294 B
/// total, 6 → 19,018 B, 7 → 21,699 B — an exactly linear 2,681 B/action, of
/// which 408 B is the serialized action ([`SHIELDED_ACTION_WIRE_BYTES`]) and
/// 2,273 B is proof growth. Pinned by
/// `shielded_wire_cost_model_matches_measured_transitions` below.
pub(crate) const SHIELDED_PROOF_WIRE_BYTES_PER_ACTION: u64 = 2_273;

/// Fixed on-wire envelope overhead of a shielded state transition: 2,932 bytes.
///
/// Everything that does not scale with the action count: the transition's
/// non-action fields (anchor, value balance, flags, signatures, asset-lock
/// proof / identity keys where present) plus the proof's fixed portion.
/// Derived from the same measured points as
/// [`SHIELDED_PROOF_WIRE_BYTES_PER_ACTION`] (8,294 − 2 × 2,681 = 2,932,
/// consistent across the 2-, 6- and 7-action measurements of a
/// `ShieldFromAssetLock` with a chain asset-lock proof). This is the BASELINE
/// envelope: transition types whose non-Orchard fields have VARIABLE
/// serialized size — an instant asset-lock proof embedding its funding
/// transaction and `InstantLock` (both carry input vectors; DPP admits
/// asset-lock transactions with up to 100 inputs), or an identity-create key
/// set of up to six keys — must account for those bytes on top of this
/// constant via the `extra_envelope_bytes` argument of
/// [`max_shielded_actions_for_envelope`] /
/// [`estimated_shielded_transition_wire_bytes_with_envelope`], so the
/// pre-proving gate sees the size the byte prefilter will see. DAPI's byte
/// prefilter remains the authoritative gate.
pub(crate) const SHIELDED_TRANSITION_WIRE_OVERHEAD_BYTES: u64 = 2_932;

/// Encoded size of the CHAIN asset-lock proof that is already counted inside
/// [`SHIELDED_TRANSITION_WIRE_OVERHEAD_BYTES`]: 40 bytes.
///
/// The baseline envelope was measured from complete `ShieldFromAssetLock`
/// transitions that each carried a chain asset-lock proof, so those bytes are
/// part of the 2,932. A `ShieldFromAssetLock` that supplies its own proof
/// REPLACES that field rather than adding to it — passing the full serialized
/// proof as `extra_envelope_bytes` would model `baseline + full proof` and
/// double-count this much. Callers therefore price the DELTA
/// (`serialized proof − this constant`, saturating), which is the only part
/// that actually grows the transition beyond the measured baseline.
///
/// Subtracting is safe in both directions: a chain proof is at or near this
/// size so the delta floors at 0 and keeps the baseline ceiling, while an
/// instant proof's several-KiB delta still tightens the ceiling. The
/// remaining error is bounded by the few bytes a chain proof's varint fields
/// vary by (`core_chain_locked_height` / `vout` magnitude), which is far below
/// the ~2.7 KiB granularity of one action.
///
/// Pinned to the calibration proof by
/// `baseline_asset_lock_proof_bytes_matches_the_calibration_proof` (#4312
/// review finding b6f78dd76eb7).
pub(crate) const SHIELDED_BASELINE_ASSET_LOCK_PROOF_BYTES: u64 = 40;

/// Conservative estimate of a shielded transition's on-wire serialized size
/// for a bundle of `num_actions` Orchard actions with the baseline envelope.
///
/// `SHIELDED_TRANSITION_WIRE_OVERHEAD_BYTES + num_actions ×
/// (SHIELDED_ACTION_WIRE_BYTES + SHIELDED_PROOF_WIRE_BYTES_PER_ACTION)` —
/// the linear model pinned against measured proved transitions (see
/// [`SHIELDED_PROOF_WIRE_BYTES_PER_ACTION`]). Transitions with
/// variable-size non-Orchard fields use
/// [`estimated_shielded_transition_wire_bytes_with_envelope`].
pub(crate) fn estimated_shielded_transition_wire_bytes(num_actions: usize) -> u64 {
    estimated_shielded_transition_wire_bytes_with_envelope(num_actions, 0)
}

/// [`estimated_shielded_transition_wire_bytes`] plus `extra_envelope_bytes`
/// of transition-specific envelope beyond the measured baseline — the
/// serialized size of the transition's variable-length non-Orchard fields
/// (an embedded instant asset-lock proof, an identity key set), which the
/// fixed [`SHIELDED_TRANSITION_WIRE_OVERHEAD_BYTES`] does not cover.
pub(crate) fn estimated_shielded_transition_wire_bytes_with_envelope(
    num_actions: usize,
    extra_envelope_bytes: u64,
) -> u64 {
    SHIELDED_TRANSITION_WIRE_OVERHEAD_BYTES
        .saturating_add(extra_envelope_bytes)
        .saturating_add(
            (num_actions as u64)
                .saturating_mul(SHIELDED_ACTION_WIRE_BYTES + SHIELDED_PROOF_WIRE_BYTES_PER_ACTION),
        )
}

/// The EFFECTIVE per-transition Orchard action ceiling under `platform_version`:
/// the largest action count that satisfies BOTH versioned limits.
///
/// Two independent consensus limits bound a shielded bundle:
///
/// 1. the structural cap `system_limits.max_shielded_transition_actions`
///    (enforced by every shielded `validate_structure`), and
/// 2. the byte cap `system_limits.max_state_transition_size` (enforced by
///    DAPI's byte prefilter / Tenderdash `mempool.max-tx-bytes` and the
///    Drive-ABCI consensus decoder BEFORE structural validation runs).
///
/// Because the on-wire size grows ~2,681 B per action on a ~2.9 KiB envelope
/// (see `estimated_shielded_transition_wire_bytes`), the byte cap is the
/// binding constraint at current constants: 6 actions serialize to ~19.0 KiB
/// while 7 need ~21.7 KiB against the 20 KiB limit — so the structural cap of
/// 16 is unreachable unless `max_state_transition_size` is raised. Builders
/// MUST gate on this derived ceiling before proving (via
/// `shielded_bundle_action_count`); otherwise a 7..16-action bundle passes the
/// structural check, burns the expensive Halo 2 proof, and is only then
/// rejected by the byte prefilter.
///
/// This is the ceiling for the BASELINE envelope. Transition types with
/// variable-size non-Orchard fields (instant asset-lock proofs, identity key
/// sets) must use `max_shielded_actions_for_envelope` with their measured
/// extra bytes — a large enough envelope tightens the ceiling below 6.
pub fn max_shielded_actions_per_transition(
    platform_version: &platform_version::version::PlatformVersion,
) -> usize {
    max_shielded_actions_for_envelope(platform_version, 0)
}

/// [`max_shielded_actions_per_transition`], with the size budget reduced by
/// `extra_envelope_bytes` of transition-specific envelope beyond the measured
/// baseline (see [`SHIELDED_TRANSITION_WIRE_OVERHEAD_BYTES`]).
///
/// An instant asset-lock proof embeds its funding transaction and
/// `InstantLock` — both carry input vectors, and DPP admits asset-lock
/// transactions with up to 100 inputs — and an identity create carries up to
/// six variable public keys; either can consume the ~1.4 KiB of slack the
/// baseline ceiling leaves under the byte cap, so the pre-proving gate must
/// price them in or a bundle passes the gate, burns the Halo 2 proof, and is
/// only then rejected by DAPI's byte prefilter (#4312 review finding
/// e90e9cf15f52).
pub(crate) fn max_shielded_actions_for_envelope(
    platform_version: &platform_version::version::PlatformVersion,
    extra_envelope_bytes: u64,
) -> usize {
    let structural = platform_version
        .system_limits
        .max_shielded_transition_actions as usize;
    let per_action = SHIELDED_ACTION_WIRE_BYTES + SHIELDED_PROOF_WIRE_BYTES_PER_ACTION;
    let size_budget = platform_version
        .system_limits
        .max_state_transition_size
        .saturating_sub(SHIELDED_TRANSITION_WIRE_OVERHEAD_BYTES)
        .saturating_sub(extra_envelope_bytes);
    // per_action is a non-zero constant; the division is total.
    let by_size = (size_budget / per_action) as usize;
    structural.min(by_size)
}

/// Calibrated effective storage-byte cost of the Core withdrawal document a
/// `ShieldedWithdrawal` creates.
///
/// A `ShieldedWithdrawal` does not only write notes/nullifiers like the other pool-paid
/// transitions — it ALSO inserts a Core withdrawal document into the withdrawals contract
/// (`AddWithdrawalDocument`), which writes the document plus its withdrawals-contract index
/// entries. That insert has a real, GroveDB-metered cost of ≈110,085,900 credits, which is
/// ~98% storage and is FLAT regardless of the bundle's action count (the document and its
/// indexes are the same size whether the withdrawal spends one note or sixteen).
///
/// `compute_minimum_shielded_fee` prices only the per-action note/nullifier storage and the
/// per-bundle ZK compute, so it does NOT cover this document insert. We therefore add the
/// document cost to the ShieldedWithdrawal fee as a flat BYTE-BASED component, sized at
/// `SHIELDED_WITHDRAWAL_DOCUMENT_STORAGE_BYTES` effective bytes priced at the SAME per-byte
/// storage rate the per-action note storage uses (`disk + processing` credits/byte). The
/// measured ≈110M cost corresponds to ≈4017 effective bytes at that rate; 4100 covers it with
/// a small (~2%) margin, and — because it is priced off the same rate — it tracks the storage
/// rate as it evolves, exactly like the per-action note storage does. See
/// [`compute_minimum_shielded_fee::compute_shielded_withdrawal_fee`].
pub const SHIELDED_WITHDRAWAL_DOCUMENT_STORAGE_BYTES: u64 = 4100;

/// Calibrated effective storage-byte cost of the single `AddBalanceToAddress` write an `Unshield`
/// performs, crediting the net (`unshielding_amount − fee`) to the output platform address.
///
/// Like the other pool-paid transitions, an `Unshield` writes its change notes and nullifiers — but
/// it ALSO credits a transparent platform address with `AddBalanceToAddress`. In the new-address
/// worst case that write touches the address subtree (the address path plus its balance/nonce
/// entries), a real, GroveDB-metered cost of ≈6,239,100 credits (≈222 of those bytes are storage)
/// that is FLAT regardless of the bundle's action count (the address write is the same size whether
/// the unshield spends one note or sixteen).
///
/// `compute_minimum_shielded_fee` prices only the per-action note/nullifier storage and the
/// per-bundle ZK compute, so it does NOT cover this address write. We therefore add the address
/// cost to the Unshield fee as a flat BYTE-BASED component, sized at
/// `SHIELDED_UNSHIELD_ADDRESS_STORAGE_BYTES` effective bytes priced at the SAME per-byte storage
/// rate the per-action note storage uses (`disk + processing` credits/byte).
///
/// The constant is the **storage** portion of the address write: the metered `AddBalanceToAddress`
/// op costs ≈6,239,100 credits total, of which the *storage* part is ≈6,075,000 ≈ **222 effective
/// bytes** at the storage rate. We size the component to that storage figure — because it is a
/// `bytes × per_byte_rate` term it is booked as storage, so it should match the address write's
/// storage cost, not its total. The small remaining op-processing (~164K) is already covered by the
/// per-action processing fee. Pricing it off the same rate means it tracks the storage rate as it
/// evolves, exactly like the per-action note storage does. See
/// [`compute_minimum_shielded_fee::compute_shielded_unshield_fee`].
pub const SHIELDED_UNSHIELD_ADDRESS_STORAGE_BYTES: u64 = 222;

/// Common Orchard bundle parameters shared across all shielded transition types.
///
/// Groups the fields that every shielded transition carries identically:
/// the serialized actions, Sinsemilla anchor, Halo 2 proof, and RedPallas
/// binding signature. Using this struct reduces parameter counts in SDK
/// helper functions from 10-12 down to 5-8.
pub struct OrchardBundleParams {
    /// The serialized Orchard actions (spends + outputs).
    pub actions: Vec<SerializedAction>,
    /// Sinsemilla root of the note commitment tree at bundle creation time (32 bytes).
    /// This is the Orchard Anchor — the root of the depth-32 Sinsemilla Merkle
    /// tree over extracted note commitments (cmx values), NOT the GroveDB
    /// commitment tree state root.
    pub anchor: [u8; 32],
    /// Halo 2 zero-knowledge proof bytes.
    pub proof: Vec<u8>,
    /// RedPallas binding signature (64 bytes) over the bundle's value balance.
    pub binding_signature: [u8; 64],
}

/// A serialized Orchard action extracted from a bundle.
///
/// Each Orchard action structurally contains one spend and one output. The spend
/// consumes a previously created note (revealing its nullifier), while the output
/// creates a new note (publishing its commitment). Although paired in the same struct,
/// observers cannot link which prior note was spent or what value the new note holds —
/// the zero-knowledge proof ensures privacy.
///
/// These fields are raw bytes suitable for network serialization. During validation,
/// they are parsed back into typed Orchard structs and verified via `BatchValidator`
/// (Halo 2 proof + RedPallas signatures).
///
/// All fields except `spend_auth_sig` are covered by the Orchard bundle commitment
/// (BLAKE2b-256 per ZIP-244), which feeds into the platform sighash. The signatures
/// and proof are verified separately and are not part of the commitment.
/// `#[json_safe_fields]` auto-injects `#[serde(with = ...)]` on the byte fields:
/// every `[u8; N]` → `serde_bytes` (const-generic), `Vec<u8>` → `serde_bytes_var`.
/// Keeps the wire shape (Uint8Array in binary, base64 string in JSON) without
/// per-field annotations.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[derive(Debug, Clone, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
pub struct SerializedAction {
    /// Unique tag derived from the spent note's position and spending key.
    /// Published on-chain to prevent double-spends: if this nullifier already
    /// exists in the nullifier set, the transaction is rejected. The nullifier
    /// is deterministic for a given note but unlinkable to the note's commitment,
    /// preserving sender privacy.
    pub nullifier: [u8; 32],

    /// Randomized spend validating key (RedPallas verification key).
    /// Derived from the spender's full viewing key with per-action randomness.
    /// Used to verify `spend_auth_sig`, proving the spender controls the spending
    /// key for the consumed note without revealing which key it is.
    pub rk: [u8; 32],

    /// Extracted note commitment for the newly created output note.
    /// This is added to the commitment tree after the transition is applied,
    /// allowing the recipient to later spend it. The commitment hides the note's
    /// value, recipient, and randomness — only the recipient (who knows the
    /// decryption key) can identify and spend this note.
    pub cmx: [u8; 32],

    /// Encrypted note ciphertext (216 bytes = epk 32 + enc_ciphertext 104 + out_ciphertext 80).
    /// Contains the `TransmittedNoteCiphertext` fields packed contiguously:
    /// - `epk`: ephemeral public key for Diffie-Hellman key agreement (32 bytes)
    /// - `enc_ciphertext`: note plaintext encrypted to the recipient (104 bytes = 52 compact + 36 memo + 16 AEAD tag)
    /// - `out_ciphertext`: encrypted to the sender for wallet recovery (80 bytes)
    ///
    /// Stored on-chain so recipients can scan and decrypt notes addressed to them.
    /// Only the intended recipient (or sender) can decrypt; all others see random bytes.
    pub encrypted_note: Vec<u8>,

    /// Value commitment (Pedersen commitment to the note's value).
    /// Commits to the value flowing through this action without revealing it.
    /// The binding signature later proves that the sum of all `cv_net` commitments
    /// across actions is consistent with the declared `value_balance`, ensuring
    /// no credits are created or destroyed.
    pub cv_net: [u8; 32],

    /// RedPallas spend authorization signature over the platform sighash.
    /// Proves the spender authorized this specific bundle (including all actions,
    /// value_balance, anchor, and any bound transparent fields). Verified against
    /// `rk` during batch validation. This prevents replay attacks — a valid
    /// signature from one transition cannot be reused in another.
    pub spend_auth_sig: [u8; 64],
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for SerializedAction {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for SerializedAction {}

#[cfg(test)]
mod wire_cost_tests {
    use super::*;
    use platform_version::version::PlatformVersion;

    /// Pin the linear wire-cost model to the sizes measured on real proved
    /// transitions (recorded in the `seed_pool_batch_fits_max_state_transition_size`
    /// signing test: 2 actions → 8,294 B, 6 → 19,018 B, 7 → 21,699 B — the last
    /// rejected by tenderdash's `mempool.max-tx-bytes = 20480` as "Tx too
    /// large"). If a proof- or action-encoding change moves these numbers, this
    /// fails alongside that signing test and the constants must be re-measured.
    #[test]
    fn shielded_wire_cost_model_matches_measured_transitions() {
        assert_eq!(estimated_shielded_transition_wire_bytes(2), 8_294);
        assert_eq!(estimated_shielded_transition_wire_bytes(6), 19_018);
        assert_eq!(estimated_shielded_transition_wire_bytes(7), 21_699);
    }

    /// The effective ceiling must be derived from BOTH versioned limits, and at
    /// the current constants (20 KiB size limit, 16-action structural cap) the
    /// size limit is the binding one: 6 actions fit, 7 do not. This is the
    /// number the `system_limits` doc comments state; a version bump that
    /// changes either constant moves this derivation with it.
    #[test]
    fn effective_action_ceiling_is_size_bound_at_current_limits() {
        let platform_version = PlatformVersion::latest();
        let effective = max_shielded_actions_per_transition(platform_version);
        let structural = platform_version
            .system_limits
            .max_shielded_transition_actions as usize;
        let max_size = platform_version.system_limits.max_state_transition_size;

        assert_eq!(
            effective, 6,
            "at a 20 KiB size limit the derived ceiling must be 6 actions"
        );
        assert!(
            effective <= structural,
            "the effective ceiling can never exceed the structural cap"
        );
        // The derivation must be exactly "largest n whose estimated size fits".
        assert!(
            estimated_shielded_transition_wire_bytes(effective) <= max_size,
            "the ceiling itself must fit the size limit"
        );
        assert!(
            effective == structural
                || estimated_shielded_transition_wire_bytes(effective + 1) > max_size,
            "one more action than the (size-bound) ceiling must NOT fit"
        );
    }

    /// Transition-specific envelope bytes must tighten the ceiling at the
    /// exact byte boundary (#4312 review finding e90e9cf15f52): extra bytes
    /// within the slack the baseline ceiling leaves under the size limit keep
    /// the ceiling; one byte past the slack displaces an action; an envelope
    /// larger than the whole budget must degrade to a zero ceiling, never
    /// panic or wrap.
    #[test]
    fn envelope_bytes_tighten_the_action_ceiling_at_the_exact_boundary() {
        let platform_version = PlatformVersion::latest();
        let baseline = max_shielded_actions_per_transition(platform_version);
        let per_action = SHIELDED_ACTION_WIRE_BYTES + SHIELDED_PROOF_WIRE_BYTES_PER_ACTION;
        let max_size = platform_version.system_limits.max_state_transition_size;
        // Bytes left under the size limit once the baseline envelope and the
        // baseline-ceiling actions are paid for.
        let slack =
            max_size - SHIELDED_TRANSITION_WIRE_OVERHEAD_BYTES - baseline as u64 * per_action;

        assert_eq!(
            max_shielded_actions_for_envelope(platform_version, 0),
            baseline,
            "a zero extra envelope must reproduce the baseline ceiling"
        );
        assert_eq!(
            max_shielded_actions_for_envelope(platform_version, slack),
            baseline,
            "an envelope exactly filling the slack must keep the ceiling"
        );
        assert_eq!(
            max_shielded_actions_for_envelope(platform_version, slack + 1),
            baseline - 1,
            "one byte past the slack must displace one action"
        );
        assert_eq!(
            max_shielded_actions_for_envelope(platform_version, u64::MAX),
            0,
            "an envelope beyond the whole budget must degrade to zero, not wrap"
        );

        // The estimator and the ceiling must agree: the ceiling is exactly
        // the largest action count whose estimated size (with the same
        // envelope) fits the limit.
        for extra in [0, slack, slack + 1] {
            let ceiling = max_shielded_actions_for_envelope(platform_version, extra);
            assert!(
                estimated_shielded_transition_wire_bytes_with_envelope(ceiling, extra) <= max_size
            );
            assert!(
                estimated_shielded_transition_wire_bytes_with_envelope(ceiling + 1, extra)
                    > max_size
            );
        }
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> SerializedAction {
        SerializedAction {
            nullifier: [0x11; 32],
            rk: [0x22; 32],
            cmx: [0x33; 32],
            // Encrypted note is variable-length (216 bytes per the field doc); a
            // shorter payload still exercises the `serde_bytes_var` path.
            encrypted_note: vec![0x44, 0x55, 0x66, 0x77],
            cv_net: [0x88; 32],
            spend_auth_sig: [0x99; 64],
        }
    }

    // `SerializedAction` is a struct with `serde(rename_all = "camelCase")`.
    // `#[json_safe_fields]` auto-injects `#[serde(with = ...)]` on the byte
    // fields: `[u8; N]` → `serde_bytes` (const-generic), `Vec<u8>` →
    // `serde_bytes_var`. The wire shape is base64 strings in JSON HR and
    // raw bytes in non-HR.

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        use base64::{engine::general_purpose::STANDARD, Engine};
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Each byte field is base64-encoded in HR.
        assert_eq!(
            json,
            json!({
                "nullifier": STANDARD.encode([0x11; 32]),
                "rk": STANDARD.encode([0x22; 32]),
                "cmx": STANDARD.encode([0x33; 32]),
                "encryptedNote": STANDARD.encode([0x44, 0x55, 0x66, 0x77]),
                "cvNet": STANDARD.encode([0x88; 32]),
                "spendAuthSig": STANDARD.encode([0x99; 64]),
            })
        );
        let recovered = SerializedAction::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        use platform_value::Value;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        // `[u8; 32]` → `Value::Bytes32`, `[u8; 64]` and `Vec<u8>` (via
        // `serde_bytes_var`) → `Value::Bytes(Vec<u8>)`.
        assert_eq!(
            value,
            Value::Map(vec![
                (Value::Text("nullifier".into()), Value::Bytes32([0x11; 32])),
                (Value::Text("rk".into()), Value::Bytes32([0x22; 32])),
                (Value::Text("cmx".into()), Value::Bytes32([0x33; 32])),
                (
                    Value::Text("encryptedNote".into()),
                    Value::Bytes(vec![0x44, 0x55, 0x66, 0x77]),
                ),
                (Value::Text("cvNet".into()), Value::Bytes32([0x88; 32])),
                (
                    Value::Text("spendAuthSig".into()),
                    Value::Bytes(vec![0x99; 64]),
                ),
            ])
        );
        let recovered = SerializedAction::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
