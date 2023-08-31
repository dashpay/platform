use std::sync::Arc;

use platform_value::Value;

use crate::{identity::state_transition::identity_credit_withdrawal_transition::validation::basic::validate_identity_credit_withdrawal_transition_basic::IdentityCreditWithdrawalTransitionBasicValidator, tests::fixtures::identity_credit_withdrawal_transition_fixture_raw_object, version::ProtocolVersionValidator};

#[cfg(test)]
pub fn setup_test() -> (Value, IdentityCreditWithdrawalTransitionBasicValidator) {
    let protocol_version_validator = ProtocolVersionValidator::default();

    (
        identity_credit_withdrawal_transition_fixture_raw_object(),
        IdentityCreditWithdrawalTransitionBasicValidator::new(Arc::new(protocol_version_validator))
            .unwrap(),
    )
}

#[cfg(test)]
mod validate_identity_credit_withdrawal_transition_basic_factory {
    use super::*;

    use crate::assert_basic_consensus_errors;
    use crate::consensus::basic::BasicError;
    use crate::errors::consensus::ConsensusError;

    use crate::NonConsensusError;

    mod protocol_version {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("protocolVersion").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "protocolVersion");
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("protocolVersion", "1")
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/protocolVersion");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        async fn should_be_valid() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("protocolVersion", -1i32)
                .unwrap();

            let result = validator.validate(&raw_state_transition).await;

            match result {
                Ok(_) => {
                    panic!("Expected error");
                }
                Err(e) => match e {
                    NonConsensusError::ValueError(e) => {
                        assert_eq!(e.to_string(), "integer out of bounds");
                    }
                    other => {
                        panic!("Expected version parsing error, got {}", other);
                    }
                },
            }
        }
    }

    mod type_property {
        use super::*;
        use crate::consensus::ConsensusError;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("type").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "type");
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("type", "1").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 2);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/type");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        async fn should_be_equal_to_6() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("type", 42).unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/type");
            assert_eq!(error.keyword(), "const");
        }
    }

    mod identity_id {
        use super::*;
        use crate::assert_basic_consensus_errors;
        use crate::consensus::ConsensusError;

        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("identityId").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "identityId");
        }

        #[tokio::test]
        pub async fn should_be_a_byte_array() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("identityId", vec!["string"; 32])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 32);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/identityId/0");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_shorter_than_32_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("identityId", vec![0; 30])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/identityId");
            assert_eq!(error.keyword(), "minItems");
        }

        #[tokio::test]
        pub async fn should_be_not_longer_than_32_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("identityId", vec![0; 33])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/identityId");
            assert_eq!(error.keyword(), "maxItems");
        }
    }

    mod amount {
        use super::*;
        use crate::consensus::ConsensusError;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("amount").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "amount");
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("amount", "1").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/amount");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_less_than_1() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("amount", 900).unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/amount");
            assert_eq!(error.keyword(), "minimum");
        }
    }

    mod core_fee_per_byte {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("coreFeePerByte").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "coreFeePerByte");
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("coreFeePerByte", "1")
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/coreFeePerByte");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_less_than_1() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("coreFeePerByte", -1)
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/coreFeePerByte");
            assert_eq!(error.keyword(), "minimum");
        }

        #[tokio::test]
        pub async fn should_be_not_more_than_u32_max() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("coreFeePerByte", u32::MAX as u64 + 1u64)
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/coreFeePerByte");
            assert_eq!(error.keyword(), "maximum");
        }

        #[tokio::test]
        pub async fn should_be_in_a_fibonacci_sequence() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("coreFeePerByte", 6)
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(
                result,
                BasicError::InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
                1
            );

            let error = errors.first().unwrap();

            assert_eq!(error.core_fee_per_byte(), 6);
        }
    }

    mod pooling {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("pooling").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "pooling");
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("pooling", "1").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 2);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/pooling");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        async fn should_be_valid_enum_variant() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("pooling", 3).unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/pooling");
            assert_eq!(error.keyword(), "enum");
        }

        #[tokio::test]
        async fn should_constraint_variant_to_0() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("pooling", 2).unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(
                result,
                BasicError::NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
                1
            );

            let error = errors.first().unwrap();

            assert_eq!(error.pooling(), 2);
        }
    }

    mod output_script {
        use crate::consensus::ConsensusError;
        use crate::identity::state_transition::properties::PROPERTY_OUTPUT_SCRIPT;

        use super::*;

        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove(PROPERTY_OUTPUT_SCRIPT).unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "outputScript");
        }

        #[tokio::test]
        pub async fn should_be_a_byte_array() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value(PROPERTY_OUTPUT_SCRIPT, vec!["string"; 23])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 23);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/outputScript/0");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_shorter_than_23_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value(PROPERTY_OUTPUT_SCRIPT, vec![0; 9])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/outputScript");
            assert_eq!(error.keyword(), "minItems");
        }

        #[tokio::test]
        pub async fn should_be_not_longer_than_25_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value(PROPERTY_OUTPUT_SCRIPT, vec![0; 10018])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/outputScript");
            assert_eq!(error.keyword(), "maxItems");
        }

        #[tokio::test]
        pub async fn should_be_of_a_proper_type() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value(PROPERTY_OUTPUT_SCRIPT, vec![6; 23])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let _errors = assert_basic_consensus_errors!(
                result,
                BasicError::InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
                1
            );
        }
    }

    mod signature {
        use super::*;
        use crate::assert_basic_consensus_errors;

        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("signature").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "signature");
        }

        #[tokio::test]
        pub async fn should_be_a_byte_array() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("signature", vec!["string"; 65])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 65);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature/0");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_shorter_than_65_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("signature", vec![0; 64])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature");
            assert_eq!(error.keyword(), "minItems");
        }

        #[tokio::test]
        pub async fn should_be_not_longer_than_65_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("signature", vec![0; 66])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature");
            assert_eq!(error.keyword(), "maxItems");
        }
    }

    mod signature_public_key_id {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("signaturePublicKeyId").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "signaturePublicKeyId");
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("signaturePublicKeyId", "1")
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signaturePublicKeyId");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_less_than_0() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("signaturePublicKeyId", -1)
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signaturePublicKeyId");
            assert_eq!(error.keyword(), "minimum");
        }
    }

    #[tokio::test]
    async fn should_return_valid_result() {
        let (raw_state_transition, validator) = setup_test();

        let result = validator.validate(&raw_state_transition).await.unwrap();

        assert!(result.is_valid());
    }
}
