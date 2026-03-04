#[cfg(feature = "shielded-bundle-building")]
pub mod builder;

use bincode::{Decode, Encode};
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::fee::Credits;
use platform_version::version::PlatformVersion;

/// Permanent storage bytes per shielded action:
/// 280 bytes in BulkAppendTree (32 cmx + 32 rho + 216 encrypted note)
/// + 32 bytes in nullifier tree = 312 bytes total.
pub const SHIELDED_STORAGE_BYTES_PER_ACTION: u64 = 312;

/// Domain separator for Platform sighash computation.
const SIGHASH_DOMAIN: &[u8] = b"DashPlatformSighash";

/// Computes the platform sighash from an Orchard bundle commitment and optional
/// transparent field data.
///
/// The sighash is computed as:
///   `SHA-256(SIGHASH_DOMAIN || bundle_commitment || extra_data)`
///
/// This binds transparent state transition fields (like `output_address` and `amount`
/// in unshield transitions) to the Orchard signatures, preventing replay attacks
/// where an attacker substitutes transparent fields while reusing a valid Orchard bundle.
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

/// Computes the minimum fee (in credits) for a shielded state transition.
///
/// The fee formula mirrors the on-chain validation in `validate_minimum_shielded_fee`:
///   `min_fee = proof_verification_fee + num_actions × (processing_fee + storage_fee)`
///
/// where `storage_fee = SHIELDED_STORAGE_BYTES_PER_ACTION × (disk + processing) credits/byte`.
///
/// # Parameters
/// - `num_actions` — number of Orchard actions in the bundle
/// - `platform_version` — protocol version (determines fee constants)
pub fn compute_minimum_shielded_fee(
    num_actions: usize,
    platform_version: &PlatformVersion,
) -> Credits {
    let constants = &platform_version
        .drive_abci
        .validation_and_processing
        .event_constants;
    let storage = &platform_version.fee_version.storage;
    let storage_fee = SHIELDED_STORAGE_BYTES_PER_ACTION
        * (storage.storage_disk_usage_credit_per_byte + storage.storage_processing_credit_per_byte);
    let per_action = constants.shielded_per_action_processing_fee + storage_fee;
    constants.shielded_proof_verification_fee + num_actions as u64 * per_action
}

/// Serde helper for `[u8; 64]` fields (serde only supports arrays up to 32).
#[cfg(feature = "state-transition-serde-conversion")]
pub(crate) mod serde_bytes_64 {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(bytes)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[u8; 64], D::Error> {
        let vec = <Vec<u8>>::deserialize(deserializer)?;
        vec.try_into().map_err(|v: Vec<u8>| {
            serde::de::Error::custom(format!("expected 64 bytes, got {}", v.len()))
        })
    }
}

/// Common Orchard bundle parameters shared across all shielded transition types.
///
/// Groups the fields that every shielded transition carries identically:
/// the serialized actions, bundle flags, Sinsemilla anchor, Halo 2 proof,
/// and RedPallas binding signature. Using this struct reduces parameter counts
/// in SDK helper functions from 10-12 down to 5-8.
pub struct OrchardBundleParams {
    /// The serialized Orchard actions (spends + outputs).
    pub actions: Vec<SerializedAction>,
    /// Bundle flags byte.
    pub flags: u8,
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
#[derive(Debug, Clone, Encode, Decode, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
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
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(with = "serde_bytes_64")
    )]
    pub spend_auth_sig: [u8; 64],
}
