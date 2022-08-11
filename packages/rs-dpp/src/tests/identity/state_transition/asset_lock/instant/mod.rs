#[cfg(test)]
mod validate_instant_asset_lock_proof_structure_factory {
    use std::str::FromStr;
    use std::sync::Arc;

    use dashcore::hashes::sha256d::Hash as Sha256;
    use dashcore::hashes::Hash;
    use dashcore::Txid;
    use dashcore::{PrivateKey, Transaction};
    use jsonschema::error::ValidationErrorKind;
    use serde_json::Value as JsonValue;

    use crate::assert_consensus_errors;
    use crate::consensus::ConsensusError;
    use crate::identity::state_transition::asset_lock_proof::{
        AssetLockProof, InstantAssetLockProof,
    };
    use crate::identity::state_transition::asset_lock_proof::{
        AssetLockTransactionValidator, InstantAssetLockProofStructureValidator,
    };
    use crate::state_repository::MockStateRepositoryLike;
    use crate::tests::fixtures::instant_asset_lock_proof_fixture;
    use crate::tests::fixtures::{
        instant_asset_lock_is_lock_fixture, instant_asset_lock_proof_transaction_fixture,
    };
    use crate::tests::utils::SerdeTestExtension;

    struct TestData {
        pub validate_instant_asset_lock_proof_structure:
            InstantAssetLockProofStructureValidator<MockStateRepositoryLike>,
        pub raw_proof: JsonValue,
        pub transaction: Transaction,
        pub public_key_hash: Vec<u8>,
        pub state_repository_mock: Arc<MockStateRepositoryLike>,
    }

    fn setup_test(maybe_state_repository_mock: Option<MockStateRepositoryLike>) -> TestData {
        let private_key_hex = "cSBnVM4xvxarwGQuAfQFwqDg9k5tErHUHzgWsEfD4zdwUasvqRVY";
        let private_key = PrivateKey::from_str(private_key_hex).unwrap();
        let asset_lock = instant_asset_lock_proof_fixture(Some(private_key));

        let transaction = asset_lock.transaction().unwrap().clone();

        let raw_proof = asset_lock.to_raw_object().unwrap();

        let state_repository_mock = maybe_state_repository_mock.unwrap_or_else(|| {
            let mut state_repository_mock = MockStateRepositoryLike::new();
            state_repository_mock
                .expect_verify_instant_lock()
                .returning(|_asset_lock| Ok(true));

            state_repository_mock
                .expect_is_asset_lock_transaction_out_point_already_used()
                .returning(|_| Ok(false));
            state_repository_mock
        });

        let state_repository = Arc::new(state_repository_mock);

        let public_key_hash = hex::decode("88d9931ea73d60eaf7e5671efc0552b912911f2a").unwrap();

        let validate_asset_lock_transaction_mock =
            AssetLockTransactionValidator::new(state_repository.clone());
        let validate_asset_lock_transaction = Arc::new(validate_asset_lock_transaction_mock);

        let validate_instant_asset_lock_proof_structure =
            InstantAssetLockProofStructureValidator::new(
                state_repository.clone(),
                validate_asset_lock_transaction,
            )
            .unwrap();

        TestData {
            validate_instant_asset_lock_proof_structure,
            raw_proof,
            transaction,
            public_key_hash,
            state_repository_mock: state_repository,
        }
    }

