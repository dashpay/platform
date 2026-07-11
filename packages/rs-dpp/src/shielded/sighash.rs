//! Platform sighash preimage construction for shielded transitions.
//!
//! Shielded transitions carry NO platform identity signature — authorization is the Orchard proof +
//! per-action spend-auth signatures + the RedPallas binding signature over the platform sighash.
//! These helpers build the transparent `extra_data` each transition binds into that sighash so the
//! signing (client/builder) and verifying (consensus) sides commit to identical bytes. The byte
//! layouts are consensus-critical and versioned via `dpp.methods.shielded_extra_sighash_data`.

use crate::address_funds::PlatformAddress;
use crate::identity::identity_public_key::contract_bounds::ContractBounds;
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::withdrawal::Pooling;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;
use sha2::{Digest, Sha256};

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
/// Dispatches on the platform-versioned `dpp.methods.shielded_extra_sighash_data` so the
/// consensus-critical byte layout can evolve across protocol versions without breaking older
/// transitions — the same versioning the sibling shielded fee methods use. The signing
/// (client/builder) and verifying (consensus) sides both call this single function with the same
/// `platform_version`, so they can never produce divergent preimages.
pub fn shielded_withdrawal_extra_sighash_data(
    output_script: &[u8],
    unshielding_amount: u64,
    core_fee_per_byte: u32,
    pooling: Pooling,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(shielded_withdrawal_extra_sighash_data_v0(
            output_script,
            unshielding_amount,
            core_fee_per_byte,
            pooling,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "shielded_withdrawal_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// v0 byte layout of [`shielded_withdrawal_extra_sighash_data`] (see that function's doc comment for
/// the layout and rationale). Frozen: never mutate; a layout change requires a new `_v1` + version.
pub fn shielded_withdrawal_extra_sighash_data_v0(
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
pub fn unshield_extra_sighash_data(
    output_address: &[u8],
    unshielding_amount: u64,
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(unshield_extra_sighash_data_v0(
            output_address,
            unshielding_amount,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "unshield_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// v0 byte layout of [`unshield_extra_sighash_data`] (see that function's doc comment for the layout
/// and rationale). Frozen: never mutate; a layout change requires a new `_v1` + version bump.
pub fn unshield_extra_sighash_data_v0(output_address: &[u8], unshielding_amount: u64) -> Vec<u8> {
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
    platform_version: &PlatformVersion,
) -> Result<Vec<u8>, ProtocolError> {
    match platform_version.dpp.methods.shielded_extra_sighash_data {
        0 => Ok(identity_create_from_shielded_extra_sighash_data_v0(
            identity_id,
            denomination,
            send_to_address_on_creation_failure,
            public_keys,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "identity_create_from_shielded_extra_sighash_data".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}

/// v0 byte layout of [`identity_create_from_shielded_extra_sighash_data`] (see that function's doc
/// comment for the layout and rationale). Frozen: never mutate; a layout change requires a new `_v1`
/// + version bump.
pub fn identity_create_from_shielded_extra_sighash_data_v0(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::core_script::CoreScript;
    use crate::withdrawal::Pooling;
    // These tests pin the v0 preimage directly (they assert exact bytes), so resolve the bare helper
    // names to the `_v0` impls rather than the version-dispatching public wrappers.
    use crate::shielded::shielded_withdrawal_extra_sighash_data_v0 as shielded_withdrawal_extra_sighash_data;
    use crate::shielded::unshield_extra_sighash_data_v0 as unshield_extra_sighash_data;

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
        // Pin the v0 preimage directly (see the note in the parent test module).
        use crate::identity::{KeyType, Purpose, SecurityLevel};
        use crate::shielded::identity_create_from_shielded_extra_sighash_data_v0 as identity_create_from_shielded_extra_sighash_data;
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
