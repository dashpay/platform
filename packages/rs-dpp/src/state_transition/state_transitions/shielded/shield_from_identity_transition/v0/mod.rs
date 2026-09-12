mod identity_signed;
mod state_transition_like;
mod state_transition_validation;
mod types;
pub(super) mod v0_methods;
mod version;

use crate::fee::Credits;
use crate::identity::KeyID;
use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use crate::shielded::SerializedAction;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Identity balance to shielded pool.
///
/// Every field before `signature_public_key_id` is covered by the identity signature,
/// which binds the Orchard bundle to this identity and nonce. The bundle itself is an
/// outputs-only Orchard bundle whose value balance equals `-amount` (value entering
/// the pool), verified exactly like `Shield`.
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
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[platform_serialize(unversioned)]
pub struct ShieldFromIdentityTransitionV0 {
    /// The identity whose balance funds the shield.
    pub identity_id: Identifier,
    /// Credits leaving the identity balance and entering the shielded pool
    /// (the absolute value of the bundle's value balance).
    pub amount: Credits,
    /// Orchard actions (spend-output pairs; spends disabled)
    pub actions: Vec<SerializedAction>,
    /// Sinsemilla root of the note commitment tree (Orchard Anchor)
    pub anchor: [u8; 32],
    /// Halo2 proof bytes
    pub proof: Vec<u8>,
    /// RedPallas binding signature
    pub binding_signature: [u8; 64],
    /// Identity nonce (replay protection)
    pub nonce: IdentityNonce,
    /// Fee multiplier
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
    use crate::state_transition::StateTransition;
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

    pub(crate) fn make_v0() -> ShieldFromIdentityTransitionV0 {
        ShieldFromIdentityTransitionV0 {
            identity_id: Identifier::from([7u8; 32]),
            amount: 1000u64,
            actions: vec![SerializedAction {
                nullifier: [1u8; 32],
                rk: [2u8; 32],
                cmx: [3u8; 32],
                encrypted_note: vec![4u8; 216],
                cv_net: [5u8; 32],
                spend_auth_sig: [6u8; 64],
            }],
            anchor: [7u8; 32],
            proof: vec![8u8; 100],
            binding_signature: [9u8; 64],
            nonce: 3,
            user_fee_increase: 1,
            signature_public_key_id: 2,
            signature: BinaryData::new(vec![0u8; 65]),
        }
    }

    #[test]
    fn test_shield_from_identity_transition_v0_serialization_round_trip() {
        test_round_trip(make_v0());
    }

    #[test]
    fn signable_bytes_change_when_any_bundle_field_changes() {
        let base: StateTransition = make_v0().into();
        let base_bytes = base.signable_bytes().expect("signable bytes");

        let mut amount = make_v0();
        amount.amount += 1;
        let mut anchor = make_v0();
        anchor.anchor[0] ^= 1;
        let mut proof = make_v0();
        proof.proof[0] ^= 1;
        let mut binding = make_v0();
        binding.binding_signature[0] ^= 1;
        let mut action = make_v0();
        action.actions[0].cmx[0] ^= 1;
        let mut nonce = make_v0();
        nonce.nonce += 1;
        let mut identity = make_v0();
        identity.identity_id = Identifier::from([8u8; 32]);

        for (name, mutated) in [
            ("amount", amount),
            ("anchor", anchor),
            ("proof", proof),
            ("binding_signature", binding),
            ("action", action),
            ("nonce", nonce),
            ("identity_id", identity),
        ] {
            let st: StateTransition = mutated.into();
            assert_ne!(
                st.signable_bytes().expect("signable bytes"),
                base_bytes,
                "{name} must be covered by the identity signature"
            );
        }

        let mut signature_only = make_v0();
        signature_only.signature = BinaryData::new(vec![1u8; 65]);
        signature_only.signature_public_key_id = 9;
        let st: StateTransition = signature_only.into();
        assert_eq!(
            st.signable_bytes().expect("signable bytes"),
            base_bytes,
            "signature fields must be excluded from the sig hash"
        );
    }
}