    mod asset_lock_type {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.remove_key("type");

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
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
        async fn should_be_equal_to_0() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.set_key_value("type", -1);

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/type");
            assert_eq!(error.keyword().unwrap(), "const");
        }
    }

    mod instant_lock {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.remove_key("instantLock");

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");
            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"instantLock\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        async fn should_be_a_byte_array() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_key_value("instantLock", vec!["string"; 165]);

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 165);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/instantLock/0");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        async fn should_not_be_shorter_than_160_bytes() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_key_value("instantLock", vec![0u8; 159]);

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/instantLock");
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[tokio::test]
        async fn should_not_be_longer_than_100_kb() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_key_value("instantLock", vec![0u8; 100001]);

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/instantLock");
            assert_eq!(error.keyword().unwrap(), "maxItems");
        }

        #[tokio::test]
        async fn should_be_valid() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_key_value("instantLock", vec![0u8; 200]);

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            assert_consensus_errors!(result, ConsensusError::InvalidInstantAssetLockProofError, 1);

            let consensus_error = result.errors().first().unwrap();
            assert_eq!(consensus_error.code(), 1041);
        }

        #[tokio::test]
        async fn should_lock_the_same_transaction() {
            let test_data = setup_test(None);
            let transaction = instant_asset_lock_proof_transaction_fixture(None);
            let instant_lock =
                instant_asset_lock_is_lock_fixture(Txid::from_hash(Sha256::from_inner([0u8; 32])));
            let asset_lock_proof = AssetLockProof::Instant(InstantAssetLockProof::new(
                instant_lock,
                transaction.clone(),
                0,
            ));

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&asset_lock_proof.to_raw_object().unwrap())
                .await
                .unwrap();

            let errors = assert_consensus_errors!(
                result,
                ConsensusError::IdentityAssetLockProofLockedTransactionMismatchError,
                1
            );

            let error = errors.first().unwrap();
            let consensus_error = result.errors().first().unwrap();
            assert_eq!(consensus_error.code(), 1031);
            assert_eq!(
                error.instant_lock_transaction_id(),
                Txid::from_hash(Sha256::from_inner([0u8; 32]))
            );
            assert_eq!(error.asset_lock_transaction_id(), transaction.txid());
        }

        #[tokio::test]
        async fn should_have_valid_signature() {
            let mut state_repository_mock = MockStateRepositoryLike::new();
            state_repository_mock
                .expect_verify_instant_lock()
                .returning(|_asset_lock| Ok(false));
            let test_data = setup_test(Some(state_repository_mock));

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            assert!(!result.is_valid());

            assert_consensus_errors!(
                result,
                ConsensusError::InvalidInstantAssetLockProofSignatureError,
                1
            );

            let consensus_error = result.errors().first().unwrap();
            assert_eq!(consensus_error.code(), 1042);
        }
    }

    mod transaction {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.remove_key("transaction");

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");
            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"transaction\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        async fn should_be_a_byte_array() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_key_value("instantLock", vec!["string"; 65]);

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 66);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/instantLock/0");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        async fn should_not_be_shorter_than_1_byte() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_key_value("instantLock", vec![0u8; 0]);

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/instantLock");
            assert_eq!(error.keyword().unwrap(), "minItems");
        }

        #[tokio::test]
        async fn should_not_be_longer_than_100_kb() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_key_value("instantLock", vec![0u8; 100001]);

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/instantLock");
            assert_eq!(error.keyword().unwrap(), "maxItems");
        }

        #[tokio::test]
        async fn should_should_be_valid() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_key_value("transaction", vec![0u8; 64]);

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            assert_consensus_errors!(
                result,
                ConsensusError::InvalidIdentityAssetLockTransactionError,
                1
            );

            let consensus_error = result.errors().first().unwrap();
            assert_eq!(consensus_error.code(), 1038);
        }
    }

    mod output_index {
        use super::*;

        #[tokio::test]
        async fn should_be_present() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.remove_key("outputIndex");

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword().unwrap(), "required");
            match error.kind() {
                ValidationErrorKind::Required { property } => {
                    assert_eq!(property.to_string(), "\"outputIndex\"");
                }
                _ => panic!("Expected to be missing property"),
            }
        }

        #[tokio::test]
        async fn should_be_an_integer() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.set_key_value("outputIndex", 1.1);

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/outputIndex");
            assert_eq!(error.keyword().unwrap(), "type");
        }

        #[tokio::test]
        async fn should_not_be_less_than_0() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.set_key_value("outputIndex", -1);

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof)
                .await
                .unwrap();

            let errors = assert_consensus_errors!(result, ConsensusError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/outputIndex");
            assert_eq!(error.keyword().unwrap(), "minimum");
        }
    }

    #[tokio::test]
    async fn should_return_a_valid_result() {
        let test_data = setup_test(None);

        let result = test_data
            .validate_instant_asset_lock_proof_structure
            .validate(&test_data.raw_proof)
            .await
            .unwrap();

        assert!(result.is_valid());
        assert_eq!(result.data().unwrap().to_vec(), test_data.public_key_hash);
    }
}
