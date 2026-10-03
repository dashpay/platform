mod proved;
mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v1_methods;
mod version;

use crate::address_funds::PlatformAddress;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::shielded::SerializedAction;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize, PlatformSignable,
};
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Version 1 of `ShieldFromAssetLock`: the fields of version 0, with an Orchard bundle proved
/// against the preimage that binds its kind tag and the asset lock funding it
/// ([`crate::shielded::shield_from_asset_lock_extra_sighash_data`]).
///
/// It is the only version protocol version 14 admits, and it is not admitted before. Version 0,
/// whose bundle binds nothing, is refused from protocol version 14 on its version byte when the
/// transition is decoded, before any proof is verified: it is not charged and its asset lock is
/// left unspent, so a version 0 transition still waiting when version 14 activates costs its
/// sender nothing. Accepting it would verify its unbound bundle against the bound preimage and
/// burn the proof-failure penalty from an honest lock.
#[cfg_attr(feature = "json-conversion", crate::serialization::json_safe_fields)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    PlatformSignable,
    PartialEq,
    DecodeUntrusted,
)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
pub struct ShieldFromAssetLockTransitionV1 {
    /// Asset lock proof from L1
    pub asset_lock_proof: AssetLockProof,
    /// Orchard actions (spend-output pairs)
    pub actions: Vec<SerializedAction>,
    /// Amount of credits flowing into the shielded pool from the asset lock.
    /// Must be > 0 and <= i64::MAX.
    pub value_balance: u64,
    /// Sinsemilla root of the note commitment tree (Orchard Anchor)
    pub anchor: [u8; 32],
    /// Halo2 proof bytes
    pub proof: Vec<u8>,
    /// RedPallas binding signature
    pub binding_signature: [u8; 64],
    /// Optional platform-address output that receives the asset-lock surplus; see
    /// [`ShieldFromAssetLockTransitionV0::surplus_output`](super::v0::ShieldFromAssetLockTransitionV0::surplus_output).
    /// Covered by the signable bytes like every field before `signature`.
    pub surplus_output: Option<PlatformAddress>,
    /// ECDSA signature over the signable bytes (excluded from sig hash)
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use crate::serialization::{PlatformDeserializableUntrusted, PlatformSerializable, Signable};
    use crate::state_transition::shield_from_asset_lock_transition::v0::ShieldFromAssetLockTransitionV0;
    use crate::state_transition::shield_from_asset_lock_transition::ShieldFromAssetLockTransition;
    use crate::state_transition::{FeatureVersioned, StateTransition, StateTransitionLike};
    use dashcore::OutPoint;

    fn make_v1() -> ShieldFromAssetLockTransitionV1 {
        ShieldFromAssetLockTransitionV1 {
            asset_lock_proof: AssetLockProof::Chain(ChainAssetLockProof {
                core_chain_locked_height: 100,
                out_point: OutPoint::from([11u8; 36]),
            }),
            actions: vec![SerializedAction {
                nullifier: [1u8; 32],
                rk: [2u8; 32],
                cmx: [3u8; 32],
                encrypted_note: vec![4u8; 216],
                cv_net: [5u8; 32],
                spend_auth_sig: [6u8; 64],
            }],
            value_balance: 1000u64,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
            surplus_output: Some(PlatformAddress::P2pkh([0x33; 20])),
            signature: BinaryData::new(vec![10u8; 65]),
        }
    }

    fn as_v0(v1: &ShieldFromAssetLockTransitionV1) -> ShieldFromAssetLockTransitionV0 {
        ShieldFromAssetLockTransitionV0 {
            asset_lock_proof: v1.asset_lock_proof.clone(),
            actions: v1.actions.clone(),
            value_balance: v1.value_balance,
            anchor: v1.anchor,
            proof: v1.proof.clone(),
            binding_signature: v1.binding_signature,
            surplus_output: v1.surplus_output,
            signature: v1.signature.clone(),
        }
    }

    #[test]
    fn should_round_trip_through_the_state_transition_bytes_as_version_1() {
        let transition: StateTransition = make_v1().into();
        let bytes = transition.serialize_to_bytes().expect("serialize");
        let decoded =
            StateTransition::deserialize_from_bytes_untrusted(&bytes).expect("deserialize");
        assert_eq!(decoded, transition);
        assert!(matches!(
            decoded,
            StateTransition::ShieldFromAssetLock(ShieldFromAssetLockTransition::V1(_))
        ));
    }

    #[test]
    fn should_report_version_1() {
        let v1 = make_v1();
        assert_eq!(v1.feature_version(), 1);
        assert_eq!(v1.state_transition_protocol_version(), 1);
        let transition: ShieldFromAssetLockTransition = v1.into();
        assert_eq!(transition.feature_version(), 1);
    }

    #[test]
    fn should_commit_the_signature_to_the_transition_version() {
        // The same fields signed as version 0 and as version 1 give different signable bytes, so
        // nobody can turn a signed version 1 into a version 0 (or back) without the lock's key.
        let v1 = make_v1();
        let as_v1: StateTransition = v1.clone().into();
        let as_v0: StateTransition = as_v0(&v1).into();
        assert_ne!(
            as_v1.signable_bytes().expect("signable"),
            as_v0.signable_bytes().expect("signable")
        );
    }
}
