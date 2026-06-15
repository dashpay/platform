mod proved;
mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use crate::address_funds::PlatformAddress;
use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::shielded::SerializedAction;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::BinaryData;
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
pub struct ShieldFromAssetLockTransitionV0 {
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
    /// Optional platform-address output that receives the asset-lock surplus
    /// (`asset_lock_value − value_balance − fee`, where `fee = compute_minimum_shielded_fee
    /// + asset_lock_base_cost`). When `None`, the surplus is instead added to the fee pools,
    /// but only up to `shielded_implicit_fee_cap` (otherwise the transition is rejected so a
    /// client cannot accidentally forfeit a large remainder). Placed before `signature` so it
    /// is covered by the signable bytes — the ECDSA signature commits to the surplus
    /// destination, which therefore cannot be redirected without invalidating the transition.
    pub surplus_output: Option<PlatformAddress>,
    /// ECDSA signature over the signable bytes (excluded from sig hash)
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::state_transition::asset_lock_proof::chain::ChainAssetLockProof;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use dashcore::OutPoint;
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
    fn test_shield_from_asset_lock_transition_v0_serialization_round_trip() {
        let chain_proof = ChainAssetLockProof {
            core_chain_locked_height: 100,
            out_point: OutPoint::from([11u8; 36]),
        };

        let transition = ShieldFromAssetLockTransitionV0 {
            asset_lock_proof: AssetLockProof::Chain(chain_proof),
            actions: vec![SerializedAction {
                nullifier: [1u8; 32],
                rk: [2u8; 32],
                cmx: [3u8; 32],
                encrypted_note: vec![4u8; 692],
                cv_net: [5u8; 32],
                spend_auth_sig: [6u8; 64],
            }],
            value_balance: 1000u64,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
            surplus_output: None,
            signature: BinaryData::new(vec![10u8; 65]),
        };

        test_round_trip(transition);
    }

    fn make_v0() -> ShieldFromAssetLockTransitionV0 {
        ShieldFromAssetLockTransitionV0 {
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
            surplus_output: None,
            signature: BinaryData::new(vec![10u8; 65]),
        }
    }

    #[test]
    fn test_state_transition_type() {
        use crate::state_transition::{StateTransitionLike, StateTransitionType};
        let t = make_v0();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::ShieldFromAssetLock
        );
    }

    #[test]
    fn test_state_transition_protocol_version() {
        use crate::state_transition::StateTransitionLike;
        let t = make_v0();
        assert_eq!(t.state_transition_protocol_version(), 0);
    }

    #[test]
    fn test_state_transition_modified_data_ids_empty() {
        use crate::state_transition::StateTransitionLike;
        let t = make_v0();
        assert!(t.modified_data_ids().is_empty());
    }

    #[test]
    fn test_state_transition_unique_identifiers_is_base64_asset_lock() {
        use crate::state_transition::StateTransitionLike;
        let t = make_v0();
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
        // Should be valid non-empty base64 (not the error fallback)
        assert!(!ids[0].is_empty());
    }

    #[test]
    fn test_signature_setters() {
        use crate::state_transition::StateTransitionSingleSigned;
        let mut t = make_v0();
        assert_eq!(t.signature().as_slice(), &vec![10u8; 65][..]);
        t.set_signature(BinaryData::new(vec![99u8; 65]));
        assert_eq!(t.signature().as_slice(), &vec![99u8; 65][..]);
        t.set_signature_bytes(vec![1u8; 65]);
        assert_eq!(t.signature().as_slice(), &vec![1u8; 65][..]);
    }

    #[test]
    fn test_asset_lock_proof_accessor_and_setter() {
        use crate::identity::state_transition::AssetLockProved;
        let mut t = make_v0();
        match t.asset_lock_proof() {
            AssetLockProof::Chain(_) => {}
            _ => panic!("expected Chain"),
        }
        t.set_asset_lock_proof(AssetLockProof::Chain(ChainAssetLockProof {
            core_chain_locked_height: 999,
            out_point: OutPoint::from([0xAB_u8; 36]),
        }))
        .unwrap();
        if let AssetLockProof::Chain(c) = t.asset_lock_proof() {
            assert_eq!(c.core_chain_locked_height, 999);
        } else {
            panic!("expected Chain");
        }
    }

    #[test]
    fn test_feature_versioned() {
        use crate::state_transition::FeatureVersioned;
        assert_eq!(make_v0().feature_version(), 0);
    }

    #[test]
    fn test_round_trip_with_surplus_output() {
        // The optional surplus_output field must serialize/deserialize identically. It sits before
        // the sig-excluded `signature` field, so it is part of the signable bytes (the asset-lock
        // ECDSA signature commits to it — a missing/desynced field would change the wire layout).
        let mut transition = make_v0();
        transition.surplus_output = Some(crate::address_funds::PlatformAddress::P2pkh([0x44; 20]));
        test_round_trip(transition);
    }

    #[test]
    fn test_signable_bytes_commit_to_surplus_output() {
        // The asset-lock ECDSA signature must commit to the surplus destination: redirecting the
        // surplus to a different address (or toggling it on/off) MUST change the signable bytes, so a
        // valid signature cannot be replayed against a different `surplus_output`.
        //
        // The `PlatformSignable` derive excludes fields by NAME (only `signature`, via
        // `exclude_from_sig_hash`) — NOT by position. So the invariant under test is precisely
        // "surplus_output is NOT excluded from the sighash", which we confirm by observing the
        // signable bytes differ. The `signature` field itself is held constant across all variants
        // below, so any difference is attributable solely to `surplus_output`.
        use crate::serialization::Signable;

        let addr_a = crate::address_funds::PlatformAddress::P2pkh([0x44; 20]);
        let addr_b = crate::address_funds::PlatformAddress::P2pkh([0x55; 20]);

        let mut none_variant = make_v0();
        none_variant.surplus_output = None;

        let mut some_a = make_v0();
        some_a.surplus_output = Some(addr_a);

        let mut some_b = make_v0();
        some_b.surplus_output = Some(addr_b);

        let none_bytes = none_variant
            .signable_bytes()
            .expect("should compute signable bytes for None variant");
        let some_a_bytes = some_a
            .signable_bytes()
            .expect("should compute signable bytes for Some(addr_a) variant");
        let some_b_bytes = some_b
            .signable_bytes()
            .expect("should compute signable bytes for Some(addr_b) variant");

        // None vs Some(addr): toggling the surplus output on changes the sighash.
        assert_ne!(
            none_bytes, some_a_bytes,
            "signable bytes must differ between surplus_output None and Some(addr) \
             (signature commits to the presence of a surplus destination)"
        );

        // Some(addrA) vs Some(addrB): redirecting to a different address changes the sighash.
        assert_ne!(
            some_a_bytes, some_b_bytes,
            "signable bytes must differ between two distinct surplus_output addresses \
             (signature commits to the surplus destination, not just its presence)"
        );

        // Sanity: an unrelated change to the excluded `signature` field must NOT change the signable
        // bytes — this isolates the difference above to `surplus_output` rather than to incidental
        // field churn, and pins that `signature` IS excluded from the sighash.
        let mut some_a_other_sig = some_a.clone();
        some_a_other_sig.signature = BinaryData::new(vec![0xEE; 65]);
        assert_eq!(
            some_a_bytes,
            some_a_other_sig
                .signable_bytes()
                .expect("should compute signable bytes after mutating the excluded signature"),
            "mutating the sig-excluded `signature` field must not change the signable bytes"
        );
    }
}
