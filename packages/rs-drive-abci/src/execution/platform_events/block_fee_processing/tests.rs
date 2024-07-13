

#[cfg(test)]
mod refund_tests {
    use crate::execution::validation::state_transition::tests::{fast_forward_to_block, setup_identity, fetch_expected_identity_balance, setup_identity_with_system_credits, process_state_transitions};
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::document::document_methods::DocumentMethodsV0;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use dpp::document::{Document, DocumentV0Setters};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::platform_value::Bytes32;
        use assert_matches::assert_matches;
    use dpp::data_contract::document_type::DocumentTypeRef;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::fee::fee_result::FeeResult;
    use dpp::identity::{Identity, IdentityPublicKey};
    use drive::common::setup_contract;
    use simple_signer::signer::SimpleSigner;
    use crate::expect_match;
    use crate::rpc::core::MockCoreRPCLike;

    fn setup_initial_document<'a>(platform: &TempPlatform<MockCoreRPCLike>, profile: DocumentTypeRef, rng: &mut StdRng, identity: &'a Identity, key: &IdentityPublicKey,
                              signer: &SimpleSigner) -> (Document, FeeResult) {
        let platform_version = PlatformVersion::latest();

        let platform_state = platform.state.load();

        assert!(profile.documents_mutable());

        let entropy = Bytes32::random_with_rng(rng);

        let mut document = profile
            .random_document_with_identifier_and_entropy(
                rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("avatarUrl", "http://test.com/bob.jpg".into());

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("displayName", "Samuel".into());
        altered_document.set("avatarUrl", "http://test.com/cat.jpg".into());

        let serialized_len = document.serialize(profile, platform_version).expect("expected to serialize").len() as u64;

        let documents_batch_create_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                profile,
                entropy.0,
                key,
                2,
                0,
                signer,
                platform_version,
                None,
                None,
                None,
            )
                .expect("expect to create documents batch transition");

        let (mut fee_results, processed_block_fee_outcome) = process_state_transitions(&platform, &vec![documents_batch_create_transition.clone()], BlockInfo::default(), &platform_state);

        let fee_result = fee_results.remove(0);

        let credits_verified = platform.platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive).expect("expected to check sum trees");

        let balanced = credits_verified.ok().expect("expected that credits will balance when we remove in same block");

        assert!(balanced, "platform should be balanced {}", credits_verified);

        let added_bytes = fee_result.storage_fee / platform_version.fee_version.storage.storage_disk_usage_credit_per_byte;

        assert_eq!(added_bytes, serialized_len);

        assert_eq!(fee_result.storage_fee, 24570000);

        assert_eq!(fee_result.storage_fee,processed_block_fee_outcome.fees_in_pools.storage_fees);

        assert_eq!(fee_result.processing_fee,processed_block_fee_outcome.fees_in_pools.processing_fees);

        let expected_user_balance_after_creation = dash_to_credits!(1) - fee_result.total_base_fee();

        fetch_expected_identity_balance(&platform, identity.id(), platform_version, expected_user_balance_after_creation);

        (document, fee_result)
    }

    #[test]
        fn test_document_refund_immediate() {
            let platform_version = PlatformVersion::latest();
            let mut platform = TestPlatformBuilder::new()
                .build_with_mock_rpc()
                .set_genesis_state();

        let dashpay_contract_no_indexes = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-no-indexes.json",
            None,
            None,
        );

        let profile = dashpay_contract_no_indexes
            .document_type_for_name("profile")
            .expect("expected a profile document type");


        let mut rng = StdRng::seed_from_u64(433);

            let platform_state = platform.state.load();

            let (identity, signer, key) = setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

            fetch_expected_identity_balance(&platform, identity.id(), platform_version, dash_to_credits!(1));
            
            let (document, insertion_fee_result) = setup_initial_document(&platform, profile, &mut rng, &identity, &key, &signer);

        let documents_batch_delete_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
                .expect("expect to create documents batch transition");

        let (mut fee_results, processed_block_fee_outcome) = process_state_transitions(&platform, &vec![documents_batch_delete_transition.clone()], BlockInfo::default(), &platform_state);

        let fee_result = fee_results.remove(0);

        let credits_verified = platform.platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive).expect("expected to check sum trees");

        let balanced = credits_verified.ok().expect("expected that credits will balance when we remove in same block");

        assert!(balanced, "platform should be balanced {}", credits_verified);

        let refund_amount = fee_result.fee_refunds.calculate_refunds_amount_for_identity(identity.id()).expect("expected refunds for identity");

        assert_eq!(refund_amount, insertion_fee_result.storage_fee);


        let storage_cost = fee_result.storage_fee;

        assert_eq!(fee_result.storage_fee,processed_block_fee_outcome.fees_in_pools.storage_fees);

        assert_eq!(fee_result.processing_fee,processed_block_fee_outcome.fees_in_pools.processing_fees);

        let expected_user_balance_after_creation = dash_to_credits!(1) - fee_result.total_base_fee();

        fetch_expected_identity_balance(&platform, identity.id(), platform_version, expected_user_balance_after_creation);

        
        let expected_user_balance_after_deletion = expected_user_balance_after_creation - fee_result.processing_fee + refund_amount;

        fetch_expected_identity_balance(&platform, identity.id(), platform_version, expected_user_balance_after_deletion);
        }

    #[test]
    fn test_document_refund_a_few_blocks_later() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(&platform, identity.id(), platform_version, dash_to_credits!(1));

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        assert!(profile.documents_mutable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = profile
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("avatarUrl", "http://test.com/bob.jpg".into());

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("displayName", "Samuel".into());
        altered_document.set("avatarUrl", "http://test.com/cat.jpg".into());

        let documents_batch_create_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                profile,
                entropy.0,
                &key,
                2,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
                .expect("expect to create documents batch transition");

        let documents_batch_create_serialized_transition = documents_batch_create_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_create_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

        let fee_result = expect_match!(processing_result.into_execution_results().remove(0), StateTransitionExecutionResult::SuccessfulExecution(_, fee_result) => fee_result);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let storage_cost = fee_result.storage_fee;

        let expected_user_balance_after_creation = dash_to_credits!(1) - fee_result.total_base_fee();

        fetch_expected_identity_balance(&platform, identity.id(), platform_version, expected_user_balance_after_creation);


        fast_forward_to_block(&platform, 60_000, 5); //a few blocks later

        let documents_batch_delete_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                3,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
                .expect("expect to create documents batch transition");

        let documents_batch_delete_serialized_transition = documents_batch_delete_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_delete_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
            )
            .expect("expected to process state transition");

        assert_matches!(
                processing_result.execution_results().as_slice(),
                [StateTransitionExecutionResult::SuccessfulExecution(_, _)]
            );

        let fee_result = expect_match!(processing_result.into_execution_results().remove(0), StateTransitionExecutionResult::SuccessfulExecution(_, fee_result) => fee_result);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let refund_amount = fee_result.fee_refunds.calculate_refunds_amount_for_identity(identity.id()).expect("expected refunds for identity");

        assert_eq!(refund_amount, storage_cost);

        let expected_user_balance_after_deletion = expected_user_balance_after_creation - fee_result.processing_fee + refund_amount;

        fetch_expected_identity_balance(&platform, identity.id(), platform_version, expected_user_balance_after_deletion);

        fast_forward_to_block(&platform, 5_000_000, 50); //a few blocks later


        let platform_state = platform.state.load();
    }
}