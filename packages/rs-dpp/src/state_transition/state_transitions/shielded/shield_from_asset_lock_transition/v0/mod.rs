mod proved;
mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
use crate::prelude::UserFeeIncrease;
use crate::shielded::SerializedAction;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::BinaryData;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

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
    feature = "state-transition-serde-conversion",
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
    #[cfg_attr(
        feature = "state-transition-serde-conversion",
        serde(with = "crate::shielded::serde_bytes_64")
    )]
    pub binding_signature: [u8; 64],
    // TODO: remove user_fee_increase — the fee is implicitly the difference between
    // the asset lock value and value_balance, so no separate fee multiplier is needed.
    /// Fee multiplier
    pub user_fee_increase: UserFeeIncrease,
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
            user_fee_increase: 0u16,
            signature: BinaryData::new(vec![10u8; 65]),
        };

        test_round_trip(transition);
    }
}
