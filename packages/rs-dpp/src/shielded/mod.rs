#[cfg(feature = "shielded-client")]
pub mod builder;

mod compute_minimum_shielded_fee;

use bincode::{Decode, Encode};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::withdrawal::Pooling;

// Re-exported so the public path stays `dpp::shielded::compute_minimum_shielded_fee` (the
// module and the function share a name but live in different namespaces).
pub use compute_minimum_shielded_fee::{
    compute_minimum_shielded_fee, compute_shielded_unshield_fee, compute_shielded_verification_fee,
    compute_shielded_withdrawal_fee,
};

/// Permanent storage bytes per shielded action: 312 bytes total.
///
/// - 280 bytes in the BulkAppendTree: 32 (`cmx`, the note commitment) + 32
///   (`rho`) + 216 (the encrypted note ciphertext).
/// - 32 bytes in the nullifier tree.
///
/// The 216-byte encrypted note is Orchard's `TransmittedNoteCiphertext`, laid
/// out as `epk(32) || enc_ciphertext(104) || out_ciphertext(80)`:
///
/// - `epk` (32): the note's ephemeral public key, published in the clear. The
///   recipient combines it with their incoming viewing key (Diffie–Hellman) to
///   derive the AEAD key.
/// - `enc_ciphertext` (104): the note encrypted to the recipient (opened with
///   the incoming viewing key) — ChaCha20-Poly1305 over the note plaintext. It
///   holds the compact note (52 = version 1 + diversifier `d` 11 + value 8 +
///   `rseed` 32), the memo (36), and the AEAD tag (16); the 52-byte compact
///   prefix is what wallets trial-decrypt during sync to detect their own notes.
/// - `out_ciphertext` (80): the note encrypted to the sender for wallet
///   recovery (opened with the outgoing viewing key): out plaintext
///   (64 = `pk_d` 32 + `esk` 32) + AEAD tag (16).
///
/// This is the standard Orchard layout except the memo is 36 bytes (`DashMemo`)
/// instead of Zcash's 512 — the dashpay `orchard` fork makes the memo size a
/// type parameter (`MemoSize`) — which is why each note is 216 bytes
/// (`ENCRYPTED_NOTE_SIZE`) rather than Zcash Orchard's ~692.
pub const SHIELDED_STORAGE_BYTES_PER_ACTION: u64 = 312;

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

/// Domain separator for Platform sighash computation.
const SIGHASH_DOMAIN: &[u8] = b"DashPlatformSighash";

/// Computes the platform sighash from an Orchard bundle commitment and optional
/// transparent field data.
///
/// The sighash is computed as:
///   `SHA-256(SIGHASH_DOMAIN || bundle_commitment || extra_data)`
///
/// This binds transparent state transition fields (like `output_address` in unshield
/// or `output_script` in shielded withdrawal) to the Orchard signatures, preventing
/// replay attacks where an attacker substitutes transparent fields while reusing a
/// valid Orchard bundle.
///
/// The same computation must be used on both the signing (client) and verification
/// (platform) sides. For transitions without transparent fields (shield and
/// shielded_transfer), `extra_data` is empty.
pub fn compute_platform_sighash(bundle_commitment: &[u8; 32], extra_data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(SIGHASH_DOMAIN);
    hasher.update(bundle_commitment);
    hasher.update(extra_data);
    hasher.finalize().into()
}

