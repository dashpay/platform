use std::sync::Arc;

use serde_json::Value;

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
    use crate::tests::utils::SerdeTestExtension;
    use crate::NonConsensusError;
    use jsonschema::error::ValidationErrorKind;

    mod protocol_version {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove_key("protocolVersion");

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

            raw_state_transition.set_key_value("protocolVersion", "1");

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/protocolVersion");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        async fn should_be_valid() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("protocolVersion", -1);

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

            raw_state_transition.remove_key("type");

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

            raw_state_transition.set_key_value("type", "1");

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 2);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/type");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        async fn should_be_equal_to_6() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("type", 42);

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

            raw_state_transition.remove_key("identityId");

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

            raw_state_transition.set_key_value("identityId", vec!["string"; 32]);

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 32);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/identityId/0");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_shorter_than_32_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("identityId", vec![0; 30]);

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/identityId");
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[tokio::test]
        pub async fn should_be_not_longer_than_32_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("identityId", vec![0; 33]);

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

            raw_state_transition.remove_key("amount");

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

            raw_state_transition.set_key_value("amount", "1");

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/amount");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_less_than_1() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("amount", 900);

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/amount");
            assert_eq!(error.keyword().unwrap(), "minimum");
        }
    }

    mod core_fee {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove_key("coreFee");

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"coreFee\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        async fn should_be_integer() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("coreFee", "1");

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/coreFee");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_less_than_1() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("coreFee", -1);

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/coreFee");
            assert_eq!(error.keyword().unwrap(), "minimum");
        }
    }

    mod pooling {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove_key("pooling");

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

            raw_state_transition.set_key_value("pooling", "1");

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 2);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/pooling");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        async fn should_be_valid_enum_variant() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("pooling", 3);

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/pooling");
            assert_eq!(error.keyword().unwrap(), "enum");
        }
    }

    mod output {
        use super::*;

        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove_key("output");

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"output\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        pub async fn should_be_a_byte_array() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("output", vec!["string"; 65]);

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 65);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/output/0");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_shorter_than_10_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("output", vec![0; 9]);

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/output");
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[tokio::test]
        pub async fn should_be_not_longer_than_10017_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("output", vec![0; 10018]);

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/output");
            assert_eq!(error.keyword().unwrap(), "maxItems");
        }
    }

    mod signature {
        use super::*;

        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.remove_key("signature");

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

            raw_state_transition.set_key_value("signature", vec!["string"; 65]);

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 65);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature/0");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_shorter_than_65_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("signature", vec![0; 64]);

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature");
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[tokio::test]
        pub async fn should_be_not_longer_than_65_bytes() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("signature", vec![0; 66]);

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

            raw_state_transition.remove_key("signaturePublicKeyId");

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

            raw_state_transition.set_key_value("signaturePublicKeyId", "1");

            let result = validator.validate(&raw_state_transition).await.unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signaturePublicKeyId");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_less_than_0() {
            let (mut raw_state_transition, validator) = setup_test();

            raw_state_transition.set_key_value("signaturePublicKeyId", -1);

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

        assert_eq!(result.is_valid(), true);
    }
}
