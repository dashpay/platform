use std::sync::Arc;

use platform_value::Value;

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

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "protocolVersion");
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

            assert_eq!(error.instance_path(), "/protocolVersion");
            assert_eq!(error.keyword(), "type");
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

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "type");
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

            let allowed_value: u32 = error
                .params()
                .get_integer("allowedValue")
                .expect("should get allowedValue");

            assert_eq!(error.instance_path(), "/type");
            assert_eq!(error.keyword(), "const");
            assert_eq!(allowed_value, 3);
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

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "assetLockProof");
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

            assert_eq!(error.instance_path(), "/assetLockProof");
            assert_eq!(error.keyword(), "type");
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

            assert_eq!(error.instance_path(), "/transaction");
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

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "signature");
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

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 65);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/signature/0");
            assert_eq!(error.keyword(), "type");
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

            assert_eq!(error.instance_path(), "/signature");
            assert_eq!(error.keyword(), "minItems");
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

            assert_eq!(error.instance_path(), "/signature");
            assert_eq!(error.keyword(), "maxItems");
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
