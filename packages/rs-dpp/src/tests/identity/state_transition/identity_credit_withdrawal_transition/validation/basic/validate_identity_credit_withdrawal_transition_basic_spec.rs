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

    use crate::assert_consensus_errors;
    use crate::consensus::ConsensusError;
    use crate::NonConsensusError;
    use jsonschema::error::ValidationErrorKind;

    mod protocol_version {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("protocolVersion").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");
            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"protocolVersion\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("protocolVersion", "1")
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/protocolVersion");
            assert_eq!(error.keyword().unwrap(), "type");
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
                    NonConsensusError::SerdeParsingError(e) => {
                        assert_eq!(e.message(), "Expected protocolVersion to be a uint");
                    }
                    _ => {
                        panic!("Expected version parsing error");
                    }
                },
            }
        }
    }

    mod type_property {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("type").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"type\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("type", "1").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 2);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/type");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        async fn should_be_equal_to_6() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("type", 42).unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/type");
            assert_eq!(error.keyword().unwrap(), "const");
        }
    }

    mod identity_id {
        use super::*;

        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("identityId").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"identityId\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        pub async fn should_be_a_byte_array() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("identityId", vec!["string"; 32])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 32);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/identityId/0");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_shorter_than_32_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("identityId", vec![0; 30])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/identityId");
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[tokio::test]
        pub async fn should_be_not_longer_than_32_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("identityId", vec![0; 33])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/identityId");
            assert_eq!(error.keyword().unwrap(), "maxItems");
        }
    }

    mod amount {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("amount").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"amount\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("amount", "1").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/amount");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_less_than_1() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("amount", 900).unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/amount");
            assert_eq!(error.keyword().unwrap(), "minimum");
        }
    }

    mod core_fee_per_byte {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("coreFeePerByte").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"coreFeePerByte\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("coreFeePerByte", "1")
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/coreFeePerByte");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_less_than_1() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("coreFeePerByte", -1)
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/coreFeePerByte");
            assert_eq!(error.keyword().unwrap(), "minimum");
        }

        #[tokio::test]
        pub async fn should_be_not_more_than_u32_max() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("coreFeePerByte", u32::MAX as u64 + 1u64)
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/coreFeePerByte");
            assert_eq!(error.keyword().unwrap(), "maximum");
        }

        #[tokio::test]
        pub async fn should_be_in_a_fibonacci_sequence() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("coreFeePerByte", 6)
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(
                result,
                ConsensusError::InvalidIdentityCreditWithdrawalTransitionCoreFeeError,
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

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"pooling\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("pooling", "1").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 2);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/pooling");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        async fn should_be_valid_enum_variant() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("pooling", 3).unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/pooling");
            assert_eq!(error.keyword().unwrap(), "enum");
        }

        #[tokio::test]
        async fn should_constraint_variant_to_0() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_into_value("pooling", 2).unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(
                result,
                ConsensusError::NotImplementedIdentityCreditWithdrawalTransitionPoolingError,
                1
            );

            let error = errors.first().unwrap();

            assert_eq!(error.pooling(), 2);
        }
    }

    mod output_script {
        use crate::identity::core_script::CoreScript;

        use super::*;

        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("outputScript").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"outputScript\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        pub async fn should_be_a_byte_array() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("outputScript", vec!["string"; 23])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 23);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/outputScript/0");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_shorter_than_23_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("outputScript", vec![0; 9])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/outputScript");
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[tokio::test]
        pub async fn should_be_not_longer_than_25_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("outputScript", vec![0; 10018])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/outputScript");
            assert_eq!(error.keyword().unwrap(), "maxItems");
        }

        #[tokio::test]
        pub async fn should_be_of_a_proper_type() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("outputScript", vec![6; 23])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(
                result,
                ConsensusError::InvalidIdentityCreditWithdrawalTransitionOutputScriptError,
                1
            );

            let error = errors.first().unwrap();

            assert_eq!(error.output_script(), CoreScript::from_bytes(vec![6; 23]));
        }
    }

    mod signature {
        use super::*;

        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("signature").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"signature\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        pub async fn should_be_a_byte_array() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("signature", vec!["string"; 65])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 65);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature/0");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_shorter_than_65_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("signature", vec![0; 64])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature");
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[tokio::test]
        pub async fn should_be_not_longer_than_65_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("signature", vec![0; 66])
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature");
            assert_eq!(error.keyword().unwrap(), "maxItems");
        }
    }

    mod signature_public_key_id {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove("signaturePublicKeyId").unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"signaturePublicKeyId\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("signaturePublicKeyId", "1")
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signaturePublicKeyId");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_less_than_0() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition
                .set_into_value("signaturePublicKeyId", -1)
                .unwrap();

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signaturePublicKeyId");
            assert_eq!(error.keyword().unwrap(), "minimum");
        }
    }

    #[tokio::test]
    async fn should_return_valid_result() {
        let (raw_state_transition, validator) = setup_test();

        let result = validator.validate(&raw_state_transition).await.unwrap();

        assert!(result.is_valid());
    }
}
