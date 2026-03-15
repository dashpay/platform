//! Comprehensive tests for identity state transitions.
//!
//! Tests cover: accessors, StateTransitionLike, StateTransitionIdentitySigned,
//! StateTransitionSingleSigned, StateTransitionOwned, StateTransitionHasUserFeeIncrease,
//! FeatureVersioned, default_versioned constructors, and StateTransitionFieldTypes.

#[cfg(test)]
mod tests {
    use platform_value::{BinaryData, Identifier};
    use platform_version::version::PlatformVersion;

    use crate::identity::core_script::CoreScript;
    use crate::identity::{Purpose, SecurityLevel};
    use crate::state_transition::{
        FeatureVersioned, StateTransitionFieldTypes, StateTransitionHasUserFeeIncrease,
        StateTransitionIdentitySigned, StateTransitionLike, StateTransitionOwned,
        StateTransitionSingleSigned, StateTransitionType,
    };
    use crate::withdrawal::Pooling;

    // =========================================================================
    // Credit Transfer Transition
    // =========================================================================
    mod credit_transfer {
        use super::*;
        use crate::state_transition::identity_credit_transfer_transition::{
            accessors::IdentityCreditTransferTransitionAccessorsV0,
            v0::IdentityCreditTransferTransitionV0, IdentityCreditTransferTransition,
        };

        fn sample_v0() -> IdentityCreditTransferTransitionV0 {
            IdentityCreditTransferTransitionV0 {
                identity_id: Identifier::new([1u8; 32]),
                recipient_id: Identifier::new([2u8; 32]),
                amount: 1000,
                nonce: 42,
                user_fee_increase: 5,
                signature_public_key_id: 7,
                signature: BinaryData::new(vec![0xAA; 65]),
            }
        }

        fn sample_enum() -> IdentityCreditTransferTransition {
            IdentityCreditTransferTransition::V0(sample_v0())
        }

        // -- V0 struct-level tests --

        #[test]
        fn v0_state_transition_protocol_version() {
            assert_eq!(sample_v0().state_transition_protocol_version(), 0);
        }

        #[test]
        fn v0_state_transition_type() {
            assert_eq!(
                sample_v0().state_transition_type(),
                StateTransitionType::IdentityCreditTransfer
            );
        }

        #[test]
        fn v0_modified_data_ids_contains_both_ids() {
            let t = sample_v0();
            let ids = t.modified_data_ids();
            assert_eq!(ids.len(), 2);
            assert_eq!(ids[0], Identifier::new([1u8; 32]));
            assert_eq!(ids[1], Identifier::new([2u8; 32]));
        }

        #[test]
        fn v0_unique_identifiers_format() {
            let t = sample_v0();
            let ids = t.unique_identifiers();
            assert_eq!(ids.len(), 1);
            assert!(ids[0].contains("-2a")); // 42 == 0x2a
        }

