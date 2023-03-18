use platform_value::Value;
use std::sync::Arc;

use crate::bls::NativeBlsModule;
use crate::identity::state_transition::asset_lock_proof::{
    AssetLockProofValidator, AssetLockTransactionValidator, ChainAssetLockProofStructureValidator,
    InstantAssetLockProofStructureValidator,
};
use crate::identity::state_transition::identity_create_transition::validation::basic::IdentityCreateTransitionBasicValidator;
use crate::identity::state_transition::validate_public_key_signatures::TPublicKeysSignaturesValidator;
use crate::identity::validation::TPublicKeysValidator;
use crate::state_repository::MockStateRepositoryLike;
use crate::validation::SimpleValidationResult;
use crate::version::ProtocolVersionValidator;

#[derive(Default)]
pub struct SignaturesValidatorMock {}

impl TPublicKeysSignaturesValidator for SignaturesValidatorMock {
    fn validate_public_key_signatures<'a>(
        &self,
        _raw_state_transition: &Value,
        _raw_public_keys: impl IntoIterator<Item = &'a Value>,
    ) -> Result<crate::validation::SimpleValidationResult, crate::NonConsensusError> {
        Ok(SimpleValidationResult::default())
    }
}

pub fn setup_test(
    public_keys_validator: Arc<impl TPublicKeysValidator>,
    public_keys_transition_validator: Arc<impl TPublicKeysValidator>,
    state_repository_mock: MockStateRepositoryLike,
) -> (
    Value,
    IdentityCreateTransitionBasicValidator<
        impl TPublicKeysValidator,
        impl TPublicKeysValidator,
        MockStateRepositoryLike,
        SignaturesValidatorMock,
        NativeBlsModule,
    >,
) {
    let state_repository = Arc::new(state_repository_mock);
    let asset_lock_transaction_validator =
        Arc::new(AssetLockTransactionValidator::new(state_repository.clone()));

    let instant_asset_lock_validator = InstantAssetLockProofStructureValidator::new(
        state_repository.clone(),
        asset_lock_transaction_validator.clone(),
    )
    .unwrap();

    let chain_asset_lock_validator = ChainAssetLockProofStructureValidator::new(
        state_repository,
        asset_lock_transaction_validator,
    )
    .unwrap();

    let asset_lock_proof_validator = Arc::new(AssetLockProofValidator::new(
        instant_asset_lock_validator,
        chain_asset_lock_validator,
    ));

    let protocol_version_validator = ProtocolVersionValidator::default();
    (
        crate::tests::fixtures::identity_create_transition_fixture(None),
        IdentityCreateTransitionBasicValidator::new(
            protocol_version_validator,
            public_keys_validator,
            public_keys_transition_validator,
            asset_lock_proof_validator,
            NativeBlsModule::default(),
            SignaturesValidatorMock::default(),
        )
        .unwrap(),
    )
}

mod validate_identity_create_transition_basic_factory {
    use std::sync::Arc;

    use crate::identity::validation::RequiredPurposeAndSecurityLevelValidator;
    use crate::state_repository::MockStateRepositoryLike;
    use crate::tests::fixtures::PublicKeysValidatorMock;
    use crate::validation::ValidationResult;

    pub use super::setup_test;

    mod protocol_version {
        use std::sync::Arc;

        use jsonschema::error::ValidationErrorKind;

        use crate::consensus::ConsensusError;
        use crate::identity::validation::RequiredPurposeAndSecurityLevelValidator;
        use crate::state_repository::MockStateRepositoryLike;
        use crate::tests::fixtures::get_public_keys_validator_for_transition;
        use crate::{assert_consensus_errors, NonConsensusError};

        use super::setup_test;

        #[tokio::test]
        pub async fn should_be_present() {
            let state_repository = MockStateRepositoryLike::new();
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                state_repository,
            );
            raw_state_transition.remove("protocolVersion").unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

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
        pub async fn should_be_an_integer() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition
                .set_into_value("protocolVersion", "1")
                .unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/protocolVersion");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_valid() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition
                .set_into_value("protocolVersion", -1)
                .unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await;

