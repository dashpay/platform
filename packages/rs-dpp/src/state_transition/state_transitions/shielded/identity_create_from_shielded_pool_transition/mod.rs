pub mod accessors;
pub mod methods;
mod state_transition_estimated_fee_validation;
mod state_transition_like;
mod state_transition_validation;
pub mod v0;
mod version;

use crate::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
use crate::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0Signable;
use crate::state_transition::StateTransitionFieldTypes;

pub type IdentityCreateFromShieldedPoolTransitionLatest =
    IdentityCreateFromShieldedPoolTransitionV0;

use crate::identity::state_transition::OptionallyAssetLockProved;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
use crate::shielded::SerializedAction;
use crate::util::hash::hash_double;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use derive_more::From;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::Identifier;
use platform_versioning::PlatformVersioned;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformSignable,
    PlatformVersioned,
    From,
    PartialEq,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(
    all(feature = "json-conversion", feature = "serde-conversion"),
    derive(JsonConvertible)
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[platform_version_path_bounds(
    "dpp.state_transition_serialization_versions.identity_create_from_shielded_pool_state_transition"
)]
pub enum IdentityCreateFromShieldedPoolTransition {
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
    V0(IdentityCreateFromShieldedPoolTransitionV0),
}

/// Derives the new identity's id from a set of spend nullifiers as
/// `double_sha256(nullifier_0 || nullifier_1 || …)` over the SORTED nullifier set.
///
/// Nullifiers are globally-unique one-time spend tags (enforced by `validate_nullifiers`), so the
/// derived id is unique by construction and single-use. Sorting makes the id independent of
/// action ordering (non-malleable). The same derivation runs at consensus to re-derive and check
/// the supplied id, and the id is committed into the Orchard `extra_sighash_data`, so the bundle
/// cannot be redirected to a different identity.
pub fn identity_id_from_nullifiers(nullifiers: &[[u8; 32]]) -> Identifier {
    let mut sorted: Vec<[u8; 32]> = nullifiers.to_vec();
    sorted.sort_unstable();
    let mut buf = Vec::with_capacity(sorted.len() * 32);
    for nullifier in &sorted {
        buf.extend_from_slice(nullifier);
    }
    Identifier::new(hash_double(buf))
}

/// Whether the identity id an `IdentityCreateFromShieldedPool` will publish can be reproduced
/// OFFLINE, before (or after) the bundle is built, from the spent-note set alone.
///
/// The id is derived over the bundle's PUBLISHED nullifiers — every action's nullifier, padding
/// included. Orchard's `BundleType::DEFAULT` pads any bundle to `MIN_ACTIONS = 2`, and a padding
/// action carries a **randomly generated** dummy nullifier. So:
///
/// - `num_real_spends >= 2` — no padding is added, every published nullifier is the deterministic
///   nullifier of a real note, and the id is a pure function of the spent notes. It can be
///   predicted before building and RE-derived identically on a later retry.
/// - `num_real_spends < 2` — the bundle is padded and at least one published nullifier is fresh
///   randomness. The id is unpredictable beforehand and, critically, **not reproducible**: a retry
///   builds a different dummy and therefore a different identity id.
///
/// Any flow that must recognise "this identity is the one my earlier attempt created" — idempotent
/// claim recovery being the motivating case — MUST gate on this. When it returns `false` the
/// caller cannot derive an expected id and has to treat recovery as unreliable rather than
/// computing an id that will not match. Guarding on the *note count* is the cheapest correct check:
/// it needs no chain lookup and is decided before any proving work.
///
/// The corollary drives note layout: funding an address with two sub-target notes (instead of one
/// note covering the whole target) forces a later spend of that address to select BOTH — greedy
/// largest-first selection cannot stop after one note that does not cover the target — which keeps
/// the padding action, and its random nullifier, out of the bundle entirely.
pub fn shielded_identity_id_is_reproducible(num_real_spends: usize) -> bool {
    // Mirrors Orchard's `MIN_ACTIONS = 2`: at or above it, no padding action is appended.
    num_real_spends >= 2
}

/// Convenience wrapper around [`identity_id_from_nullifiers`] that extracts the nullifiers from a
/// slice of serialized Orchard actions. Shared by the SDK builder and the consensus re-derivation
/// check so both compute the id identically.
pub fn derive_identity_id_from_actions(actions: &[SerializedAction]) -> Identifier {
    let nullifiers: Vec<[u8; 32]> = actions.iter().map(|a| a.nullifier).collect();
    identity_id_from_nullifiers(&nullifiers)
}

// `IdentityCreateFromShieldedPool` funds the new identity from the shielded pool, not an asset
// lock, so it proves no asset lock (the default `None`).
impl OptionallyAssetLockProved for IdentityCreateFromShieldedPoolTransition {}

impl StateTransitionFieldTypes for IdentityCreateFromShieldedPoolTransition {
    fn signature_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn identifiers_property_paths() -> Vec<&'static str> {
        vec![]
    }

    fn binary_property_paths() -> Vec<&'static str> {
        vec![]
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
pub(crate) mod json_convertible_tests {
    use super::*;
    use crate::address_funds::PlatformAddress;
    use crate::identity::{KeyType, Purpose, SecurityLevel};
    use crate::shielded::SerializedAction;
    use crate::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
    use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
    use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
    use platform_value::{platform_value, BinaryData, Bytes32};
    use serde_json::json;

    fn fixture_action() -> SerializedAction {
        SerializedAction {
            nullifier: [0x11; 32],
            rk: [0x22; 32],
            cmx: [0x33; 32],
            encrypted_note: vec![0x44; 216],
            cv_net: [0x55; 32],
            spend_auth_sig: [0x66; 64],
        }
    }

    fn fixture_public_key() -> IdentityPublicKeyInCreation {
        IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: 7,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::HIGH,
            contract_bounds: None,
            read_only: true,
            data: BinaryData::new(vec![0x88; 33]),
            signature: BinaryData::new(vec![0x99; 65]),
        })
    }

    pub(crate) fn fixture() -> IdentityCreateFromShieldedPoolTransition {
        let actions = vec![fixture_action()];
        // identity_id is derived from the nullifiers, so the wire literal below is
        // deterministic for this fixture.
        let identity_id = derive_identity_id_from_actions(&actions);
        IdentityCreateFromShieldedPoolTransition::V0(IdentityCreateFromShieldedPoolTransitionV0 {
            public_keys: vec![fixture_public_key()],
            denomination: 10_000_000_000,
            actions,
            anchor: [0x77; 32],
            proof: vec![0x88; 192],
            binding_signature: [0x99; 64],
            send_to_address_on_creation_failure: PlatformAddress::P2pkh([0xa1; 20]),
            identity_id,
        })
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        // Sized-int fields whose JSON wire encoding loses size info:
        // `denomination` (u64), `publicKeys[].id` (u32 KeyID),
        // `publicKeys[].type`/`purpose`/`securityLevel` (u8 repr enums).
        // PlatformAddress → hex string in HR / 21 bytes non-HR; `identityId`
        // → base58 string (Identifier). The value-path assertion locks all
        // sized variants via explicit suffixes.
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "publicKeys": [{
                    "$formatVersion": "0",
                    "id": 7,
                    "type": 0,
                    "purpose": 0,
                    "securityLevel": 2,
                    "contractBounds": serde_json::Value::Null,
                    "readOnly": true,
                    "data": "iIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI",
                    "signature": "mZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZk=",
                }],
                "denomination": 10_000_000_000u64,
                "actions": [{
                    "nullifier": "ERERERERERERERERERERERERERERERERERERERERERE=",
                    "rk": "IiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI=",
                    "cmx": "MzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzMzM=",
                    "encryptedNote": "RERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERERE",
                    "cvNet": "VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVVU=",
                    "spendAuthSig": "ZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZg==",
                }],
                "anchor": "d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3c=",
                "proof": "iIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiIiI",
                "bindingSignature": "mZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmZmQ==",
                "sendToAddressOnCreationFailure": "00a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1",
                "identityId": "71Rhgo8fHdyu6FtYEGTHVfGQBB13vE8ehHZ2PWo3F8JS",
            })
        );
        let recovered =
            IdentityCreateFromShieldedPoolTransition::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let identity_id = match &original {
            IdentityCreateFromShieldedPoolTransition::V0(v0) => v0.identity_id,
        };
        let value = original.to_object().expect("to_object");
        // Explicit suffixes lock in sized variants: `denomination` u64,
        // `publicKeys[].id` u32, `type`/`purpose`/`securityLevel` u8.
        // PlatformAddress non-HR → 21-byte `Value::Bytes` (P2pkh type byte
        // 0x00); `identityId` → `Value::Identifier`.
        let mut address_bytes = vec![0x00];
        address_bytes.extend(vec![0xa1; 20]);
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "publicKeys": [{
                    "$formatVersion": "0",
                    "id": 7u32,
                    "type": 0u8,
                    "purpose": 0u8,
                    "securityLevel": 2u8,
                    "contractBounds": platform_value::Value::Null,
                    "readOnly": true,
                    "data": BinaryData::new(vec![0x88; 33]),
                    "signature": BinaryData::new(vec![0x99; 65]),
                }],
                "denomination": 10_000_000_000u64,
                "actions": [{
                    "nullifier": Bytes32::new([0x11; 32]),
                    "rk": Bytes32::new([0x22; 32]),
                    "cmx": Bytes32::new([0x33; 32]),
                    "encryptedNote": platform_value::Value::Bytes(vec![0x44; 216]),
                    "cvNet": Bytes32::new([0x55; 32]),
                    "spendAuthSig": platform_value::Value::Bytes(vec![0x66; 64]),
                }],
                "anchor": Bytes32::new([0x77; 32]),
                "proof": platform_value::Value::Bytes(vec![0x88; 192]),
                "bindingSignature": platform_value::Value::Bytes(vec![0x99; 64]),
                "sendToAddressOnCreationFailure": platform_value::Value::Bytes(address_bytes),
                "identityId": identity_id,
            })
        );
        let recovered =
            IdentityCreateFromShieldedPoolTransition::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use crate::shielded::SerializedAction;

    fn mk_action(nullifier_byte: u8) -> SerializedAction {
        SerializedAction {
            nullifier: [nullifier_byte; 32],
            rk: [2u8; 32],
            cmx: [3u8; 32],
            encrypted_note: vec![4u8; 216],
            cv_net: [5u8; 32],
            spend_auth_sig: [6u8; 64],
        }
    }

    #[test]
    fn id_derivation_is_order_independent() {
        let a = derive_identity_id_from_actions(&[mk_action(0x11), mk_action(0x22)]);
        let b = derive_identity_id_from_actions(&[mk_action(0x22), mk_action(0x11)]);
        assert_eq!(a, b, "id must not depend on action ordering");
    }

    #[test]
    fn id_derivation_differs_for_different_nullifiers() {
        let a = derive_identity_id_from_actions(&[mk_action(0x11)]);
        let b = derive_identity_id_from_actions(&[mk_action(0x12)]);
        assert_ne!(a, b);
    }

    #[test]
    fn serialization_round_trip() {
        use crate::identity::{KeyType, Purpose, SecurityLevel};
        use crate::state_transition::identity_create_from_shielded_pool_transition::v0::IdentityCreateFromShieldedPoolTransitionV0;
        use crate::state_transition::public_key_in_creation::v0::IdentityPublicKeyInCreationV0;
        use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
        use platform_value::BinaryData;

        let actions = vec![mk_action(0x11)];
        let identity_id = derive_identity_id_from_actions(&actions);
        let key = IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
            id: 0,
            key_type: KeyType::ECDSA_SECP256K1,
            purpose: Purpose::AUTHENTICATION,
            security_level: SecurityLevel::MASTER,
            contract_bounds: None,
            read_only: false,
            data: BinaryData::new(vec![0xAB; 33]),
            signature: BinaryData::new(vec![0xCD; 65]),
        });
        let transition: IdentityCreateFromShieldedPoolTransition =
            IdentityCreateFromShieldedPoolTransitionV0 {
                public_keys: vec![key],
                denomination: 10_000_000_000,
                actions,
                anchor: [7u8; 32],
                proof: vec![8u8; 100],
                binding_signature: [9u8; 64],
                send_to_address_on_creation_failure: crate::address_funds::PlatformAddress::P2pkh(
                    [0u8; 20],
                ),
                identity_id,
            }
            .into();

        let bytes = transition.serialize_to_bytes().expect("serialize");
        let restored = IdentityCreateFromShieldedPoolTransition::deserialize_from_bytes(&bytes)
            .expect("deserialize");
        assert_eq!(transition, restored);
    }
}
