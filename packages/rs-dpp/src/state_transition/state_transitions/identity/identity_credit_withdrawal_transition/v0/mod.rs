mod identity_signed;
#[cfg(feature = "state-transition-json-conversion")]
mod json_conversion;
mod state_transition_like;
mod types;
#[cfg(feature = "state-transition-value-conversion")]
mod value_conversion;
mod version;

use crate::serialization::json_safe_fields;
use bincode::{Decode, Encode};
use platform_serialization_derive::PlatformSignable;
use platform_value::BinaryData;
#[cfg(feature = "state-transition-serde-conversion")]
use serde::{Deserialize, Serialize};

use crate::prelude::{IdentityNonce, UserFeeIncrease};
use crate::{
    identity::{core_script::CoreScript, KeyID},
    prelude::Identifier,
    withdrawal::Pooling,
    ProtocolError,
};

#[json_safe_fields]
#[derive(Debug, Clone, Encode, Decode, PlatformSignable, PartialEq)]
#[cfg_attr(
    feature = "state-transition-serde-conversion",
    derive(Serialize, Deserialize),
    serde(rename_all = "camelCase")
)]
#[derive(Default)]
pub struct IdentityCreditWithdrawalTransitionV0 {
    pub identity_id: Identifier,
    pub amount: u64,
    pub core_fee_per_byte: u32,
    pub pooling: Pooling,
    pub output_script: CoreScript,
    pub nonce: IdentityNonce,
    pub user_fee_increase: UserFeeIncrease,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature_public_key_id: KeyID,
    #[platform_signable(exclude_from_sig_hash)]
    pub signature: BinaryData,
}

#[cfg(test)]
mod test {
    use crate::identity::core_script::CoreScript;
    use crate::identity::KeyID;
    use crate::prelude::{IdentityNonce, UserFeeIncrease};
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use crate::state_transition::identity_credit_withdrawal_transition::v0::Pooling;
    use crate::ProtocolError;
    use bincode::{Decode, Encode};
    use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
    use platform_value::{BinaryData, Identifier};
    use rand::Rng;
    use std::fmt::Debug;

    // Structure with 1 property
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_serialize(unversioned)]
    struct IdentityCreditWithdrawalTransitionV01 {
        pub identity_id: Identifier,
    }

    // Structure with 2 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_serialize(unversioned)]
    struct IdentityCreditWithdrawalTransitionV02 {
        pub identity_id: Identifier,
        pub amount: u64,
    }

    // Structure with 3 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_serialize(unversioned)]
    struct IdentityCreditWithdrawalTransitionV03 {
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
    }

    // Structure with 4 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_serialize(unversioned)]
    struct IdentityCreditWithdrawalTransitionV04 {
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
        pub pooling: Pooling,
    }

    // Structure with 5 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_serialize(unversioned)]
    struct IdentityCreditWithdrawalTransitionV05 {
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
        pub pooling: Pooling,
        pub output_script: CoreScript,
    }

    // Structure with 6 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_serialize(unversioned)]
    struct IdentityCreditWithdrawalTransitionV06 {
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
        pub pooling: Pooling,
        pub output_script: CoreScript,
        pub nonce: IdentityNonce,
    }

    // Structure with 7 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_serialize(unversioned)]
    struct IdentityCreditWithdrawalTransitionV07 {
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
        pub pooling: Pooling,
        pub output_script: CoreScript,
        pub nonce: IdentityNonce,
        pub user_fee_increase: UserFeeIncrease,
    }

    // Structure with 8 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_serialize(unversioned)]
    struct IdentityCreditWithdrawalTransitionV08 {
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
        pub pooling: Pooling,
        pub output_script: CoreScript,
        pub nonce: IdentityNonce,
        pub user_fee_increase: UserFeeIncrease,
        pub signature_public_key_id: KeyID,
    }

    // Structure with 9 properties
    #[derive(Debug, Clone, Encode, Decode, PlatformDeserialize, PlatformSerialize, PartialEq)]
    #[platform_serialize(unversioned)]
    struct IdentityCreditWithdrawalTransitionV09 {
        pub identity_id: Identifier,
        pub amount: u64,
        pub core_fee_per_byte: u32,
        pub pooling: Pooling,
        pub output_script: CoreScript,
        pub nonce: IdentityNonce,
        pub user_fee_increase: UserFeeIncrease,
        pub signature_public_key_id: KeyID,
        pub signature: BinaryData,
    }

    fn test_identity_credit_withdrawal_transition<
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
    fn test_identity_credit_withdrawal_transition_1() {
        let transition = IdentityCreditWithdrawalTransitionV01 {
            identity_id: Identifier::random(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_2() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransitionV02 {
            identity_id: Identifier::random(),
            amount: rng.gen(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_3() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransitionV03 {
            identity_id: Identifier::random(),
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_4() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransitionV04 {
            identity_id: Identifier::random(),
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
            pooling: Pooling::Standard,
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_5() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransitionV05 {
            identity_id: Identifier::random(),
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_6() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransitionV06 {
            identity_id: Identifier::random(),
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            nonce: rng.gen(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_7() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransitionV07 {
            identity_id: Identifier::random(),
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            nonce: rng.gen(),
            user_fee_increase: rng.gen(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_8() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransitionV08 {
            identity_id: Identifier::random(),
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            nonce: rng.gen(),
            user_fee_increase: rng.gen(),
            signature_public_key_id: rng.gen(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }

    #[test]
    fn test_identity_credit_withdrawal_transition_9() {
        let mut rng = rand::thread_rng();
        let transition = IdentityCreditWithdrawalTransitionV09 {
            identity_id: Identifier::random(),
            amount: rng.gen(),
            core_fee_per_byte: rng.gen(),
            pooling: Pooling::Standard,
            output_script: CoreScript::from_bytes((0..23).collect::<Vec<u8>>()),
            nonce: rng.gen(),
            user_fee_increase: rng.gen(),
            signature_public_key_id: rng.gen(),
            signature: [0; 65].to_vec().into(),
        };
        test_identity_credit_withdrawal_transition(transition);
    }
}