        #[test]
        fn v0_user_fee_increase() {
            let mut t = sample_v0();
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 5);
            t.set_user_fee_increase(10);
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 10);
        }

        #[test]
        fn v0_signature_accessors() {
            let mut t = sample_v0();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xAA; 65])
            );

            let new_sig = BinaryData::new(vec![0xBB; 65]);
            StateTransitionSingleSigned::set_signature(&mut t, new_sig.clone());
            assert_eq!(StateTransitionSingleSigned::signature(&t), &new_sig);

            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0xCC; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xCC; 65])
            );
        }

        #[test]
        fn v0_owner_id() {
            assert_eq!(sample_v0().owner_id(), Identifier::new([1u8; 32]));
        }

        #[test]
        fn v0_identity_signed() {
            let mut t = sample_v0();
            assert_eq!(t.signature_public_key_id(), 7);
            t.set_signature_public_key_id(99);
            assert_eq!(t.signature_public_key_id(), 99);
            assert_eq!(
                t.security_level_requirement(Purpose::AUTHENTICATION),
                vec![SecurityLevel::CRITICAL]
            );
            assert_eq!(t.purpose_requirement(), vec![Purpose::TRANSFER]);
        }

        #[test]
        fn v0_feature_version() {
            assert_eq!(sample_v0().feature_version(), 0);
        }

        // -- Enum-level tests --

        #[test]
        fn enum_delegates_state_transition_like() {
            let t = sample_enum();
            assert_eq!(
                t.state_transition_type(),
                StateTransitionType::IdentityCreditTransfer
            );
            assert_eq!(t.state_transition_protocol_version(), 0);
            assert_eq!(t.modified_data_ids().len(), 2);
            assert!(!t.unique_identifiers().is_empty());
        }

        #[test]
        fn enum_delegates_user_fee_increase() {
            let mut t = sample_enum();
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 5);
            t.set_user_fee_increase(20);
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 20);
        }

        #[test]
        fn enum_delegates_signature() {
            let mut t = sample_enum();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xAA; 65])
            );
            StateTransitionSingleSigned::set_signature(&mut t, BinaryData::new(vec![0xDD; 65]));
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xDD; 65])
            );
            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0xEE; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xEE; 65])
            );
        }

        #[test]
        fn enum_delegates_owner_id() {
            assert_eq!(sample_enum().owner_id(), Identifier::new([1u8; 32]));
        }

        #[test]
        fn enum_delegates_identity_signed() {
            let mut t = sample_enum();
            assert_eq!(t.signature_public_key_id(), 7);
            t.set_signature_public_key_id(42);
            assert_eq!(t.signature_public_key_id(), 42);
            assert_eq!(
                t.security_level_requirement(Purpose::TRANSFER),
                vec![SecurityLevel::CRITICAL]
            );
            assert_eq!(t.purpose_requirement(), vec![Purpose::TRANSFER]);
        }

        #[test]
        fn enum_feature_version() {
            assert_eq!(sample_enum().feature_version(), 0);
        }

        // -- Accessors --

        #[test]
        fn accessors_identity_id() {
            let mut t = sample_enum();
            assert_eq!(t.identity_id(), Identifier::new([1u8; 32]));
            t.set_identity_id(Identifier::new([9u8; 32]));
            assert_eq!(t.identity_id(), Identifier::new([9u8; 32]));
        }

        #[test]
        fn accessors_recipient_id() {
            let mut t = sample_enum();
            assert_eq!(t.recipient_id(), Identifier::new([2u8; 32]));
            t.set_recipient_id(Identifier::new([8u8; 32]));
            assert_eq!(t.recipient_id(), Identifier::new([8u8; 32]));
        }

        #[test]
        fn accessors_amount() {
            let mut t = sample_enum();
            assert_eq!(t.amount(), 1000);
            t.set_amount(5000);
            assert_eq!(t.amount(), 5000);
        }

        #[test]
        fn accessors_nonce() {
            let mut t = sample_enum();
            assert_eq!(t.nonce(), 42);
            t.set_nonce(100);
            assert_eq!(t.nonce(), 100);
        }

        // -- default_versioned --

        #[test]
        fn default_versioned_v0() {
            let pv = PlatformVersion::latest();
            let t = IdentityCreditTransferTransition::default_versioned(pv)
                .expect("should create default");
            assert!(matches!(t, IdentityCreditTransferTransition::V0(_)));
        }

        // -- StateTransitionFieldTypes --

        #[test]
        fn field_types() {
            let sig_paths = IdentityCreditTransferTransition::signature_property_paths();
            assert!(sig_paths.contains(&"signature"));

            let id_paths = IdentityCreditTransferTransition::identifiers_property_paths();
            assert!(id_paths.contains(&"identityId"));
            assert!(id_paths.contains(&"recipientId"));

            let bin_paths = IdentityCreditTransferTransition::binary_property_paths();
            assert!(bin_paths.is_empty());
        }
    }

    // =========================================================================
    // Credit Withdrawal Transition
    // =========================================================================
    mod credit_withdrawal {
        use super::*;
        use crate::state_transition::identity_credit_withdrawal_transition::{
            accessors::IdentityCreditWithdrawalTransitionAccessorsV0,
            v0::IdentityCreditWithdrawalTransitionV0, v1::IdentityCreditWithdrawalTransitionV1,
            IdentityCreditWithdrawalTransition,
        };

        fn sample_v0() -> IdentityCreditWithdrawalTransitionV0 {
            IdentityCreditWithdrawalTransitionV0 {
                identity_id: Identifier::new([3u8; 32]),
                amount: 2000,
                core_fee_per_byte: 1,
                pooling: Pooling::Standard,
                output_script: CoreScript::from_bytes(vec![0x76, 0xa9, 0x14]),
                nonce: 55,
                user_fee_increase: 3,
                signature_public_key_id: 11,
                signature: BinaryData::new(vec![0xCC; 65]),
            }
        }

        fn sample_v1() -> IdentityCreditWithdrawalTransitionV1 {
            IdentityCreditWithdrawalTransitionV1 {
                identity_id: Identifier::new([4u8; 32]),
                amount: 3000,
                core_fee_per_byte: 2,
                pooling: Pooling::Standard,
                output_script: Some(CoreScript::from_bytes(vec![0x76, 0xa9])),
                nonce: 66,
                user_fee_increase: 7,
                signature_public_key_id: 13,
                signature: BinaryData::new(vec![0xDD; 65]),
            }
        }

        fn sample_enum_v0() -> IdentityCreditWithdrawalTransition {
            IdentityCreditWithdrawalTransition::V0(sample_v0())
        }

        fn sample_enum_v1() -> IdentityCreditWithdrawalTransition {
            IdentityCreditWithdrawalTransition::V1(sample_v1())
        }

        // -- V0 struct-level --

        #[test]
        fn v0_state_transition_type() {
            assert_eq!(
                sample_v0().state_transition_type(),
                StateTransitionType::IdentityCreditWithdrawal
            );
        }

        #[test]
        fn v0_state_transition_protocol_version() {
            assert_eq!(sample_v0().state_transition_protocol_version(), 0);
        }

        #[test]
        fn v0_modified_data_ids() {
            let t = sample_v0();
            let ids = t.modified_data_ids();
            assert_eq!(ids.len(), 1);
            assert_eq!(ids[0], Identifier::new([3u8; 32]));
        }

        #[test]
        fn v0_unique_identifiers() {
            let t = sample_v0();
            let ids = t.unique_identifiers();
            assert_eq!(ids.len(), 1);
            assert!(ids[0].contains("-37")); // 55 == 0x37
        }

        #[test]
        fn v0_user_fee_increase() {
            let mut t = sample_v0();
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 3);
            t.set_user_fee_increase(15);
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 15);
        }

        #[test]
        fn v0_signature_methods() {
            let mut t = sample_v0();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xCC; 65])
            );
            StateTransitionSingleSigned::set_signature(&mut t, BinaryData::new(vec![0x11; 65]));
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x11; 65])
            );
            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0x22; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x22; 65])
            );
        }

        #[test]
        fn v0_owner_id() {
            assert_eq!(sample_v0().owner_id(), Identifier::new([3u8; 32]));
        }

        #[test]
        fn v0_identity_signed() {
            let mut t = sample_v0();
            assert_eq!(t.signature_public_key_id(), 11);
            t.set_signature_public_key_id(88);
            assert_eq!(t.signature_public_key_id(), 88);
            assert_eq!(
                t.security_level_requirement(Purpose::TRANSFER),
                vec![SecurityLevel::CRITICAL]
            );
            assert_eq!(t.purpose_requirement(), vec![Purpose::TRANSFER]);
        }

        #[test]
        fn v0_feature_version() {
            assert_eq!(sample_v0().feature_version(), 0);
        }

        // -- V1 struct-level --

        #[test]
        fn v1_state_transition_type() {
            assert_eq!(
                sample_v1().state_transition_type(),
                StateTransitionType::IdentityCreditWithdrawal
            );
        }

        #[test]
        fn v1_state_transition_protocol_version() {
            assert_eq!(sample_v1().state_transition_protocol_version(), 0);
        }

        #[test]
        fn v1_modified_data_ids() {
            let t = sample_v1();
            let ids = t.modified_data_ids();
            assert_eq!(ids.len(), 1);
            assert_eq!(ids[0], Identifier::new([4u8; 32]));
        }

        #[test]
        fn v1_unique_identifiers() {
            let t = sample_v1();
            let ids = t.unique_identifiers();
            assert_eq!(ids.len(), 1);
            assert!(ids[0].contains("-42")); // 66 == 0x42
        }

        #[test]
        fn v1_user_fee_increase() {
            let mut t = sample_v1();
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 7);
            t.set_user_fee_increase(25);
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 25);
        }

        #[test]
        fn v1_signature_methods() {
            let mut t = sample_v1();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xDD; 65])
            );
            StateTransitionSingleSigned::set_signature(&mut t, BinaryData::new(vec![0x33; 65]));
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x33; 65])
            );
            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0x44; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x44; 65])
            );
        }

        #[test]
        fn v1_owner_id() {
            assert_eq!(sample_v1().owner_id(), Identifier::new([4u8; 32]));
        }

        #[test]
        fn v1_identity_signed() {
            let mut t = sample_v1();
            assert_eq!(t.signature_public_key_id(), 13);
            t.set_signature_public_key_id(77);
            assert_eq!(t.signature_public_key_id(), 77);
            assert_eq!(
                t.security_level_requirement(Purpose::TRANSFER),
                vec![SecurityLevel::CRITICAL]
            );
            assert_eq!(
                t.purpose_requirement(),
                vec![Purpose::TRANSFER, Purpose::OWNER]
            );
        }

        #[test]
        fn v1_feature_version() {
            assert_eq!(sample_v1().feature_version(), 1);
        }

        // -- Enum-level tests (both V0 and V1 variants) --

        #[test]
        fn enum_v0_state_transition_like() {
            let t = sample_enum_v0();
            assert_eq!(
                t.state_transition_type(),
                StateTransitionType::IdentityCreditWithdrawal
            );
            assert_eq!(t.state_transition_protocol_version(), 0);
            assert_eq!(t.modified_data_ids().len(), 1);
            assert!(!t.unique_identifiers().is_empty());
        }

        #[test]
        fn enum_v1_state_transition_like() {
            let t = sample_enum_v1();
            assert_eq!(
                t.state_transition_type(),
                StateTransitionType::IdentityCreditWithdrawal
            );
            assert_eq!(t.state_transition_protocol_version(), 0);
            assert_eq!(t.modified_data_ids().len(), 1);
            assert!(!t.unique_identifiers().is_empty());
        }

        #[test]
        fn enum_v0_user_fee_increase() {
            let mut t = sample_enum_v0();
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 3);
            t.set_user_fee_increase(50);
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 50);
        }

        #[test]
        fn enum_v1_user_fee_increase() {
            let mut t = sample_enum_v1();
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 7);
            t.set_user_fee_increase(60);
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 60);
        }

        #[test]
        fn enum_v0_signature_delegates() {
            let mut t = sample_enum_v0();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xCC; 65])
            );
            StateTransitionSingleSigned::set_signature(&mut t, BinaryData::new(vec![0x55; 65]));
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x55; 65])
            );
            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0x66; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x66; 65])
            );
        }

        #[test]
        fn enum_v1_signature_delegates() {
            let mut t = sample_enum_v1();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xDD; 65])
            );
            StateTransitionSingleSigned::set_signature(&mut t, BinaryData::new(vec![0x77; 65]));
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x77; 65])
            );
            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0x88; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x88; 65])
            );
        }

        #[test]
        fn enum_v0_owner_id() {
            assert_eq!(sample_enum_v0().owner_id(), Identifier::new([3u8; 32]));
        }

        #[test]
        fn enum_v1_owner_id() {
            assert_eq!(sample_enum_v1().owner_id(), Identifier::new([4u8; 32]));
        }

        #[test]
        fn enum_v0_identity_signed() {
            let mut t = sample_enum_v0();
            assert_eq!(t.signature_public_key_id(), 11);
            t.set_signature_public_key_id(22);
            assert_eq!(t.signature_public_key_id(), 22);
            assert_eq!(
                t.security_level_requirement(Purpose::TRANSFER),
                vec![SecurityLevel::CRITICAL]
            );
            assert_eq!(t.purpose_requirement(), vec![Purpose::TRANSFER]);
        }

        #[test]
        fn enum_v1_identity_signed() {
            let mut t = sample_enum_v1();
            assert_eq!(t.signature_public_key_id(), 13);
            t.set_signature_public_key_id(33);
            assert_eq!(t.signature_public_key_id(), 33);
            assert_eq!(
                t.security_level_requirement(Purpose::TRANSFER),
                vec![SecurityLevel::CRITICAL]
            );
            assert_eq!(
                t.purpose_requirement(),
                vec![Purpose::TRANSFER, Purpose::OWNER]
            );
        }

        #[test]
        fn enum_feature_version() {
            assert_eq!(sample_enum_v0().feature_version(), 0);
            assert_eq!(sample_enum_v1().feature_version(), 1);
        }

        // -- Accessors (exercising both V0 and V1 through the enum) --

        #[test]
        fn accessors_identity_id_v0() {
            let mut t = sample_enum_v0();
            assert_eq!(t.identity_id(), Identifier::new([3u8; 32]));
            t.set_identity_id(Identifier::new([10u8; 32]));
            assert_eq!(t.identity_id(), Identifier::new([10u8; 32]));
        }

        #[test]
        fn accessors_identity_id_v1() {
            let mut t = sample_enum_v1();
            assert_eq!(t.identity_id(), Identifier::new([4u8; 32]));
            t.set_identity_id(Identifier::new([11u8; 32]));
            assert_eq!(t.identity_id(), Identifier::new([11u8; 32]));
        }

        #[test]
        fn accessors_amount_v0() {
            let mut t = sample_enum_v0();
            assert_eq!(t.amount(), 2000);
            t.set_amount(8000);
            assert_eq!(t.amount(), 8000);
        }

        #[test]
        fn accessors_amount_v1() {
            let mut t = sample_enum_v1();
            assert_eq!(t.amount(), 3000);
            t.set_amount(9000);
            assert_eq!(t.amount(), 9000);
        }

        #[test]
        fn accessors_nonce_v0() {
            let mut t = sample_enum_v0();
            assert_eq!(t.nonce(), 55);
            t.set_nonce(200);
            assert_eq!(t.nonce(), 200);
        }

        #[test]
        fn accessors_nonce_v1() {
            let mut t = sample_enum_v1();
            assert_eq!(t.nonce(), 66);
            t.set_nonce(300);
            assert_eq!(t.nonce(), 300);
        }

        #[test]
        fn accessors_pooling_v0() {
            let mut t = sample_enum_v0();
            assert_eq!(t.pooling(), Pooling::Standard);
            t.set_pooling(Pooling::Standard);
            assert_eq!(t.pooling(), Pooling::Standard);
        }

        #[test]
        fn accessors_pooling_v1() {
            let mut t = sample_enum_v1();
            assert_eq!(t.pooling(), Pooling::Standard);
            t.set_pooling(Pooling::Standard);
            assert_eq!(t.pooling(), Pooling::Standard);
        }

        #[test]
        fn accessors_core_fee_per_byte_v0() {
            let mut t = sample_enum_v0();
            assert_eq!(t.core_fee_per_byte(), 1);
            t.set_core_fee_per_byte(5);
            assert_eq!(t.core_fee_per_byte(), 5);
        }

        #[test]
        fn accessors_core_fee_per_byte_v1() {
            let mut t = sample_enum_v1();
            assert_eq!(t.core_fee_per_byte(), 2);
            t.set_core_fee_per_byte(10);
            assert_eq!(t.core_fee_per_byte(), 10);
        }

        #[test]
        fn accessors_output_script_v0() {
            let mut t = sample_enum_v0();
            // V0 always returns Some
            assert!(t.output_script().is_some());
            let new_script = CoreScript::from_bytes(vec![0xab, 0xcd]);
            t.set_output_script(Some(new_script.clone()));
            assert_eq!(t.output_script(), Some(new_script));

            // Setting None on V0 does nothing (preserves the old value)
            let before = t.output_script();
            t.set_output_script(None);
            assert_eq!(t.output_script(), before);
        }

        #[test]
        fn accessors_output_script_v1() {
            let mut t = sample_enum_v1();
            assert!(t.output_script().is_some());
            t.set_output_script(None);
            assert!(t.output_script().is_none());
            let new_script = CoreScript::from_bytes(vec![0xef]);
            t.set_output_script(Some(new_script.clone()));
            assert_eq!(t.output_script(), Some(new_script));
        }

        // -- default_versioned --

        #[test]
        fn default_versioned_latest() {
            let pv = PlatformVersion::latest();
            let t = IdentityCreditWithdrawalTransition::default_versioned(pv)
                .expect("should create default");
            // The latest version should succeed; we just verify it's one of the valid variants
            match t {
                IdentityCreditWithdrawalTransition::V0(_) => {}
                IdentityCreditWithdrawalTransition::V1(_) => {}
            }
        }

        // -- StateTransitionFieldTypes --

        #[test]
        fn field_types() {
            let sig_paths = IdentityCreditWithdrawalTransition::signature_property_paths();
            assert!(sig_paths.contains(&"signature"));
            assert!(sig_paths.contains(&"signaturePublicKeyId"));

            let id_paths = IdentityCreditWithdrawalTransition::identifiers_property_paths();
            assert!(id_paths.contains(&"identityId"));

            let bin_paths = IdentityCreditWithdrawalTransition::binary_property_paths();
            assert!(bin_paths.contains(&"signature"));
            assert!(bin_paths.contains(&"outputScript"));
        }
    }

    // =========================================================================
    // Identity Update Transition
    // =========================================================================
    mod identity_update {
        use super::*;
        use crate::state_transition::identity_update_transition::{
            accessors::IdentityUpdateTransitionAccessorsV0, v0::IdentityUpdateTransitionV0,
            IdentityUpdateTransition,
        };
        use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;

        fn sample_v0() -> IdentityUpdateTransitionV0 {
            IdentityUpdateTransitionV0 {
                identity_id: Identifier::new([5u8; 32]),
                revision: 3,
                nonce: 77,
                add_public_keys: vec![],
                disable_public_keys: vec![1, 2],
                user_fee_increase: 4,
                signature_public_key_id: 0,
                signature: BinaryData::new(vec![0xEE; 65]),
            }
        }

        fn sample_enum() -> IdentityUpdateTransition {
            IdentityUpdateTransition::V0(sample_v0())
        }

        // -- V0 struct-level --

        #[test]
        fn v0_state_transition_type() {
            assert_eq!(
                sample_v0().state_transition_type(),
                StateTransitionType::IdentityUpdate
            );
        }

        #[test]
        fn v0_state_transition_protocol_version() {
            assert_eq!(sample_v0().state_transition_protocol_version(), 0);
        }

        #[test]
        fn v0_modified_data_ids() {
            let ids = sample_v0().modified_data_ids();
            assert_eq!(ids.len(), 1);
            assert_eq!(ids[0], Identifier::new([5u8; 32]));
        }

        #[test]
        fn v0_unique_identifiers() {
            let t = sample_v0();
            let ids = t.unique_identifiers();
            assert_eq!(ids.len(), 1);
            assert!(ids[0].contains("-4d")); // 77 == 0x4d
        }

        #[test]
        fn v0_user_fee_increase() {
            let mut t = sample_v0();
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 4);
            t.set_user_fee_increase(12);
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 12);
        }

        #[test]
        fn v0_signature_methods() {
            let mut t = sample_v0();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xEE; 65])
            );
            StateTransitionSingleSigned::set_signature(&mut t, BinaryData::new(vec![0xFF; 65]));
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xFF; 65])
            );
            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0x01; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x01; 65])
            );
        }

        #[test]
        fn v0_owner_id() {
            assert_eq!(sample_v0().owner_id(), Identifier::new([5u8; 32]));
        }

        #[test]
        fn v0_identity_signed() {
            let mut t = sample_v0();
            assert_eq!(t.signature_public_key_id(), 0);
            t.set_signature_public_key_id(44);
            assert_eq!(t.signature_public_key_id(), 44);
            assert_eq!(
                t.security_level_requirement(Purpose::AUTHENTICATION),
                vec![SecurityLevel::MASTER]
            );
        }

        #[test]
        fn v0_feature_version() {
            assert_eq!(sample_v0().feature_version(), 0);
        }

        // -- V0 accessors via trait --

        #[test]
        fn v0_accessors_identity_id() {
            let mut t = sample_v0();
            assert_eq!(
                IdentityUpdateTransitionAccessorsV0::identity_id(&t),
                Identifier::new([5u8; 32])
            );
            t.set_identity_id(Identifier::new([6u8; 32]));
            assert_eq!(
                IdentityUpdateTransitionAccessorsV0::identity_id(&t),
                Identifier::new([6u8; 32])
            );
        }

        #[test]
        fn v0_accessors_revision() {
            let mut t = sample_v0();
            assert_eq!(IdentityUpdateTransitionAccessorsV0::revision(&t), 3);
            t.set_revision(10);
            assert_eq!(IdentityUpdateTransitionAccessorsV0::revision(&t), 10);
        }

        #[test]
        fn v0_accessors_nonce() {
            let mut t = sample_v0();
            assert_eq!(IdentityUpdateTransitionAccessorsV0::nonce(&t), 77);
            t.set_nonce(200);
            assert_eq!(IdentityUpdateTransitionAccessorsV0::nonce(&t), 200);
        }

        #[test]
        fn v0_accessors_public_keys_to_add() {
            let mut t = sample_v0();
            assert!(t.public_keys_to_add().is_empty());
            // We cannot easily construct IdentityPublicKeyInCreation here without heavy deps,
            // but we can set an empty vec and verify
            let empty: Vec<IdentityPublicKeyInCreation> = vec![];
            t.set_public_keys_to_add(empty);
            assert!(t.public_keys_to_add().is_empty());
        }

        #[test]
        fn v0_accessors_disable_public_keys() {
            let mut t = sample_v0();
            assert_eq!(t.public_key_ids_to_disable(), &[1, 2]);
            t.set_public_key_ids_to_disable(vec![5, 6, 7]);
            assert_eq!(t.public_key_ids_to_disable(), &[5, 6, 7]);
        }

        // -- Enum-level tests --

        #[test]
        fn enum_delegates_state_transition_like() {
            let t = sample_enum();
            assert_eq!(
                t.state_transition_type(),
                StateTransitionType::IdentityUpdate
            );
            assert_eq!(t.state_transition_protocol_version(), 0);
            assert_eq!(t.modified_data_ids().len(), 1);
            assert!(!t.unique_identifiers().is_empty());
        }

        #[test]
        fn enum_delegates_user_fee_increase() {
            let mut t = sample_enum();
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 4);
            t.set_user_fee_increase(30);
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 30);
        }

        #[test]
        fn enum_delegates_signature() {
            let mut t = sample_enum();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xEE; 65])
            );
            StateTransitionSingleSigned::set_signature(&mut t, BinaryData::new(vec![0x02; 65]));
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x02; 65])
            );
            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0x03; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x03; 65])
            );
        }

        #[test]
        fn enum_delegates_owner_id() {
            assert_eq!(sample_enum().owner_id(), Identifier::new([5u8; 32]));
        }

        #[test]
        fn enum_delegates_identity_signed() {
            let mut t = sample_enum();
            assert_eq!(t.signature_public_key_id(), 0);
            t.set_signature_public_key_id(55);
            assert_eq!(t.signature_public_key_id(), 55);
            assert_eq!(
                t.security_level_requirement(Purpose::AUTHENTICATION),
                vec![SecurityLevel::MASTER]
            );
        }

        #[test]
        fn enum_feature_version() {
            assert_eq!(sample_enum().feature_version(), 0);
        }

        // -- Enum-level accessors --

        #[test]
        fn enum_accessors_identity_id() {
            let mut t = sample_enum();
            assert_eq!(t.identity_id(), Identifier::new([5u8; 32]));
            t.set_identity_id(Identifier::new([7u8; 32]));
            assert_eq!(t.identity_id(), Identifier::new([7u8; 32]));
        }

        #[test]
        fn enum_accessors_revision() {
            let mut t = sample_enum();
            assert_eq!(t.revision(), 3);
            t.set_revision(20);
            assert_eq!(t.revision(), 20);
        }

        #[test]
        fn enum_accessors_nonce() {
            let mut t = sample_enum();
            assert_eq!(t.nonce(), 77);
            t.set_nonce(400);
            assert_eq!(t.nonce(), 400);
        }

        #[test]
        fn enum_accessors_public_keys_to_add() {
            let t = sample_enum();
            assert!(t.public_keys_to_add().is_empty());
        }

        #[test]
        fn enum_accessors_public_keys_to_add_mut() {
            let mut t = sample_enum();
            let keys = t.public_keys_to_add_mut();
            assert!(keys.is_empty());
        }

        #[test]
        fn enum_accessors_disable_public_keys() {
            let mut t = sample_enum();
            assert_eq!(t.public_key_ids_to_disable(), &[1, 2]);
            t.set_public_key_ids_to_disable(vec![10]);
            assert_eq!(t.public_key_ids_to_disable(), &[10]);
        }

        // -- default_versioned --

        #[test]
        fn default_versioned_v0() {
            let pv = PlatformVersion::latest();
            let t = IdentityUpdateTransition::default_versioned(pv).expect("should create default");
            assert!(matches!(t, IdentityUpdateTransition::V0(_)));
        }

        // -- StateTransitionFieldTypes --

        #[test]
        fn field_types() {
            let sig_paths = IdentityUpdateTransition::signature_property_paths();
            assert!(sig_paths.contains(&"signature"));
            assert!(sig_paths.contains(&"signaturePublicKeyId"));
            assert!(sig_paths.contains(&"addPublicKeys[].signature"));

            let id_paths = IdentityUpdateTransition::identifiers_property_paths();
            assert!(id_paths.contains(&"identityId"));

            let bin_paths = IdentityUpdateTransition::binary_property_paths();
            assert!(bin_paths.contains(&"signature"));
            assert!(bin_paths.contains(&"addPublicKeys[].signature"));
        }
    }

    // =========================================================================
    // Identity TopUp Transition
    // =========================================================================
    mod identity_topup {
        use super::*;
        use crate::identity::state_transition::asset_lock_proof::AssetLockProof;
        use crate::state_transition::identity_topup_transition::{
            accessors::IdentityTopUpTransitionAccessorsV0, v0::IdentityTopUpTransitionV0,
            IdentityTopUpTransition,
        };

        fn sample_v0() -> IdentityTopUpTransitionV0 {
            IdentityTopUpTransitionV0 {
                asset_lock_proof: AssetLockProof::default(),
                identity_id: Identifier::new([6u8; 32]),
                user_fee_increase: 2,
                signature: BinaryData::new(vec![0xAB; 65]),
            }
        }

        fn sample_enum() -> IdentityTopUpTransition {
            IdentityTopUpTransition::V0(sample_v0())
        }

        // -- V0 struct-level --

        #[test]
        fn v0_state_transition_type() {
            assert_eq!(
                sample_v0().state_transition_type(),
                StateTransitionType::IdentityTopUp
            );
        }

        #[test]
        fn v0_state_transition_protocol_version() {
            assert_eq!(sample_v0().state_transition_protocol_version(), 0);
        }

        #[test]
        fn v0_modified_data_ids() {
            let ids = sample_v0().modified_data_ids();
            assert_eq!(ids.len(), 1);
            assert_eq!(ids[0], Identifier::new([6u8; 32]));
        }

        #[test]
        fn v0_unique_identifiers_returns_one() {
            let t = sample_v0();
            let ids = t.unique_identifiers();
            assert_eq!(ids.len(), 1);
        }

        #[test]
        fn v0_user_fee_increase() {
            let mut t = sample_v0();
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 2);
            t.set_user_fee_increase(8);
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 8);
        }

        #[test]
        fn v0_signature_methods() {
            let mut t = sample_v0();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xAB; 65])
            );
            StateTransitionSingleSigned::set_signature(&mut t, BinaryData::new(vec![0xCD; 65]));
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xCD; 65])
            );
            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0xEF; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xEF; 65])
            );
        }

        #[test]
        fn v0_owner_id() {
            assert_eq!(sample_v0().owner_id(), Identifier::new([6u8; 32]));
        }

        #[test]
        fn v0_feature_version() {
            assert_eq!(sample_v0().feature_version(), 0);
        }

        // -- V0 accessors --

        #[test]
        fn v0_accessors_identity_id() {
            let mut t = sample_v0();
            assert_eq!(t.identity_id(), &Identifier::new([6u8; 32]));
            t.set_identity_id(Identifier::new([7u8; 32]));
            assert_eq!(t.identity_id(), &Identifier::new([7u8; 32]));
        }

        // -- Enum-level tests --

        #[test]
        fn enum_delegates_state_transition_like() {
            let t = sample_enum();
            assert_eq!(
                t.state_transition_type(),
                StateTransitionType::IdentityTopUp
            );
            assert_eq!(t.state_transition_protocol_version(), 0);
            assert_eq!(t.modified_data_ids().len(), 1);
            assert!(!t.unique_identifiers().is_empty());
        }

        #[test]
        fn enum_delegates_user_fee_increase() {
            let mut t = sample_enum();
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 2);
            t.set_user_fee_increase(40);
            assert_eq!(StateTransitionHasUserFeeIncrease::user_fee_increase(&t), 40);
        }

        #[test]
        fn enum_delegates_signature() {
            let mut t = sample_enum();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xAB; 65])
            );
            StateTransitionSingleSigned::set_signature(&mut t, BinaryData::new(vec![0x99; 65]));
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x99; 65])
            );
            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0x88; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x88; 65])
            );
        }

        #[test]
        fn enum_delegates_owner_id() {
            assert_eq!(sample_enum().owner_id(), Identifier::new([6u8; 32]));
        }

        #[test]
        fn enum_feature_version() {
            assert_eq!(sample_enum().feature_version(), 0);
        }

        // -- Enum-level accessors --

        #[test]
        fn enum_accessors_identity_id() {
            let mut t = sample_enum();
            assert_eq!(t.identity_id(), &Identifier::new([6u8; 32]));
            t.set_identity_id(Identifier::new([8u8; 32]));
            assert_eq!(t.identity_id(), &Identifier::new([8u8; 32]));
        }

        // -- default_versioned --

        #[test]
        fn default_versioned_v0() {
            let pv = PlatformVersion::latest();
            let t = IdentityTopUpTransition::default_versioned(pv).expect("should create default");
            assert!(matches!(t, IdentityTopUpTransition::V0(_)));
        }

        // -- StateTransitionFieldTypes --

        #[test]
        fn field_types() {
            let sig_paths = IdentityTopUpTransition::signature_property_paths();
            assert!(sig_paths.contains(&"signature"));

            let id_paths = IdentityTopUpTransition::identifiers_property_paths();
            assert!(id_paths.contains(&"identityId"));

            let bin_paths = IdentityTopUpTransition::binary_property_paths();
            assert!(bin_paths.is_empty());
        }
    }

    // =========================================================================
    // Masternode Vote Transition
    // =========================================================================
    mod masternode_vote {
        use super::*;
        use crate::state_transition::masternode_vote_transition::{
            accessors::MasternodeVoteTransitionAccessorsV0, v0::MasternodeVoteTransitionV0,
            MasternodeVoteTransition,
        };
        use crate::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
        use crate::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
        use crate::voting::vote_polls::VotePoll;
        use crate::voting::votes::resource_vote::v0::ResourceVoteV0;
        use crate::voting::votes::resource_vote::ResourceVote;
        use crate::voting::votes::Vote;

        fn sample_vote() -> Vote {
            Vote::ResourceVote(ResourceVote::V0(ResourceVoteV0 {
                vote_poll: VotePoll::ContestedDocumentResourceVotePoll(
                    ContestedDocumentResourceVotePoll {
                        contract_id: Identifier::default(),
                        document_type_name: "testDoc".to_string(),
                        index_name: "idx".to_string(),
                        index_values: vec![],
                    },
                ),
                resource_vote_choice: ResourceVoteChoice::TowardsIdentity(Identifier::new(
                    [20u8; 32],
                )),
            }))
        }

        fn sample_v0() -> MasternodeVoteTransitionV0 {
            MasternodeVoteTransitionV0 {
                pro_tx_hash: Identifier::new([7u8; 32]),
                voter_identity_id: Identifier::new([8u8; 32]),
                vote: sample_vote(),
                nonce: 88,
                signature_public_key_id: 5,
                signature: BinaryData::new(vec![0xBB; 65]),
            }
        }

        fn sample_enum() -> MasternodeVoteTransition {
            MasternodeVoteTransition::V0(sample_v0())
        }

        // -- V0 struct-level --

        #[test]
        fn v0_state_transition_type() {
            assert_eq!(
                sample_v0().state_transition_type(),
                StateTransitionType::MasternodeVote
            );
        }

        #[test]
        fn v0_state_transition_protocol_version() {
            assert_eq!(sample_v0().state_transition_protocol_version(), 0);
        }

        #[test]
        fn v0_modified_data_ids() {
            let ids = sample_v0().modified_data_ids();
            assert_eq!(ids.len(), 1);
            assert_eq!(ids[0], Identifier::new([8u8; 32])); // voter_identity_id
        }

        #[test]
        fn v0_unique_identifiers() {
            let t = sample_v0();
            let ids = t.unique_identifiers();
            assert_eq!(ids.len(), 1);
            assert!(ids[0].contains("-58")); // 88 == 0x58
        }

        #[test]
        fn v0_signature_methods() {
            let mut t = sample_v0();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xBB; 65])
            );
            StateTransitionSingleSigned::set_signature(&mut t, BinaryData::new(vec![0x44; 65]));
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x44; 65])
            );
            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0x55; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x55; 65])
            );
        }

        #[test]
        fn v0_owner_id() {
            assert_eq!(sample_v0().owner_id(), Identifier::new([8u8; 32]));
        }

        #[test]
        fn v0_identity_signed() {
            let mut t = sample_v0();
            assert_eq!(t.signature_public_key_id(), 5);
            t.set_signature_public_key_id(33);
            assert_eq!(t.signature_public_key_id(), 33);
            assert_eq!(
                t.security_level_requirement(Purpose::VOTING),
                vec![
                    SecurityLevel::CRITICAL,
                    SecurityLevel::HIGH,
                    SecurityLevel::MEDIUM
                ]
            );
            assert_eq!(t.purpose_requirement(), vec![Purpose::VOTING]);
        }

        #[test]
        fn v0_feature_version() {
            assert_eq!(sample_v0().feature_version(), 0);
        }

        // -- Enum-level tests --

        #[test]
        fn enum_delegates_state_transition_like() {
            let t = sample_enum();
            assert_eq!(
                t.state_transition_type(),
                StateTransitionType::MasternodeVote
            );
            assert_eq!(t.state_transition_protocol_version(), 0);
            assert_eq!(t.modified_data_ids().len(), 1);
            assert!(!t.unique_identifiers().is_empty());
        }

        #[test]
        fn enum_delegates_signature() {
            let mut t = sample_enum();
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0xBB; 65])
            );
            StateTransitionSingleSigned::set_signature(&mut t, BinaryData::new(vec![0x66; 65]));
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x66; 65])
            );
            StateTransitionSingleSigned::set_signature_bytes(&mut t, vec![0x77; 65]);
            assert_eq!(
                StateTransitionSingleSigned::signature(&t),
                &BinaryData::new(vec![0x77; 65])
            );
        }

        #[test]
        fn enum_delegates_owner_id() {
            assert_eq!(sample_enum().owner_id(), Identifier::new([8u8; 32]));
        }

        #[test]
        fn enum_delegates_identity_signed() {
            let mut t = sample_enum();
            assert_eq!(t.signature_public_key_id(), 5);
            t.set_signature_public_key_id(66);
            assert_eq!(t.signature_public_key_id(), 66);
            assert_eq!(
                t.security_level_requirement(Purpose::VOTING),
                vec![
                    SecurityLevel::CRITICAL,
                    SecurityLevel::HIGH,
                    SecurityLevel::MEDIUM
                ]
            );
            assert_eq!(t.purpose_requirement(), vec![Purpose::VOTING]);
        }

        #[test]
        fn enum_feature_version() {
            assert_eq!(sample_enum().feature_version(), 0);
        }

        // -- Accessors (enum level) --

        #[test]
        fn accessors_pro_tx_hash() {
            let mut t = sample_enum();
            assert_eq!(t.pro_tx_hash(), Identifier::new([7u8; 32]));
            t.set_pro_tx_hash(Identifier::new([15u8; 32]));
            assert_eq!(t.pro_tx_hash(), Identifier::new([15u8; 32]));
        }

        #[test]
        fn accessors_voter_identity_id() {
            let mut t = sample_enum();
            assert_eq!(t.voter_identity_id(), Identifier::new([8u8; 32]));
            t.set_voter_identity_id(Identifier::new([16u8; 32]));
            assert_eq!(t.voter_identity_id(), Identifier::new([16u8; 32]));
        }

        #[test]
        fn accessors_vote() {
            let mut t = sample_enum();
            let _vote_ref = t.vote();
            // Verify it's a ResourceVote
            assert!(matches!(t.vote(), Vote::ResourceVote(_)));

            let new_vote = sample_vote();
            t.set_vote(new_vote);
            assert!(matches!(t.vote(), Vote::ResourceVote(_)));
        }

        #[test]
        fn accessors_vote_owned() {
            let t = sample_enum();
            let vote = t.vote_owned();
            assert!(matches!(vote, Vote::ResourceVote(_)));
        }

        #[test]
        fn accessors_nonce() {
            let t = sample_enum();
            assert_eq!(t.nonce(), 88);
        }

        // -- default_versioned --

        #[test]
        fn default_versioned_v0() {
            let pv = PlatformVersion::latest();
            let t = MasternodeVoteTransition::default_versioned(pv).expect("should create default");
            assert!(matches!(t, MasternodeVoteTransition::V0(_)));
        }

        // -- StateTransitionFieldTypes --

        #[test]
        fn field_types() {
            let sig_paths = MasternodeVoteTransition::signature_property_paths();
            assert!(sig_paths.contains(&"signature"));

            let id_paths = MasternodeVoteTransition::identifiers_property_paths();
            assert!(id_paths.contains(&"proTxHash"));

            let bin_paths = MasternodeVoteTransition::binary_property_paths();
            assert!(bin_paths.is_empty());
        }
    }

    // =========================================================================
    // Public Key In Creation — accessors
    // =========================================================================
    mod public_key_in_creation {
        use super::*;
        use crate::identity::contract_bounds::ContractBounds;
        use crate::identity::KeyType;
        use crate::state_transition::public_key_in_creation::{
            accessors::{
                IdentityPublicKeyInCreationV0Getters, IdentityPublicKeyInCreationV0Setters,
            },
            v0::IdentityPublicKeyInCreationV0,
            IdentityPublicKeyInCreation,
        };

        fn sample() -> IdentityPublicKeyInCreation {
            IdentityPublicKeyInCreation::V0(IdentityPublicKeyInCreationV0 {
                id: 1,
                key_type: KeyType::ECDSA_SECP256K1,
                purpose: Purpose::AUTHENTICATION,
                security_level: SecurityLevel::MASTER,
                read_only: false,
                data: BinaryData::new(vec![0x02; 33]),
                signature: BinaryData::new(vec![0xAA; 65]),
                contract_bounds: None,
            })
        }

        #[test]
        fn getters() {
            let pk = sample();
            assert_eq!(pk.id(), 1);
            assert_eq!(pk.key_type(), KeyType::ECDSA_SECP256K1);
            assert_eq!(pk.purpose(), Purpose::AUTHENTICATION);
            assert_eq!(pk.security_level(), SecurityLevel::MASTER);
            assert!(!pk.read_only());
            assert_eq!(pk.data(), &BinaryData::new(vec![0x02; 33]));
            assert_eq!(pk.signature(), &BinaryData::new(vec![0xAA; 65]));
            assert!(pk.contract_bounds().is_none());
        }

        #[test]
        fn setters() {
            let mut pk = sample();
            pk.set_id(42);
            assert_eq!(pk.id(), 42);

            pk.set_type(KeyType::BLS12_381);
            assert_eq!(pk.key_type(), KeyType::BLS12_381);

            pk.set_purpose(Purpose::TRANSFER);
            assert_eq!(pk.purpose(), Purpose::TRANSFER);

            pk.set_security_level(SecurityLevel::HIGH);
            assert_eq!(pk.security_level(), SecurityLevel::HIGH);

            pk.set_read_only(true);
            assert!(pk.read_only());

            let new_data = BinaryData::new(vec![0x03; 48]);
            pk.set_data(new_data.clone());
            assert_eq!(pk.data(), &new_data);

            let new_sig = BinaryData::new(vec![0xBB; 96]);
            pk.set_signature(new_sig.clone());
            assert_eq!(pk.signature(), &new_sig);

            let bounds = ContractBounds::SingleContract {
                id: Identifier::new([1u8; 32]),
            };
            pk.set_contract_bounds(Some(bounds.clone()));
            assert_eq!(pk.contract_bounds(), Some(&bounds));

            pk.set_contract_bounds(None);
            assert!(pk.contract_bounds().is_none());
        }
    }

    // =========================================================================
    // Cross-type conversion tests: V0 -> StateTransition via From
    // =========================================================================
    mod from_conversions {
        use super::*;
        use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
        use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
        use crate::state_transition::identity_credit_withdrawal_transition::v1::IdentityCreditWithdrawalTransitionV1;
        use crate::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
        use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
        use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
        use crate::state_transition::StateTransition;

        #[test]
        fn credit_transfer_v0_to_state_transition() {
            let t = IdentityCreditTransferTransitionV0::default();
            let st: StateTransition = t.into();
            assert_eq!(
                st.state_transition_type(),
                StateTransitionType::IdentityCreditTransfer
            );
        }

        #[test]
        fn credit_withdrawal_v0_to_state_transition() {
            let t = IdentityCreditWithdrawalTransitionV0::default();
            let st: StateTransition = t.into();
            assert_eq!(
                st.state_transition_type(),
                StateTransitionType::IdentityCreditWithdrawal
            );
        }

        #[test]
        fn credit_withdrawal_v1_to_state_transition() {
            let t = IdentityCreditWithdrawalTransitionV1::default();
            let st: StateTransition = t.into();
            assert_eq!(
                st.state_transition_type(),
                StateTransitionType::IdentityCreditWithdrawal
            );
        }

        #[test]
        fn identity_update_v0_to_state_transition() {
            let t = IdentityUpdateTransitionV0::default();
            let st: StateTransition = t.into();
            assert_eq!(
                st.state_transition_type(),
                StateTransitionType::IdentityUpdate
            );
        }

        #[test]
        fn identity_topup_v0_to_state_transition() {
            let t = IdentityTopUpTransitionV0::default();
            let st: StateTransition = t.into();
            assert_eq!(
                st.state_transition_type(),
                StateTransitionType::IdentityTopUp
            );
        }

        #[test]
        fn masternode_vote_v0_to_state_transition() {
            let t = MasternodeVoteTransitionV0::default();
            let st: StateTransition = t.into();
            assert_eq!(
                st.state_transition_type(),
                StateTransitionType::MasternodeVote
            );
        }
    }
}