            match result {
                Ok(_) => {
                    panic!("Expected error");
                }
                Err(e) => match e {
                    NonConsensusError::ValueError(e) => {
                        assert_eq!(e.to_string(), "integer out of bounds");
                    }
                    _ => {
                        panic!("Expected value error");
                    }
                },
            }
        }
    }

    mod type_a {
        use std::sync::Arc;

        use jsonschema::error::ValidationErrorKind;

        use crate::assert_consensus_errors;
        use crate::consensus::ConsensusError;
        use crate::identity::validation::RequiredPurposeAndSecurityLevelValidator;
        use crate::state_repository::MockStateRepositoryLike;
        use crate::tests::fixtures::get_public_keys_validator_for_transition;

        use super::super::setup_test;

        #[tokio::test]
        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition.remove("type").unwrap();
            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

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
        pub async fn should_be_equal_to_2() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition.set_into_value("type", 666).unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/type");
            assert_eq!(error.keyword().unwrap(), "const");

            match error.kind() {
                ValidationErrorKind::Constant { expected_value } => {
                    assert_eq!(expected_value.as_u64().unwrap(), 2u64);
                }
                _ => panic!("Expected to have a constant value"),
            }
        }
    }

    mod asset_lock_proof {
        use std::sync::Arc;

        use jsonschema::error::ValidationErrorKind;

        use crate::assert_consensus_errors;
        use crate::consensus::ConsensusError;
        use crate::identity::validation::RequiredPurposeAndSecurityLevelValidator;
        use crate::state_repository::MockStateRepositoryLike;
        use crate::tests::fixtures::get_public_keys_validator_for_transition;

        use super::super::setup_test;

        #[tokio::test]
        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition.remove("assetLockProof").unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"assetLockProof\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        pub async fn should_be_an_object() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition
                .set_into_value("assetLockProof", 1)
                .unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/assetLockProof");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_valid() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition
                .set_value_at_path(
                    "assetLockProof",
                    "transaction",
                    "totally not a valid type".into(),
                )
                .unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();
            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/transaction");
        }
    }

    mod public_keys {
        use std::sync::Arc;

        use jsonschema::error::ValidationErrorKind;
        use platform_value::Value;

        use crate::assert_consensus_errors;
        use crate::consensus::basic::TestConsensusError;
        use crate::consensus::ConsensusError;
        use crate::identity::validation::RequiredPurposeAndSecurityLevelValidator;
        use crate::state_repository::MockStateRepositoryLike;
        use crate::tests::fixtures::{
            get_public_keys_validator_for_transition, PublicKeysValidatorMock,
        };
        use crate::validation::ValidationResult;

        use super::super::setup_test;

        #[tokio::test]
        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition.remove("publicKeys").unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");

            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"publicKeys\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        pub async fn should_not_be_empty() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition
                .set_into_value("publicKeys", Vec::<Value>::new())
                .unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.keyword().unwrap(), "minItems");
            assert_eq!(error.instance_path().to_string(), "/publicKeys");
        }

        #[tokio::test]
        pub async fn should_not_have_more_than_10_items() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );

            let public_keys = raw_state_transition
                .get_array_mut_ref("publicKeys")
                .unwrap();
            let key = public_keys.first().unwrap().clone();

            for _ in 0..10 {
                public_keys.push(key.clone());
            }

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 2);

            let error = errors.first().unwrap();

            assert_eq!(error.keyword().unwrap(), "maxItems");
            assert_eq!(error.instance_path().to_string(), "/publicKeys");
        }

        #[tokio::test]
        pub async fn should_be_unique() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );

            let public_keys = raw_state_transition
                .get_array_mut_ref("publicKeys")
                .unwrap();
            let key = public_keys.first().unwrap().clone();
            public_keys.push(key);

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.keyword().unwrap(), "uniqueItems");
            assert_eq!(error.instance_path().to_string(), "/publicKeys");
        }

        #[tokio::test]
        pub async fn should_be_valid() {
            let pk_validator_mock = Arc::new(PublicKeysValidatorMock::new());
            let pk_error = TestConsensusError::new("test");
            pk_validator_mock.returns_fun(move || {
                Ok(ValidationResult::new(Some(vec![ConsensusError::from(
                    TestConsensusError::new("test"),
                )])))
            });

            let (raw_state_transition, validator) = setup_test(
                pk_validator_mock.clone(),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::TestConsensusError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error, &&pk_error);

            assert_eq!(
                pk_validator_mock.called_with(),
                raw_state_transition.get_array("publicKeys").unwrap()
            );
        }

        #[tokio::test]
        pub async fn should_have_at_least_1_master_key() {
            let pk_validator_mock = Arc::new(PublicKeysValidatorMock::new());
            let pk_error = TestConsensusError::new("test");
            pk_validator_mock.returns_fun(move || {
                Ok(ValidationResult::new(Some(vec![ConsensusError::from(
                    TestConsensusError::new("test"),
                )])))
            });

            let (raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                pk_validator_mock.clone(),
                MockStateRepositoryLike::new(),
            );

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::TestConsensusError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error, &&pk_error);

            assert_eq!(
                pk_validator_mock.called_with(),
                raw_state_transition.get_array("publicKeys").unwrap()
            );
        }
    }

    mod signature {
        use std::sync::Arc;

        use jsonschema::error::ValidationErrorKind;

        use crate::assert_consensus_errors;
        use crate::consensus::ConsensusError;
        use crate::identity::validation::RequiredPurposeAndSecurityLevelValidator;
        use crate::state_repository::MockStateRepositoryLike;
        use crate::tests::fixtures::get_public_keys_validator_for_transition;

        use super::super::setup_test;

        #[tokio::test]
        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition.remove("signature").unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

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
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition
                .set_into_value("signature", vec!["string"; 65])
                .unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 65);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature/0");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_not_shorter_than_65_bytes() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition
                .set_into_value("signature", vec![0; 64])
                .unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature");
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[tokio::test]
        pub async fn should_be_not_longer_than_65_bytes() {
            let (mut raw_state_transition, validator) = setup_test(
                Arc::new(get_public_keys_validator_for_transition()),
                Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
                MockStateRepositoryLike::new(),
            );
            raw_state_transition
                .set_into_value("signature", vec![0; 66])
                .unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature");
            assert_eq!(error.keyword().unwrap(), "maxItems");
        }
    }

    #[tokio::test]
    pub async fn should_return_valid_result() {
        let pk_validator_mock = Arc::new(PublicKeysValidatorMock::new());
        pk_validator_mock.returns_fun(move || Ok(ValidationResult::default()));

        let mut state_repository = MockStateRepositoryLike::new();
        state_repository
            .expect_verify_instant_lock()
            .returning(|_asset_lock, _| Ok(true));
        state_repository
            .expect_is_asset_lock_transaction_out_point_already_used()
            .returning(|_asset_lock, _| Ok(false));

        let (raw_state_transition, validator) = setup_test(
            pk_validator_mock.clone(),
            Arc::new(RequiredPurposeAndSecurityLevelValidator::default()),
            state_repository,
        );
        let result = validator
            .validate(&raw_state_transition, &Default::default())
            .await
            .unwrap();

        assert!(result.is_valid());
        assert_eq!(
            pk_validator_mock.called_with(),
            raw_state_transition.get_array("publicKeys").unwrap()
        );
    }
}
