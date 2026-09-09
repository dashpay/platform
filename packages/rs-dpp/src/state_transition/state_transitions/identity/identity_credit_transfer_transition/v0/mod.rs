mod identity_signed;
mod state_transition_like;
mod types;
pub(super) mod v0_methods;
mod version;

use crate::identity::KeyID;
#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;

use crate::prelude::{Identifier, IdentityNonce, UserFeeIncrease};

use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize, PlatformSignable};
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
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
#[derive(Default)]
pub struct IdentityCreditTransferTransitionV0 {
    // Own ST fields
    pub identity_id: Identifier,
    pub recipient_id: Identifier,
    pub amount: u64,
    pub nonce: IdentityNonce,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

#[cfg(test)]
mod test {

    use crate::serialization::{PlatformDeserializable, PlatformSerializable};

    use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use platform_value::Identifier;
    use rand::Rng;
    use std::fmt::Debug;

    fn test_identity_credit_transfer_transition<
        T: PlatformSerializable + PlatformDeserializable + Debug + PartialEq,
    >(
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
    fn test_identity_credit_transfer_transition1() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditTransferTransitionV0 {
            identity_id: Identifier::random(),
            recipient_id: Identifier::random(),
            amount: rng.gen(),
            nonce: 1,
            user_fee_increase: 0,
            signature_public_key_id: rng.gen(),
            signature: [0; 65].to_vec().into(),
        };

        test_identity_credit_transfer_transition(transition);
    }

    fn make_transfer_v0() -> IdentityCreditTransferTransitionV0 {
        IdentityCreditTransferTransitionV0 {
            identity_id: Identifier::random(),
            recipient_id: Identifier::random(),
            amount: 100_000,
            nonce: 42,
            user_fee_increase: 5,
            signature_public_key_id: 1,
            signature: [0u8; 65].to_vec().into(),
        }
    }

    #[test]
    fn test_state_transition_like_v0() {
        use crate::state_transition::{
            StateTransitionLike, StateTransitionOwned, StateTransitionType,
        };
        let transition = make_transfer_v0();
        assert_eq!(
            transition.state_transition_type(),
            StateTransitionType::IdentityCreditTransfer
        );
        assert_eq!(transition.state_transition_protocol_version(), 0);
        assert_eq!(transition.owner_id(), transition.identity_id);
        let modified = transition.modified_data_ids();
        assert_eq!(modified.len(), 2);
        assert_eq!(modified[0], transition.identity_id);
        assert_eq!(modified[1], transition.recipient_id);
    }

    #[test]
    fn test_unique_identifiers_v0() {
        use crate::state_transition::StateTransitionLike;
        let transition = make_transfer_v0();
        let ids = transition.unique_identifiers();
        assert_eq!(ids.len(), 1);
        assert!(!ids[0].is_empty());
    }

    #[test]
    fn test_identity_signed_v0() {
        use crate::identity::{Purpose, SecurityLevel};
        use crate::state_transition::StateTransitionIdentitySigned;
        let mut transition = make_transfer_v0();
        assert_eq!(transition.signature_public_key_id(), 1);
        transition.set_signature_public_key_id(99);
        assert_eq!(transition.signature_public_key_id(), 99);
        let security = transition.security_level_requirement(Purpose::TRANSFER);
        assert_eq!(security, vec![SecurityLevel::CRITICAL]);
        let purpose = transition.purpose_requirement();
        assert_eq!(purpose, vec![Purpose::TRANSFER]);
    }

    #[test]
    fn test_user_fee_increase_v0() {
        use crate::state_transition::StateTransitionHasUserFeeIncrease;
        let mut transition = make_transfer_v0();
        assert_eq!(transition.user_fee_increase(), 5);
        transition.set_user_fee_increase(10);
        assert_eq!(transition.user_fee_increase(), 10);
    }

    #[test]
    fn test_single_signed_v0() {
        use crate::state_transition::StateTransitionSingleSigned;
        use platform_value::BinaryData;
        let mut transition = make_transfer_v0();
        assert_eq!(transition.signature().len(), 65);
        let new_sig = BinaryData::new(vec![1, 2, 3]);
        transition.set_signature(new_sig.clone());
        assert_eq!(transition.signature(), &new_sig);
        transition.set_signature_bytes(vec![4, 5, 6]);
        assert_eq!(transition.signature().as_slice(), &[4, 5, 6]);
    }

    #[test]
    fn test_into_state_transition_v0() {
        use crate::state_transition::StateTransition;
        let transition = make_transfer_v0();
        let st: StateTransition = transition.into();
        match st {
            StateTransition::IdentityCreditTransfer(_) => {}
            _ => panic!("expected IdentityCreditTransfer"),
        }
    }

    // Legacy `StateTransitionValueConvert` round-trip tests on the V0
    // inner struct deleted in Phase D step 9. The canonical
    // `JsonConvertible` / `ValueConvertible` round-trip is exercised via
    // the outer enum derive — these tested methods that no longer exist.

    #[test]
    fn test_default_v0() {
        let transition = IdentityCreditTransferTransitionV0::default();
        assert_eq!(transition.amount, 0);
        assert_eq!(transition.nonce, 0);
        assert_eq!(transition.user_fee_increase, 0);
    }

    #[test]
    fn test_modified_data_ids_and_unique_identifiers() {
        use crate::state_transition::StateTransitionLike;
        let t = make_transfer_v0();
        let modified = t.modified_data_ids();
        assert_eq!(modified.len(), 2);
        assert_eq!(modified[0], t.identity_id);
        assert_eq!(modified[1], t.recipient_id);
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
    }
}
