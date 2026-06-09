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
    compute_minimum_shielded_fee, compute_shielded_identity_create_fee,
    compute_shielded_unshield_fee, compute_shielded_verification_fee,
    compute_shielded_withdrawal_fee,
};

use crate::address_funds::PlatformAddress;
use crate::identity::identity_public_key::contract_bounds::ContractBounds;
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;

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

/// Builds the transparent `extra_data` bound into an `IdentityCreateFromShieldedPool`'s platform
/// sighash, with the byte layout
/// `identity_id (32) || denomination (u64 LE)
///   || send_to_address_on_creation_failure (tag u8: 0=P2pkh, 1=P2sh || hash 20)
///   || num_keys (u16 LE)
///   || for each key in supplied order: key_id (u32 LE) || purpose (u8) || security_level (u8)
///   || key_type (u8) || key_data_len (u16 LE) || key_data || read_only (u8)
///   || contract_bounds (tag u8: 0=None, 1=SingleContract id(32), 2=SingleContractDocumentType
///   id(32) name_len(u16 LE) name)`.
///
/// `IdentityCreateFromShieldedPool` carries NO platform identity signature: authorization is 100%
/// the Orchard proof + per-action spend-auth signatures + binding signature over this sighash. The
/// transparent, state-determining fields — the new identity id, the exit denomination, and the
/// FULL public-key set — must therefore be committed into the Orchard sighash, exactly as the
/// `surplus_output` field is committed into `ShieldFromAssetLock`'s ECDSA signature. Without this
/// binding a relay or block proposer could take a valid bundle exiting a denomination and re-point
/// it at a DIFFERENT identity id, or swap in DIFFERENT keys they control, stealing the credited
/// balance (the per-key proofs-of-possession alone do NOT prevent this — a relayer keeps valid PoP
/// sigs for their own keys while swapping the bundle). Binding `(this spend → these exact keys →
/// this id → this denomination)` here makes the redirection atomic-or-invalid.
///
/// The signing (client/builder) and verifying (consensus) sides MUST produce identical bytes, so
/// both call this single function. Unlike the fixed-length withdrawal/unshield helpers, the
/// variable-length key list is fully length-prefixed (both the key count and each key's data) so
/// the preimage is unambiguous for any key set.
pub fn identity_create_from_shielded_extra_sighash_data(
    identity_id: &[u8; 32],
    denomination: u64,
    send_to_address_on_creation_failure: &PlatformAddress,
    public_keys: &[IdentityPublicKeyInCreation],
) -> Vec<u8> {
    let mut data = Vec::with_capacity(32 + 8 + 21 + 2 + public_keys.len() * 44);
    data.extend_from_slice(identity_id);
    data.extend_from_slice(&denomination.to_le_bytes());
    // Bind the fallback address (type tag || 20-byte hash) so a relayer cannot redirect the
    // failure credit. Mirrors the way `unshield`/`withdrawal` bind their output address.
    match send_to_address_on_creation_failure {
        PlatformAddress::P2pkh(hash) => {
            data.push(0u8);
            data.extend_from_slice(hash);
        }
        PlatformAddress::P2sh(hash) => {
            data.push(1u8);
            data.extend_from_slice(hash);
        }
    }
    data.extend_from_slice(&(public_keys.len() as u16).to_le_bytes());
    for key in public_keys {
        data.extend_from_slice(&key.id().to_le_bytes());
        data.push(key.purpose() as u8);
        data.push(key.security_level() as u8);
        data.push(key.key_type() as u8);
        let key_data = key.data().as_slice();
        data.extend_from_slice(&(key_data.len() as u16).to_le_bytes());
        data.extend_from_slice(key_data);
        // Also bind `read_only` and `contract_bounds`. These are state-determining key fields that
        // ARE in the transition's signable_bytes, but the per-key proof-of-possession does NOT bind
        // them for hash-based key types (which accept an empty signature). Committing them into the
        // Orchard binding sighash makes them un-malleable for EVERY key type, so a relayer/proposer
        // cannot flip `read_only` or alter `contract_bounds` on an observed transition.
        data.push(key.read_only() as u8);
        match key.contract_bounds() {
            None => data.push(0u8),
            Some(ContractBounds::SingleContract { id }) => {
                data.push(1u8);
                data.extend_from_slice(id.as_bytes());
            }
            Some(ContractBounds::SingleContractDocumentType {
                id,
                document_type_name,
            }) => {
                data.push(2u8);
                data.extend_from_slice(id.as_bytes());
                let name = document_type_name.as_bytes();
                data.extend_from_slice(&(name.len() as u16).to_le_bytes());
                data.extend_from_slice(name);
            }
        }
    }
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

    mod identity_create_sighash {
        use super::*;
        use crate::identity::{KeyType, Purpose, SecurityLevel};
        use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
        use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
        use platform_value::BinaryData;

        fn mk_key(id: u32, data_byte: u8) -> IdentityPublicKeyInCreation {
            IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
                id,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::MASTER,
                contract_bounds: None,
                read_only: false,
                data: BinaryData::new(vec![data_byte; 33]),
                signature: BinaryData::new(vec![]),
            })
        }

        #[test]
        fn layout_is_length_prefixed() {
            // identity_id(32) || denomination(8)
            //   || send_to_address_on_creation_failure (tag(1) || hash(20))
            //   || num_keys(2)
            //   || [key_id(4)|purpose|sec|type|len(2)|data|read_only(1)|contract_bounds_tag(1)]
            let id = [0x11u8; 32];
            let keys = vec![mk_key(7, 0xAB)];
            let fallback = PlatformAddress::P2pkh([0x5Cu8; 20]);
            let d = identity_create_from_shielded_extra_sighash_data(
                &id,
                10_000_000_000,
                &fallback,
                &keys,
            );
            assert_eq!(&d[0..32], &id);
            assert_eq!(&d[32..40], &10_000_000_000u64.to_le_bytes());
            // Fallback address: tag(0=P2pkh) at offset 40, 20-byte hash at 41..61.
            assert_eq!(d[40], 0u8, "fallback address P2pkh tag");
            assert_eq!(&d[41..61], &[0x5Cu8; 20], "fallback address hash");
            assert_eq!(&d[61..63], &1u16.to_le_bytes());
            assert_eq!(&d[63..67], &7u32.to_le_bytes());
            assert_eq!(d[67], Purpose::AUTHENTICATION as u8);
            assert_eq!(d[68], SecurityLevel::MASTER as u8);
            assert_eq!(d[69], KeyType::ECDSA_SECP256K1 as u8);
            assert_eq!(&d[70..72], &33u16.to_le_bytes());
            assert_eq!(&d[72..105], &[0xAB; 33]);
            assert_eq!(d[105], 0u8, "read_only=false");
            assert_eq!(d[106], 0u8, "contract_bounds=None tag");
            assert_eq!(d.len(), 32 + 8 + 21 + 2 + (4 + 1 + 1 + 1 + 2 + 33 + 1 + 1));
        }

        #[test]
        fn binds_identity_id_denomination_and_keys() {
            let id_a = [0x11u8; 32];
            let id_b = [0x22u8; 32];
            let keys = vec![mk_key(0, 0xAA)];
            let fallback = PlatformAddress::P2pkh([0x01u8; 20]);
            let base = identity_create_from_shielded_extra_sighash_data(
                &id_a,
                10_000_000_000,
                &fallback,
                &keys,
            );

            // Changing the identity id changes the preimage (anti-redirection to a different id).
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_b,
                    10_000_000_000,
                    &fallback,
                    &keys
                ),
                "identity id must be bound"
            );
            // Changing the denomination changes the preimage.
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_a,
                    30_000_000_000,
                    &fallback,
                    &keys
                ),
                "denomination must be bound"
            );
            // Changing the fallback failure address changes the preimage (anti-redirection of the
            // failure credit: a relayer cannot point the penalty-charged spend at a different
            // address than the one each key's proof-of-possession signed).
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_a,
                    10_000_000_000,
                    &PlatformAddress::P2pkh([0x02u8; 20]),
                    &keys
                ),
                "fallback failure address hash must be bound"
            );
            // Changing only the fallback address TYPE (P2pkh -> P2sh, same hash) changes the
            // preimage too (the type tag is bound, not just the hash).
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_a,
                    10_000_000_000,
                    &PlatformAddress::P2sh([0x01u8; 20]),
                    &keys
                ),
                "fallback failure address type tag must be bound"
            );
            // Swapping in a different key changes the preimage (anti-key-swap).
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_a,
                    10_000_000_000,
                    &fallback,
                    &[mk_key(0, 0xBB)]
                ),
                "key data must be bound"
            );
            // Adding a key changes the preimage (the full set is bound, not just the count).
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id_a,
                    10_000_000_000,
                    &fallback,
                    &[mk_key(0, 0xAA), mk_key(1, 0xCC)]
                ),
                "the full key set must be bound"
            );
        }

        #[test]
        fn binds_read_only_and_contract_bounds() {
            use crate::identity::identity_public_key::contract_bounds::ContractBounds;
            use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
            let id = [0x11u8; 32];
            let fallback = PlatformAddress::P2pkh([0x01u8; 20]);
            let base = identity_create_from_shielded_extra_sighash_data(
                &id,
                10_000_000_000,
                &fallback,
                &[mk_key(0, 0xAA)],
            );

            // Flipping read_only changes the preimage (un-malleable for every key type).
            let mut ro_key = mk_key(0, 0xAA);
            ro_key.set_read_only(true);
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id,
                    10_000_000_000,
                    &fallback,
                    &[ro_key]
                ),
                "read_only must be bound"
            );

            // Attaching contract_bounds changes the preimage.
            let mut cb_key = mk_key(0, 0xAA);
            cb_key.set_contract_bounds(Some(ContractBounds::SingleContract {
                id: platform_value::Identifier::new([0x33; 32]),
            }));
            assert_ne!(
                base,
                identity_create_from_shielded_extra_sighash_data(
                    &id,
                    10_000_000_000,
                    &fallback,
                    &[cb_key]
                ),
                "contract_bounds must be bound"
            );
        }
    }
}
