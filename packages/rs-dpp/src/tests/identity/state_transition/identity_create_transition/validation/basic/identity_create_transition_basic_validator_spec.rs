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
use crate::validation::SimpleConsensusValidationResult;
use crate::version::ProtocolVersionValidator;

#[derive(Default)]
pub struct SignaturesValidatorMock {}

impl TPublicKeysSignaturesValidator for SignaturesValidatorMock {
    fn validate_public_key_signatures<'a>(
        &self,
        _raw_state_transition: &Value,
        _raw_public_keys: impl IntoIterator<Item = &'a Value>,
    ) -> Result<crate::validation::SimpleConsensusValidationResult, crate::NonConsensusError> {
        Ok(SimpleConsensusValidationResult::default())
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
            Arc::new(SignaturesValidatorMock::default()),
        )
        .unwrap(),
    )
}

mod validate_identity_create_transition_basic_factory {
    use std::sync::Arc;

    use crate::identity::validation::RequiredPurposeAndSecurityLevelValidator;
    use crate::state_repository::MockStateRepositoryLike;
    use crate::tests::fixtures::PublicKeysValidatorMock;
    use crate::validation::ConsensusValidationResult;

    use crate::consensus::basic::basic_error::BasicError;
    use crate::consensus::ConsensusError;

    pub use super::setup_test;

    mod protocol_version {
        use super::*;

        use std::sync::Arc;

        use crate::identity::validation::RequiredPurposeAndSecurityLevelValidator;
        use crate::state_repository::MockStateRepositoryLike;
        use crate::tests::fixtures::get_public_keys_validator_for_transition;
        use crate::{assert_basic_consensus_errors, NonConsensusError};

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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "protocolVersion");
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/protocolVersion");
            assert_eq!(error.keyword(), "type");
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
        use super::*;

        use std::sync::Arc;

        use crate::assert_basic_consensus_errors;
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "type");
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            let allowed_value: u32 = error
                .params()
                .get_integer("allowedValue")
                .expect("should get allowedValue");

            assert_eq!(error.instance_path(), "/type");
            assert_eq!(error.keyword(), "const");
            assert_eq!(allowed_value, 2);
        }
    }

    mod asset_lock_proof {
        use super::*;

        use std::sync::Arc;

        use crate::assert_basic_consensus_errors;
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "assetLockProof");
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/assetLockProof");
            assert_eq!(error.keyword(), "type");
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
            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/transaction");
        }
    }

    mod public_keys {
        use super::*;

        use platform_value::Value;
        use std::sync::Arc;

        use crate::consensus::test_consensus_error::TestConsensusError;
        use crate::consensus::ConsensusError;
        use crate::identity::validation::RequiredPurposeAndSecurityLevelValidator;
        use crate::state_repository::MockStateRepositoryLike;
        use crate::tests::fixtures::{
            get_public_keys_validator_for_transition, PublicKeysValidatorMock,
        };
        use crate::validation::ConsensusValidationResult;
        use crate::{assert_basic_consensus_errors, assert_consensus_errors};

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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "publicKeys");
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.keyword(), "minItems");
            assert_eq!(error.instance_path(), "/publicKeys");
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 2);

            let error = errors.first().unwrap();

            assert_eq!(error.keyword(), "maxItems");
            assert_eq!(error.instance_path(), "/publicKeys");
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.keyword(), "uniqueItems");
            assert_eq!(error.instance_path(), "/publicKeys");
        }

        #[tokio::test]
        pub async fn should_be_valid() {
            let pk_validator_mock = Arc::new(PublicKeysValidatorMock::new());
            let pk_error = TestConsensusError::new("test");
            pk_validator_mock.returns_fun(move || {
                Ok(ConsensusValidationResult::new_with_errors(vec![
                    ConsensusError::from(TestConsensusError::new("test")),
                ]))
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
                Ok(ConsensusValidationResult::new_with_errors(vec![
                    ConsensusError::from(TestConsensusError::new("test")),
                ]))
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
        use super::*;

        use std::sync::Arc;

        use crate::assert_basic_consensus_errors;
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "signature");
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 65);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/signature/0");
            assert_eq!(error.keyword(), "type");
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature");
            assert_eq!(error.keyword(), "minItems");
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/signature");
            assert_eq!(error.keyword(), "maxItems");
        }
    }

    #[tokio::test]
    pub async fn should_return_valid_result() {
        let pk_validator_mock = Arc::new(PublicKeysValidatorMock::new());
        pk_validator_mock.returns_fun(move || Ok(ConsensusValidationResult::default()));

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
