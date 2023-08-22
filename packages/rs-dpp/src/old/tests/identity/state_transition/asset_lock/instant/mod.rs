#[cfg(test)]
mod validate_instant_asset_lock_proof_structure_factory {
    use std::str::FromStr;
    use std::sync::Arc;

    use crate::consensus::basic::BasicError;
    use crate::consensus::consensus_error::ConsensusError;

    use dashcore::hashes::sha256d::Hash as Sha256;
    use dashcore::hashes::Hash;
    use dashcore::Txid;
    use dashcore::{PrivateKey, Transaction};
    use platform_value::Value;

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
    struct TestData {
        pub validate_instant_asset_lock_proof_structure:
            InstantAssetLockProofStructureValidator<MockStateRepositoryLike>,
        pub raw_proof: Value,
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
                .returning(|_asset_lock, _| Ok(true));

            state_repository_mock
                .expect_is_asset_lock_transaction_out_point_already_used()
                .returning(|_, _| Ok(false));
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
        use crate::assert_basic_consensus_errors;

        #[tokio::test]
        async fn should_be_present() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.remove("type").unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "type");
        }

        #[tokio::test]
        async fn should_be_equal_to_0() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.set_into_value("type", -1).unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);

            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/type");
            assert_eq!(error.keyword(), "const");
        }
    }

    mod instant_lock {
        use super::*;
        use crate::assert_basic_consensus_errors;
        use crate::consensus::basic::BasicError;
        use crate::consensus::codes::ErrorWithCode;

        #[tokio::test]
        async fn should_be_present() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.remove("instantLock").unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "instantLock");
        }

        #[tokio::test]
        async fn should_be_a_byte_array() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_into_value("instantLock", vec!["string"; 165])
                .unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 165);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/instantLock/0");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        async fn should_not_be_shorter_than_160_bytes() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_into_value("instantLock", vec![0u8; 159])
                .unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/instantLock");
            assert_eq!(error.keyword(), "minItems");
        }

        #[tokio::test]
        async fn should_not_be_longer_than_100_kb() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_into_value("instantLock", vec![0u8; 100001])
                .unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/instantLock");
            assert_eq!(error.keyword(), "maxItems");
        }

        #[tokio::test]
        async fn should_be_valid() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_into_value("instantLock", vec![0u8; 200])
                .unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            assert_basic_consensus_errors!(
                result,
                BasicError::InvalidInstantAssetLockProofError,
                1
            );

            let consensus_error = result.first_error().unwrap();
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
                .validate(
                    &asset_lock_proof.to_raw_object().unwrap(),
                    &Default::default(),
                )
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(
                result,
                BasicError::IdentityAssetLockProofLockedTransactionMismatchError,
                1
            );

            let error = errors.first().unwrap();
            let consensus_error = result.first_error().unwrap();
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
                .returning(|_asset_lock, _| Ok(false));
            let test_data = setup_test(Some(state_repository_mock));

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            assert!(!result.is_valid());

            assert_basic_consensus_errors!(
                result,
                BasicError::InvalidInstantAssetLockProofSignatureError,
                1
            );

            let consensus_error = result.first_error().unwrap();
            assert_eq!(consensus_error.code(), 1042);
        }
    }

    mod transaction {
        use super::*;
        use crate::assert_basic_consensus_errors;
        use crate::consensus::codes::ErrorWithCode;

        #[tokio::test]
        async fn should_be_present() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.remove("transaction").unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "transaction");
        }

        #[tokio::test]
        async fn should_be_a_byte_array() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_into_value("instantLock", vec!["string"; 65])
                .unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 66);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/instantLock/0");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        async fn should_not_be_shorter_than_1_byte() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_into_value("instantLock", vec![0u8; 0])
                .unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/instantLock");
            assert_eq!(error.keyword(), "minItems");
        }

        #[tokio::test]
        async fn should_not_be_longer_than_100_kb() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_into_value("instantLock", vec![0u8; 100001])
                .unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path(), "/instantLock");
            assert_eq!(error.keyword(), "maxItems");
        }

        #[tokio::test]
        async fn should_should_be_valid() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_into_value("transaction", vec![0u8; 64])
                .unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            assert_basic_consensus_errors!(
                result,
                BasicError::InvalidIdentityAssetLockTransactionError,
                1
            );

            let consensus_error = result.first_error().unwrap();
            assert_eq!(consensus_error.code(), 1038);
        }
    }

    mod output_index {
        use super::*;
        use crate::assert_basic_consensus_errors;

        #[tokio::test]
        async fn should_be_present() {
            let mut test_data = setup_test(None);
            test_data.raw_proof.remove("outputIndex").unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "");
            assert_eq!(error.keyword(), "required");
            assert_eq!(error.property_name(), "outputIndex");
        }

        #[tokio::test]
        async fn should_be_an_integer() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_into_value("outputIndex", 1.1)
                .unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/outputIndex");
            assert_eq!(error.keyword(), "type");
        }

        #[tokio::test]
        async fn should_not_be_less_than_0() {
            let mut test_data = setup_test(None);
            test_data
                .raw_proof
                .set_into_value("outputIndex", -1)
                .unwrap();

            let result = test_data
                .validate_instant_asset_lock_proof_structure
                .validate(&test_data.raw_proof, &Default::default())
                .await
                .unwrap();

            let errors = assert_basic_consensus_errors!(result, BasicError::JsonSchemaError, 1);
            let error = errors.first().unwrap();

            assert_eq!(error.instance_path().to_string(), "/outputIndex");
            assert_eq!(error.keyword(), "minimum");
        }
    }

    #[tokio::test]
    async fn should_return_a_valid_result() {
        let test_data = setup_test(None);

        let result = test_data
            .validate_instant_asset_lock_proof_structure
            .validate(&test_data.raw_proof, &Default::default())
            .await
            .unwrap();

        assert!(result.is_valid());
        assert_eq!(
            result.data_as_borrowed().expect("expected data").to_vec(),
            test_data.public_key_hash
        );
    }
}
