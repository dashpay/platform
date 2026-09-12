mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use crate::prelude::Identifier;
use crate::shielded::SerializedAction;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Shielded pool to an existing identity's balance, version 0.
///
/// No platform signature: the Orchard proof and spend-auth signatures authorize
/// the spend, and `identity_id` plus `top_up_amount` are committed into the
/// Orchard binding sighash so the transition cannot be re-pointed.
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
pub struct IdentityTopUpFromShieldedPoolTransitionV0 {
    /// The existing identity whose balance receives the top-up
    pub identity_id: Identifier,
    /// Orchard actions (spend-output pairs)
    pub actions: Vec<SerializedAction>,
    /// Gross credits leaving the pool (the bundle's value balance). The identity
    /// is credited `top_up_amount - fee`.
    pub top_up_amount: u64,
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
    fn test_identity_top_up_from_shielded_pool_transition_v0_serialization_round_trip() {
        test_round_trip(IdentityTopUpFromShieldedPoolTransitionV0 {
            identity_id: Identifier::from([7u8; 32]),
            actions: vec![SerializedAction {
                nullifier: [1u8; 32],
                rk: [2u8; 32],
                cmx: [3u8; 32],
                encrypted_note: vec![4u8; 216],
                cv_net: [5u8; 32],
                spend_auth_sig: [6u8; 64],
            }],
            top_up_amount: 1000u64,
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
        });
    }
}
