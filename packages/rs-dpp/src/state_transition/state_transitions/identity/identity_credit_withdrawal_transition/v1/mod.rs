mod identity_signed;
#[cfg(feature = "json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
mod v0_methods;
#[cfg(feature = "value-conversion")]
mod value_conversion;
mod version;

#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::BinaryData;
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::{
    identity::{core_script::CoreScript, KeyID},
    prelude::Identifier,
    withdrawal::Pooling,
    ProtocolError,
};

#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[derive(Debug, Clone, Encode, Decode, PlatformSignable, PartialEq)]
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct IdentityCreditWithdrawalTransitionV1 {
    pub identity_id: Identifier,
    pub amount: u64,
    pub core_fee_per_byte: u32,
    pub pooling: Pooling,
    /// If the send to output script is None, then we send the withdrawal to the address set by core
    pub output_script: Option<CoreScript>,
    pub nonce: IdentityNonce,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state_transition::{
        StateTransitionHasUserFeeIncrease, StateTransitionIdentitySigned, StateTransitionLike,
        StateTransitionOwned, StateTransitionSingleSigned, StateTransitionType,
        StateTransitionValueConvert,
    };
    use platform_value::BinaryData;

    fn make_withdrawal_v1() -> IdentityCreditWithdrawalTransitionV1 {
        IdentityCreditWithdrawalTransitionV1 {
            identity_id: Identifier::random(),
            amount: 200_000,
            core_fee_per_byte: 1,
            pooling: Pooling::Never,
            output_script: Some(CoreScript::from_bytes((0..23).collect::<Vec<u8>>())),
            nonce: 10,
            user_fee_increase: 3,
            signature_public_key_id: 2,
            signature: [0u8; 65].to_vec().into(),
        }
    }

    fn make_withdrawal_v1_no_script() -> IdentityCreditWithdrawalTransitionV1 {
        IdentityCreditWithdrawalTransitionV1 {
            identity_id: Identifier::random(),
            amount: 200_000,
            core_fee_per_byte: 1,
            pooling: Pooling::Standard,
            output_script: None,
            nonce: 10,
            user_fee_increase: 0,
            signature_public_key_id: 2,
            signature: [0u8; 65].to_vec().into(),
        }
    }

    #[test]
    fn test_default() {
        let t = IdentityCreditWithdrawalTransitionV1::default();
        assert_eq!(t.amount, 0);
        assert!(t.output_script.is_none());
    }

    #[test]
    fn test_state_transition_like() {
        let t = make_withdrawal_v1();
        assert_eq!(
            t.state_transition_type(),
            StateTransitionType::IdentityCreditWithdrawal
        );
        assert_eq!(t.state_transition_protocol_version(), 0);
        assert_eq!(t.modified_data_ids(), vec![t.identity_id]);
        assert_eq!(t.owner_id(), t.identity_id);
    }

    #[test]
    fn test_unique_identifiers() {
        let t = make_withdrawal_v1();
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
        assert!(!ids[0].is_empty());
    }

    #[test]
    fn test_identity_signed() {
        use crate::identity::{Purpose, SecurityLevel};
        let mut t = make_withdrawal_v1();
        assert_eq!(t.signature_public_key_id(), 2);
        t.set_signature_public_key_id(55);
        assert_eq!(t.signature_public_key_id(), 55);
        let security = t.security_level_requirement(Purpose::TRANSFER);
        assert_eq!(security, vec![SecurityLevel::CRITICAL]);
        let purpose = t.purpose_requirement();
        assert!(purpose.contains(&Purpose::TRANSFER));
        assert!(purpose.contains(&Purpose::OWNER));
    }

    #[test]
    fn test_user_fee_increase() {
        let mut t = make_withdrawal_v1();
        assert_eq!(t.user_fee_increase(), 3);
        t.set_user_fee_increase(100);
        assert_eq!(t.user_fee_increase(), 100);
    }

    #[test]
    fn test_single_signed() {
        let mut t = make_withdrawal_v1();
        assert_eq!(t.signature().len(), 65);
        t.set_signature(BinaryData::new(vec![1, 2, 3]));
        assert_eq!(t.signature().as_slice(), &[1, 2, 3]);
        t.set_signature_bytes(vec![4, 5]);
        assert_eq!(t.signature().as_slice(), &[4, 5]);
    }

    #[test]
    fn test_into_state_transition() {
        use crate::state_transition::StateTransition;
        let t = make_withdrawal_v1();
        let st: StateTransition = t.into();
        match st {
            StateTransition::IdentityCreditWithdrawal(_) => {}
            _ => panic!("expected IdentityCreditWithdrawal"),
        }
    }

    #[test]
    fn test_value_conversion_roundtrip_with_script() {
        use crate::version::LATEST_PLATFORM_VERSION;
        let t = make_withdrawal_v1();
        let obj = t.to_object(false).expect("to_object should work");
        let restored =
            IdentityCreditWithdrawalTransitionV1::from_object(obj, LATEST_PLATFORM_VERSION)
                .expect("from_object should work");
        assert_eq!(t, restored);
    }

    #[test]
    fn test_value_conversion_roundtrip_without_script() {
        use crate::version::LATEST_PLATFORM_VERSION;
        let t = make_withdrawal_v1_no_script();
        let obj = t.to_object(false).expect("to_object should work");
        let restored =
            IdentityCreditWithdrawalTransitionV1::from_object(obj, LATEST_PLATFORM_VERSION)
                .expect("from_object should work");
        assert_eq!(t, restored);
    }

    #[test]
    fn test_from_value_map() {
        use crate::version::LATEST_PLATFORM_VERSION;
        let t = make_withdrawal_v1();
        let obj = t.to_object(false).expect("to_object should work");
        let map = obj.into_btree_string_map().expect("should be map");
        let restored =
            IdentityCreditWithdrawalTransitionV1::from_value_map(map, LATEST_PLATFORM_VERSION)
                .expect("should work");
        assert_eq!(t, restored);
    }

    #[test]
    fn test_to_cleaned_object() {
        let t = make_withdrawal_v1();
        let obj = t.to_cleaned_object(false).expect("should work");
        assert!(obj.is_map());
    }

    #[test]
    fn test_to_canonical_cleaned_object() {
        let t = make_withdrawal_v1();
        let obj = t.to_canonical_cleaned_object(false).expect("should work");
        assert!(obj.is_map());
    }

    #[test]
    fn test_to_object_skip_signature() {
        let t = make_withdrawal_v1();
        let obj = t.to_object(true).expect("should work");
        let map = obj.into_btree_string_map().expect("should be map");
        assert!(!map.contains_key("signature"));
    }

    #[test]
    fn test_to_cleaned_object_skip_signature() {
        let t = make_withdrawal_v1();
        let obj = t.to_cleaned_object(true).expect("should work");
        let map = obj.into_btree_string_map().expect("should be map");
        assert!(!map.contains_key("signature"));
    }

    #[test]
    fn test_to_canonical_cleaned_object_skip_signature() {
        let t = make_withdrawal_v1();
        let obj = t.to_canonical_cleaned_object(true).expect("should work");
        let map = obj.into_btree_string_map().expect("should be map");
        assert!(!map.contains_key("signature"));
    }

    #[test]
    fn test_unique_identifier_includes_nonce() {
        let t = make_withdrawal_v1();
        let ids = t.unique_identifiers();
        assert_eq!(ids.len(), 1);
        // nonce=10 in hex is "a"
        assert!(ids[0].ends_with("-a"), "got: {}", ids[0]);
    }

    #[test]
    fn test_into_state_transition_wraps_v1() {
        use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
        use crate::state_transition::StateTransition;
        let t = make_withdrawal_v1();
        let outer: IdentityCreditWithdrawalTransition = t.clone().into();
        assert!(matches!(outer, IdentityCreditWithdrawalTransition::V1(_)));
        let st: StateTransition = t.into();
        assert!(matches!(st, StateTransition::IdentityCreditWithdrawal(_)));
    }

    #[test]
    fn test_owner_id_v1() {
        let t = make_withdrawal_v1();
        assert_eq!(t.owner_id(), t.identity_id);
    }

    #[test]
    fn test_value_conversion_script_none_roundtrip_map() {
        use crate::version::LATEST_PLATFORM_VERSION;
        let t = make_withdrawal_v1_no_script();
        let obj = t.to_object(false).expect("to_object");
        let map = obj.into_btree_string_map().expect("should be map");
        let restored =
            IdentityCreditWithdrawalTransitionV1::from_value_map(map, LATEST_PLATFORM_VERSION)
                .expect("from_value_map");
        assert!(restored.output_script.is_none());
        assert_eq!(t, restored);
    }
}
