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
                identity_id,
            }
            .into();

        let bytes = transition.serialize_to_bytes().expect("serialize");
        let restored = IdentityCreateFromShieldedPoolTransition::deserialize_from_bytes(&bytes)
            .expect("deserialize");
        assert_eq!(transition, restored);
    }
}
