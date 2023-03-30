use std::sync::Arc;

use jsonschema::error::ValidationErrorKind;
use platform_value::Value;

use crate::assert_consensus_errors;
use crate::errors::consensus::ConsensusError;
use crate::identity::state_transition::asset_lock_proof::{
    AssetLockProofValidator, AssetLockTransactionValidator, ChainAssetLockProofStructureValidator,
    InstantAssetLockProofStructureValidator,
};
use crate::identity::state_transition::identity_topup_transition::validation::basic::IdentityTopUpTransitionBasicValidator;
use crate::state_repository::MockStateRepositoryLike;
use crate::version::ProtocolVersionValidator;
use crate::NonConsensusError;

pub fn setup_test(
    state_repository_mock: MockStateRepositoryLike,
) -> (
    Value,
    IdentityTopUpTransitionBasicValidator<MockStateRepositoryLike>,
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
        crate::tests::fixtures::identity_topup_transition_fixture(None),
        IdentityTopUpTransitionBasicValidator::new(
            protocol_version_validator,
            asset_lock_proof_validator,
        )
        .unwrap(),
    )
}

mod validate_identity_topup_transition_basic {
    pub use super::*;

    use crate::assert_basic_consensus_errors;
    use crate::errors::consensus::basic::BasicError;

    mod protocol_version {
        use super::*;

        #[tokio::test]
        pub async fn should_be_present() {
            let state_repository = MockStateRepositoryLike::new();
            let (mut raw_state_transition, validator) = setup_test(state_repository);
            raw_state_transition.remove("protocolVersion").unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

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
            let (mut raw_state_transition, validator) = setup_test(MockStateRepositoryLike::new());
            raw_state_transition
                .set_into_value("protocolVersion", "1")
                .unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/protocolVersion");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_valid() {
            let (mut raw_state_transition, validator) = setup_test(MockStateRepositoryLike::new());
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
                        panic!("Expected version parsing error");
                    }
                },
            }
        }
    }

    mod type_a {
        pub use super::*;

        #[tokio::test]
        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test(MockStateRepositoryLike::new());
            raw_state_transition.remove("type").unwrap();
            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

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
        pub async fn should_be_equal_to_3() {
            let (mut raw_state_transition, validator) = setup_test(MockStateRepositoryLike::new());
            raw_state_transition.set_into_value("type", 666).unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/type");
            assert_eq!(error.keyword().unwrap(), "const");

            match error.kind() {
                ValidationErrorKind::Constant { expected_value } => {
                    assert_eq!(expected_value.as_u64().unwrap(), 3u64);
                }
                _ => panic!("Expected to have a constant value"),
            }
        }
    }

    mod asset_lock_proof {
        pub use super::*;

        #[tokio::test]
        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test(MockStateRepositoryLike::new());
            raw_state_transition.remove("assetLockProof").unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

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
            let (mut raw_state_transition, validator) = setup_test(MockStateRepositoryLike::new());
            raw_state_transition
                .set_into_value("assetLockProof", 1u64)
                .unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/assetLockProof");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        pub async fn should_be_valid() {
            let (mut raw_state_transition, validator) = setup_test(MockStateRepositoryLike::new());
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

            assert_eq!(error.instance_path().to_string(), "/transaction");
        }
    }

    mod signature {
        use super::*;

        #[tokio::test]
        pub async fn should_be_present() {
            let (mut raw_state_transition, validator) = setup_test(MockStateRepositoryLike::new());
            raw_state_transition.remove("signature").unwrap();

            let result = validator
                .validate(&raw_state_transition, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

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
            let (mut raw_state_transition, validator) = setup_test(MockStateRepositoryLike::new());
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
            let (mut raw_state_transition, validator) = setup_test(MockStateRepositoryLike::new());
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
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[tokio::test]
        pub async fn should_be_not_longer_than_65_bytes() {
            let (mut raw_state_transition, validator) = setup_test(MockStateRepositoryLike::new());
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
            assert_eq!(error.keyword().unwrap(), "maxItems");
        }
    }

    #[tokio::test]
    pub async fn should_return_valid_result() {
        let mut state_repository = MockStateRepositoryLike::new();
        state_repository
            .expect_verify_instant_lock()
            .returning(|_asset_lock, _| Ok(true));
        state_repository
            .expect_is_asset_lock_transaction_out_point_already_used()
            .returning(|_asset_lock, _| Ok(false));

        let (raw_state_transition, validator) = setup_test(state_repository);
        let result = validator
            .validate(&raw_state_transition, &Default::default())
            .await
            .unwrap();

        assert!(result.is_valid());
    }
}