/// Builds the transparent `extra_data` bound into a ShieldedWithdrawal's platform
/// sighash, with the byte layout
/// `output_script || unshielding_amount (u64 LE) || core_fee_per_byte (u32 LE) || pooling (u8)`.
///
/// Every field here is written verbatim by the transformer into the queued withdrawal
/// document that constructs the Core asset-unlock TxOut. Binding all of them into the
/// Orchard sighash means the binding signature authorizes them: since ShieldedWithdrawal
/// has no identity-key signature and no address-witness check, the Orchard signature is
/// the only authorization boundary, so a relay or block proposer cannot malleate
/// `core_fee_per_byte` (or `pooling`, were it ever unpinned from `Never`) — e.g. flip a
/// user's `core_fee_per_byte = 1` to a much larger Fibonacci value to redirect the
/// withdrawn amount into L1 miner fees — without invalidating the proof.
///
/// The signing (client/builder) and verifying (consensus) sides MUST produce identical
/// bytes, so both call this single function.
///
/// The layout places the variable-length `output_script` first with no length prefix. This
/// is unambiguous only because `validate_structure` runs before proof verification and pins
/// `output_script` to a canonical, fixed-length P2PKH (25 bytes) or P2SH (23 bytes); the
/// remaining fields are fixed-width, so the preimage is well-defined for every accepted
/// transition. If that script-shape restriction is ever relaxed, add a length prefix here.
pub fn shielded_withdrawal_extra_sighash_data(
    output_script: &[u8],
    unshielding_amount: u64,
    core_fee_per_byte: u32,
    pooling: Pooling,
) -> Vec<u8> {
    let mut data = Vec::with_capacity(output_script.len() + 8 + 4 + 1);
    data.extend_from_slice(output_script);
    data.extend_from_slice(&unshielding_amount.to_le_bytes());
    data.extend_from_slice(&core_fee_per_byte.to_le_bytes());
    data.push(pooling as u8);
    data
}

/// Builds the transparent `extra_data` bound into an Unshield's platform sighash, with the
/// byte layout `output_address || unshielding_amount (u64 LE)`.
///
/// As with [`shielded_withdrawal_extra_sighash_data`], the signing (client/builder) and
/// verifying (consensus) sides MUST produce identical bytes, so both call this single
/// function. Unshield credits a transparent platform address (not a Core asset-unlock
/// `TxOut`), so it carries no `core_fee_per_byte`/`pooling` to bind.
pub fn unshield_extra_sighash_data(output_address: &[u8], unshielding_amount: u64) -> Vec<u8> {
    let mut data = Vec::with_capacity(output_address.len() + 8);
    data.extend_from_slice(output_address);
    data.extend_from_slice(&unshielding_amount.to_le_bytes());
    data
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::core_script::CoreScript;
    use crate::withdrawal::Pooling;

    #[test]
    fn withdrawal_sighash_data_binds_core_fee_per_byte() {
        let script = CoreScript::new_p2pkh([1u8; 20]);
        let a = shielded_withdrawal_extra_sighash_data(script.as_bytes(), 1000, 1, Pooling::Never);
        let b = shielded_withdrawal_extra_sighash_data(script.as_bytes(), 1000, 2, Pooling::Never);
        assert_ne!(
            a, b,
            "changing core_fee_per_byte must change the sighash preimage"
        );
    }

    #[test]
    fn withdrawal_sighash_data_binds_pooling() {
        // `pooling` is pinned to `Never` by `validate_structure`, so this binding is currently
        // dead defense-in-depth; assert it is nonetheless mixed into the preimage so a future
        // unpinning would still be authorized by the Orchard binding signature.
        let script = CoreScript::new_p2pkh([1u8; 20]);
        let a = shielded_withdrawal_extra_sighash_data(script.as_bytes(), 1000, 1, Pooling::Never);
        let b = shielded_withdrawal_extra_sighash_data(
            script.as_bytes(),
            1000,
            1,
            Pooling::IfAvailable,
        );
        assert_ne!(a, b, "changing pooling must change the sighash preimage");
    }

    #[test]
    fn withdrawal_sighash_data_layout() {
        // output_script(2) || unshielding_amount(8) || core_fee_per_byte(4) || pooling(1)
        let d = shielded_withdrawal_extra_sighash_data(&[0xAA, 0xBB], 1, 2, Pooling::Never);
        assert_eq!(d.len(), 2 + 8 + 4 + 1);
        assert_eq!(&d[0..2], &[0xAA, 0xBB]);
        assert_eq!(&d[2..10], &1u64.to_le_bytes());
        assert_eq!(&d[10..14], &2u32.to_le_bytes());
        assert_eq!(d[14], Pooling::Never as u8);
    }

    #[test]
    fn unshield_sighash_data_layout() {
        // output_address || unshielding_amount(8)
        let d = unshield_extra_sighash_data(&[0xAA, 0xBB, 0xCC], 5);
        assert_eq!(d.len(), 3 + 8);
        assert_eq!(&d[0..3], &[0xAA, 0xBB, 0xCC]);
        assert_eq!(&d[3..11], &5u64.to_le_bytes());
    }
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for SerializedAction {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for SerializedAction {}

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
