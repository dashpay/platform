mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use crate::address_funds::PlatformAddress;
use crate::shielded::SerializedAction;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
    PlatformSignable,
    PartialEq,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
pub struct UnshieldTransitionV0 {
    /// Address receiving the unshielded funds
    pub output_address: PlatformAddress,
    /// Orchard actions (spend-output pairs)
    pub actions: Vec<SerializedAction>,
    /// Total credits leaving the shielded pool (recipient amount + fee)
    pub unshielding_amount: u64,
    /// Sinsemilla root of the note commitment tree (Orchard Anchor)
    pub anchor: [u8; 32],
    /// Halo2 proof bytes
    pub proof: Vec<u8>,
    /// RedPallas binding signature
    pub binding_signature: [u8; 64],
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address_funds::PlatformAddress;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use std::fmt::Debug;

    fn test_round_trip<T: PlatformSerializable + PlatformDeserializable + Debug + PartialEq>(
        transition: T,
    ) where
        <T as PlatformSerializable>::Error: std::fmt::Debug,
    {
        let serialized = T::serialize_to_bytes(&transition).expect("expected to serialize");
        let deserialized =
            T::deserialize_from_bytes(serialized.as_slice()).expect("expected to deserialize");
        assert_eq!(transition, deserialized);
    }

    #[test]
    fn test_unshield_transition_v0_serialization_round_trip() {
        let transition = UnshieldTransitionV0 {
            output_address: PlatformAddress::P2pkh([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
            ]),
            actions: vec![SerializedAction {
                nullifier: [1u8; 32],
                rk: [2u8; 32],
                cmx: [3u8; 32],
                encrypted_note: vec![4u8; 692],
                cv_net: [5u8; 32],
                spend_auth_sig: [6u8; 64],
            }],
            unshielding_amount: 1000u64,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
        };

        test_round_trip(transition);
    }

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

    fn make_v0(actions: Vec<SerializedAction>) -> UnshieldTransitionV0 {
        UnshieldTransitionV0 {
            output_address: PlatformAddress::P2pkh([1u8; 20]),
            actions,
            unshielding_amount: 1000u64,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
        }
    }

    #[test]
    fn test_state_transition_type() {
        use crate::state_transition::{StateTransitionLike, StateTransitionType};
        let t = make_v0(vec![mk_action(1)]);
        assert_eq!(t.state_transition_type(), StateTransitionType::Unshield);
    }

    #[test]
    fn test_state_transition_protocol_version() {
        use crate::state_transition::StateTransitionLike;
        let t = make_v0(vec![mk_action(1)]);
        assert_eq!(t.state_transition_protocol_version(), 0);
    }

    #[test]
    fn test_state_transition_modified_data_ids_empty() {
        use crate::state_transition::StateTransitionLike;
        let t = make_v0(vec![mk_action(1)]);
        assert!(t.modified_data_ids().is_empty());
    }

    #[test]
    fn test_unique_identifiers_from_nullifiers_multiple() {
        use crate::state_transition::StateTransitionLike;
        let t = make_v0(vec![mk_action(0x11), mk_action(0x22), mk_action(0x33)]);
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 3);
        assert_eq!(ids[0], hex::encode([0x11u8; 32]));
        assert_eq!(ids[2], hex::encode([0x33u8; 32]));
    }

    #[test]
    fn test_unique_identifiers_empty_actions() {
        use crate::state_transition::StateTransitionLike;
        let t = make_v0(vec![]);
        assert!(t.unique_identifiers().is_empty());
    }

    #[test]
    fn test_feature_versioned() {
        use crate::state_transition::FeatureVersioned;
        assert_eq!(make_v0(vec![mk_action(1)]).feature_version(), 0);
    }
}
