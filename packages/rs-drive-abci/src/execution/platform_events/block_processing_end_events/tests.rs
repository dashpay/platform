#[cfg(test)]
mod refund_tests {
    use crate::execution::validation::state_transition::tests::{
        fetch_expected_identity_balance, process_state_transitions,
        setup_identity_with_system_credits,
    };
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::data_contract::document_type::DocumentTypeRef;
    use dpp::data_contract::DataContract;
    use dpp::document::document_methods::DocumentMethodsV0;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::{Document, DocumentV0Setters};
    use dpp::fee::fee_result::FeeResult;
    use dpp::fee::Credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::Bytes32;
    use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::batch_transition::BatchTransition;
    use drive::util::test_helpers::setup_contract;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::ops::Deref;

    // There's a fee for the first document that a user creates on a contract as they add space
    // For the identity data contract nonce
    async fn setup_join_contract_document(
        platform: &TempPlatform<MockCoreRPCLike>,
        profile: DocumentTypeRef<'_>,
        rng: &mut StdRng,
        identity: &Identity,
        key: &IdentityPublicKey,
        signer: &SimpleSigner,
    ) -> Credits {
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
        document
            .set_id_for_creation(profile, &entropy.0, 2, platform_version)
            .expect("expected to set the document id");

        document.set("avatarUrl", "http://test.com/ivan.jpg".into());

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("displayName", "Ivan".into());
        altered_document.set("avatarUrl", "http://test.com/dog.jpg".into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                profile,
                entropy.0,
                key,
                2,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let (mut fee_results, processed_block_fee_outcome) = process_state_transitions(
            platform,
            &vec![documents_batch_create_transition.clone()],
            BlockInfo::default(),
            &platform_state,
        );

        let fee_result = fee_results.remove(0);

        let credits_verified = platform
            .platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to check sum trees");

        let balanced = credits_verified
            .ok()
            .expect("expected that credits will balance when we remove in same block");

        assert!(balanced, "platform should be balanced {}", credits_verified);

        assert_eq!(
            fee_result.storage_fee,
            processed_block_fee_outcome.fees_in_pools.storage_fees
        );

        assert_eq!(
            fee_result.processing_fee,
            processed_block_fee_outcome.fees_in_pools.processing_fees
        );

        let expected_user_balance_after_creation =
            dash_to_credits!(1) - fee_result.total_base_fee();

        fetch_expected_identity_balance(
            platform,
            identity.id(),
            platform_version,
            expected_user_balance_after_creation,
        );

        expected_user_balance_after_creation
    }

    async fn setup_initial_document(
        platform: &TempPlatform<MockCoreRPCLike>,
        dashpay: &DataContract,
        profile: DocumentTypeRef<'_>,
        rng: &mut StdRng,
        identity: &Identity,
        key: &IdentityPublicKey,
        signer: &SimpleSigner,
    ) -> (Document, FeeResult, Credits) {
        // Let's make another document first just so the operations of joining a contract are out of the way
        // (A user pays to add some data to the state on the first time they make their first document for a contract)
        let user_credits_left =
            setup_join_contract_document(platform, profile, rng, identity, key, signer).await;

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
        document
            .set_id_for_creation(profile, &entropy.0, 3, platform_version)
            .expect("expected to set the document id");

        document.set("avatarUrl", "http://test.com/bob.jpg".into());

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("displayName", "Samuel".into());
        altered_document.set("avatarUrl", "http://test.com/cat.jpg".into());

        let serialized_len = document
            .serialize(profile, dashpay, platform_version)
            .expect("expected to serialize")
            .len() as u64;

        assert_eq!(serialized_len, 173);

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                profile,
                entropy.0,
                key,
                3,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let (mut fee_results, _) = process_state_transitions(
            platform,
            &vec![documents_batch_create_transition.clone()],
            BlockInfo::default(),
            &platform_state,
        );

        let fee_result = fee_results.remove(0);

        let credits_verified = platform
            .platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to check sum trees");

        let balanced = credits_verified
            .ok()
            .expect("expected that credits will balance when we remove in same block");

        assert!(balanced, "platform should be balanced {}", credits_verified);

        assert_eq!(fee_result.storage_fee, 11124000);

        let added_bytes = fee_result.storage_fee
            / platform_version
                .fee_version
                .storage
                .storage_disk_usage_credit_per_byte;

        // Key -> 65 bytes
        // 32 bytes for the key prefix
        // 32 bytes for the unique id
        // 1 byte for key_size (required space for 64)

        // Value -> 279
        //   1 for the flag option with flags
        //   1 for the flags size
        //   35 for flags 32 + 1 + 2
        //   1 for the enum type
        //   1 for item
        //   173 for item serialized bytes (verified above)
        //   1 for Basic Merk
        // 32 for node hash
        // 32 for value hash
        // 2 byte for the value_size (required space for above 128)

        // Parent Hook -> 68
        // Key Bytes 32
        // Hash Size 32
        // Key Length 1
        // Child Heights 2
        // Basic Merk 1

        assert_eq!(added_bytes, 65 + 279 + 68);

        let expected_user_balance_after_creation = user_credits_left - fee_result.total_base_fee();

        fetch_expected_identity_balance(
            platform,
            identity.id(),
            platform_version,
            expected_user_balance_after_creation,
        );

        (document, fee_result, expected_user_balance_after_creation)
    }

    #[tokio::test]
    async fn test_document_refund_immediate() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let dashpay_contract_no_indexes = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-no-indexes.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let profile = dashpay_contract_no_indexes
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            dash_to_credits!(1),
        );

        let (document, insertion_fee_result, current_user_balance) = setup_initial_document(
            &platform,
            &dashpay_contract_no_indexes,
            profile,
            &mut rng,
            &identity,
            &key,
            &signer,
        )
        .await;

        let documents_batch_delete_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let (mut fee_results, _) = process_state_transitions(
            &platform,
            &vec![documents_batch_delete_transition.clone()],
            BlockInfo::default(),
            &platform_state,
        );

        let fee_result = fee_results.remove(0);

        let credits_verified = platform
            .platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to check sum trees");

        let balanced = credits_verified
            .ok()
            .expect("expected that credits will balance when we remove in same block");

        assert!(balanced, "platform should be balanced {}", credits_verified);

        let refund_amount = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(identity.id())
            .expect("expected refunds for identity");

        // we should be refunding more than 99%
        let lower_bound = insertion_fee_result.storage_fee * 99 / 100;
        assert!(refund_amount > lower_bound, "expected the refund amount to be more than 99% of the storage cost, as it is for just one out of 2000 epochs");
        assert!(
            refund_amount < insertion_fee_result.storage_fee,
            "expected the refund amount to be less than the insertion cost"
        );

        assert_eq!(fee_result.storage_fee, 0);

        let expected_user_balance_after_deletion =
            current_user_balance - fee_result.total_base_fee() + refund_amount;

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            expected_user_balance_after_deletion,
        );
    }
    #[tokio::test]
    async fn test_document_refund_after_an_epoch() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let dashpay_contract_no_indexes = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-no-indexes.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let profile = dashpay_contract_no_indexes
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let mut rng = StdRng::seed_from_u64(433);

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            dash_to_credits!(1),
        );

        let (document, insertion_fee_result, current_user_balance) = setup_initial_document(
            &platform,
            &dashpay_contract_no_indexes,
            profile,
            &mut rng,
            &identity,
            &key,
            &signer,
        )
        .await;

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let documents_batch_delete_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let platform_state = platform.state.load();

        let (mut fee_results, _) = process_state_transitions(
            &platform,
            &vec![documents_batch_delete_transition.clone()],
            *platform_state.last_block_info(),
            &platform_state,
        );

        let fee_result = fee_results.remove(0);

        let credits_verified = platform
            .platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to check sum trees");

        let balanced = credits_verified
            .ok()
            .expect("expected that credits will balance when we remove in same block");

        assert!(balanced, "platform should be balanced {}", credits_verified);

        let refund_amount = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(identity.id())
            .expect("expected refunds for identity");

        // we should be refunding more than 99% still
        let lower_bound = insertion_fee_result.storage_fee * 99 / 100;
        assert!(refund_amount > lower_bound, "expected the refund amount to be more than 99% of the storage cost, as it is for just one out of 2000 epochs");
        assert!(
            refund_amount < insertion_fee_result.storage_fee,
            "expected the refund amount to be less than the insertion cost"
        );

        assert_eq!(fee_result.storage_fee, 0);

        let expected_user_balance_after_deletion =
            current_user_balance - fee_result.total_base_fee() + refund_amount;

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            expected_user_balance_after_deletion,
        );
    }

    #[tokio::test]
    async fn test_document_refund_after_a_year() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let dashpay_contract_no_indexes = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-no-indexes.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let profile = dashpay_contract_no_indexes
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let mut rng = StdRng::seed_from_u64(433);

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            dash_to_credits!(1),
        );

        let (document, insertion_fee_result, current_user_balance) = setup_initial_document(
            &platform,
            &dashpay_contract_no_indexes,
            profile,
            &mut rng,
            &identity,
            &key,
            &signer,
        )
        .await;

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 40, false); //a year later

        let documents_batch_delete_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let platform_state = platform.state.load();

        let (mut fee_results, _) = process_state_transitions(
            &platform,
            &vec![documents_batch_delete_transition.clone()],
            *platform_state.last_block_info(),
            &platform_state,
        );

        let fee_result = fee_results.remove(0);

        let credits_verified = platform
            .platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to check sum trees");

        let balanced = credits_verified
            .ok()
            .expect("expected that credits will balance when we remove in same block");

        assert!(balanced, "platform should be balanced {}", credits_verified);

        let refund_amount = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(identity.id())
            .expect("expected refunds for identity");

        // we should be refunding around 94% after a year.
        let refunded_percentage = refund_amount * 100 / insertion_fee_result.storage_fee;
        assert_eq!(refunded_percentage, 94);

        assert_eq!(fee_result.storage_fee, 0);

        let expected_user_balance_after_deletion =
            current_user_balance - fee_result.total_base_fee() + refund_amount;

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            expected_user_balance_after_deletion,
        );
    }

    #[tokio::test]
    async fn test_document_refund_after_25_years() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let dashpay_contract_no_indexes = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-no-indexes.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let profile = dashpay_contract_no_indexes
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let mut rng = StdRng::seed_from_u64(433);

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            dash_to_credits!(1),
        );

        let (document, insertion_fee_result, current_user_balance) = setup_initial_document(
            &platform,
            &dashpay_contract_no_indexes,
            profile,
            &mut rng,
            &identity,
            &key,
            &signer,
        )
        .await;

        fast_forward_to_block(&platform, 10_200_000_000, 9000, 42, 40 * 25, false); //25 years later

        let documents_batch_delete_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let platform_state = platform.state.load();

        let (mut fee_results, _) = process_state_transitions(
            &platform,
            &vec![documents_batch_delete_transition.clone()],
            *platform_state.last_block_info(),
            &platform_state,
        );

        let fee_result = fee_results.remove(0);

        let credits_verified = platform
            .platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to check sum trees");

        let balanced = credits_verified
            .ok()
            .expect("expected that credits will balance when we remove in same block");

        assert!(balanced, "platform should be balanced {}", credits_verified);

        let refund_amount = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(identity.id())
            .expect("expected refunds for identity");

        // we should be refunding around 21% after 25 years.
        let refunded_percentage = refund_amount * 100 / insertion_fee_result.storage_fee;
        assert_eq!(refunded_percentage, 21);

        assert_eq!(fee_result.storage_fee, 0);

        let expected_user_balance_after_deletion =
            current_user_balance - fee_result.total_base_fee() + refund_amount;

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            expected_user_balance_after_deletion,
        );
    }

    #[tokio::test]
    async fn test_document_refund_after_50_years() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let dashpay_contract_no_indexes = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-no-indexes.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let profile = dashpay_contract_no_indexes
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let mut rng = StdRng::seed_from_u64(433);

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            dash_to_credits!(1),
        );

        let (document, _, current_user_balance) = setup_initial_document(
            &platform,
            &dashpay_contract_no_indexes,
            profile,
            &mut rng,
            &identity,
            &key,
            &signer,
        )
        .await;

        fast_forward_to_block(&platform, 10_200_000_000, 9000, 42, 40 * 50, false); //50 years later

        let documents_batch_delete_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let platform_state = platform.state.load();

        let (mut fee_results, _) = process_state_transitions(
            &platform,
            &vec![documents_batch_delete_transition.clone()],
            *platform_state.last_block_info(),
            &platform_state,
        );

        let fee_result = fee_results.remove(0);

        let credits_verified = platform
            .platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to check sum trees");

        let balanced = credits_verified
            .ok()
            .expect("expected that credits will balance when we remove in same block");

        assert!(balanced, "platform should be balanced {}", credits_verified);

        let refund_amount = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(identity.id())
            .expect("expected refunds for identity");

        // we should be refunding nothing after 50 years.
        assert_eq!(refund_amount, 0);

        assert_eq!(fee_result.storage_fee, 0);

        let expected_user_balance_after_deletion =
            current_user_balance - fee_result.total_base_fee() + refund_amount;

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            expected_user_balance_after_deletion,
        );
    }

    #[tokio::test]
    async fn test_document_refund_after_10_epochs_on_different_fee_version_increasing_fees() {
        let platform_version = PlatformVersion::latest();
        let platform_version_with_higher_fees = platform_version.clone();

        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let dashpay_contract_no_indexes = setup_contract(
            &platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-no-indexes.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );

        let profile = dashpay_contract_no_indexes
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let mut rng = StdRng::seed_from_u64(433);

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            dash_to_credits!(1),
        );

        let (document, insertion_fee_result, current_user_balance) = setup_initial_document(
            &platform,
            &dashpay_contract_no_indexes,
            profile,
            &mut rng,
            &identity,
            &key,
            &signer,
        )
        .await;

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 10, false); //next epoch

        let documents_batch_delete_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                None,
                &signer,
                &platform_version_with_higher_fees,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let mut platform_state = platform.state.load().clone().deref().clone();

        platform_state
            .previous_fee_versions_mut()
            .insert(5, platform_version_with_higher_fees.fee_version.as_static());

        let (mut fee_results, _) = process_state_transitions(
            &platform,
            &vec![documents_batch_delete_transition.clone()],
            *platform_state.last_block_info(),
            &platform_state,
        );

        let fee_result = fee_results.remove(0);

        let credits_verified = platform
            .platform
            .drive
            .calculate_total_credits_balance(None, &platform_version_with_higher_fees.drive)
            .expect("expected to check sum trees");

        let balanced = credits_verified
            .ok()
            .expect("expected that credits will balance when we remove in same block");

        assert!(balanced, "platform should be balanced {}", credits_verified);

        let refund_amount = fee_result
            .fee_refunds
            .calculate_refunds_amount_for_identity(identity.id())
            .expect("expected refunds for identity");

        // we should be refunding around 21% after 25 years.
        let refunded_percentage = refund_amount * 100 / insertion_fee_result.storage_fee;
        assert_eq!(refunded_percentage, 98);

        assert_eq!(fee_result.storage_fee, 0);

        let expected_user_balance_after_deletion =
            current_user_balance - fee_result.total_base_fee() + refund_amount;

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            &platform_version_with_higher_fees,
            expected_user_balance_after_deletion,
        );
    }
}

#[cfg(test)]
mod storage_refund_clawback_tests {
    use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0;
    use crate::execution::types::block_execution_context::BlockExecutionContext;
    use crate::execution::types::block_fees::v0::BlockFeesV0;
    use crate::execution::types::block_state_info::v0::BlockStateInfoV0;
    use crate::execution::types::block_state_info::BlockStateInfo;
    use crate::execution::validation::state_transition::tests::setup_identity_with_system_credits;
    use crate::platform_types::epoch_info::v0::EpochInfoV0;
    use crate::platform_types::epoch_info::EpochInfo;
    use crate::platform_types::platform_state::PlatformStateV0Methods;
    use crate::platform_types::state_transitions_processing_result::StateTransitionExecutionResult;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
    use dpp::block::block_info::BlockInfo;
    use dpp::block::epoch::{Epoch, EpochIndex};
    use dpp::dash_to_credits;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        CreateRandomDocument, DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::data_contract::DataContract;
    use dpp::document::DocumentV0Setters;
    use dpp::fee::epoch::distribution::{
        calculate_storage_fee_refund_amount_and_leftovers,
        distribute_storage_fee_to_epochs_collection,
    };
    use dpp::fee::epoch::{perpetual_storage_epochs, CreditsPerEpoch, SignedCreditsPerEpoch};
    use dpp::fee::fee_result::FeeResult;
    use dpp::fee::{Credits, SignedCredits};
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::platform_value::Bytes32;
    use dpp::serialization::PlatformSerializable;
    use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::state_transition::StateTransition;
    use dpp::version::ProtocolVersion;
    use drive::util::test_helpers::setup_contract;
    use drive::util::test_helpers::test_utils::identities::create_test_masternode_identities;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use std::collections::{BTreeMap, BTreeSet};
    use std::ops::Range;

    /// A chain driven block by block through state transition processing and the real block
    /// end fee processing, so storage is paid, refunded and clawed back as on a network.
    struct RefundChain {
        platform: TempPlatform<MockCoreRPCLike>,
        proposer: [u8; 32],
        height: u64,
        last_epoch_index: Option<EpochIndex>,
        last_block_time_ms: Option<u64>,
    }

    impl RefundChain {
        fn new(protocol_version: ProtocolVersion) -> Self {
            let platform = TestPlatformBuilder::new()
                .with_initial_protocol_version(protocol_version)
                .build_with_mock_rpc()
                .set_genesis_state();

            let platform_version =
                PlatformVersion::get(protocol_version).expect("expected a known version");

            let proposer = create_test_masternode_identities(
                &platform.drive,
                1,
                Some(17),
                None,
                platform_version,
            )[0];

            Self {
                platform,
                proposer,
                height: 0,
                last_epoch_index: None,
                last_block_time_ms: None,
            }
        }

        fn platform_version(&self) -> &'static PlatformVersion {
            self.platform
                .state
                .load()
                .current_platform_version()
                .expect("expected the state's platform version")
        }

        fn epochs_per_era(&self) -> u16 {
            self.platform.config.drive.epochs_per_era
        }

        /// Runs one block in `epoch_index`. It is an epoch change when the previous block was in
        /// another epoch, with that epoch as the previous one, as `EpochInfoV0::calculate` sets
        /// it on a network.
        fn run_block(
            &mut self,
            epoch_index: EpochIndex,
            state_transitions: &[StateTransition],
        ) -> Vec<StateTransitionExecutionResult> {
            let platform_state = self.platform.state.load();
            let platform_version = self.platform_version();

            self.height += 1;

            let epoch_time_ms = self.platform.config.execution.epoch_time_length_s * 1000;
            let block_time_ms = epoch_index as u64 * epoch_time_ms + self.height * 1000;
            let core_height = self.height as u32 + 1;

            let epoch_info = match self.last_epoch_index {
                None => EpochInfoV0 {
                    current_epoch_index: epoch_index,
                    previous_epoch_index: None,
                    is_epoch_change: true,
                },
                Some(last_epoch_index) if last_epoch_index != epoch_index => EpochInfoV0 {
                    current_epoch_index: epoch_index,
                    previous_epoch_index: Some(last_epoch_index),
                    is_epoch_change: true,
                },
                Some(_) => EpochInfoV0 {
                    current_epoch_index: epoch_index,
                    previous_epoch_index: None,
                    is_epoch_change: false,
                },
            };

            let block_info = BlockInfo {
                time_ms: block_time_ms,
                height: self.height,
                core_height,
                epoch: Epoch::new(epoch_index).expect("expected a valid epoch"),
            };

            let raw_state_transitions = state_transitions
                .iter()
                .map(|state_transition| {
                    state_transition
                        .serialize_to_bytes()
                        .expect("expected to serialize")
                })
                .collect::<Vec<_>>();

            let transaction = self.platform.drive.grove.start_transaction();

            let processing_result = self
                .platform
                .platform
                .process_raw_state_transitions(
                    &raw_state_transitions,
                    &platform_state,
                    &block_info,
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transitions");

            let block_fees: BlockFeesV0 = processing_result.aggregated_fees().clone().into();

            let block_execution_context = BlockExecutionContext::V0(BlockExecutionContextV0 {
                block_state_info: BlockStateInfo::V0(BlockStateInfoV0 {
                    height: self.height,
                    round: 0,
                    block_time_ms,
                    previous_block_time_ms: self.last_block_time_ms,
                    proposer_pro_tx_hash: self.proposer,
                    core_chain_locked_height: core_height,
                    block_hash: None,
                    app_hash: None,
                }),
                epoch_info: EpochInfo::V0(epoch_info),
                unsigned_withdrawal_transactions: Default::default(),
                block_address_balance_changes: Default::default(),
                block_platform_state: (**platform_state).clone(),
                proposer_results: None,
            });

            // verify_sum_trees is on by default, so this fails if the credits do not balance
            self.platform
                .process_block_fees_and_validate_sum_trees(
                    &block_execution_context,
                    block_fees.into(),
                    &transaction,
                    platform_version,
                )
                .expect("expected to process block fees with balanced credits");

            self.platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");

            fast_forward_to_block(
                &self.platform,
                block_time_ms,
                self.height,
                core_height,
                epoch_index,
                false,
            );

            self.last_epoch_index = Some(epoch_index);
            self.last_block_time_ms = Some(block_time_ms);

            processing_result.into_execution_results()
        }

        fn epoch_storage_pools(&self, epochs: Range<EpochIndex>) -> BTreeMap<EpochIndex, Credits> {
            let platform_version = self.platform_version();

            // Paid epochs have no pool anymore
            epochs
                .filter_map(|epoch_index| {
                    self.platform
                        .drive
                        .get_epoch_storage_credits_for_distribution(
                            &Epoch::new(epoch_index).expect("expected a valid epoch"),
                            None,
                            platform_version,
                        )
                        .ok()
                        .map(|credits| (epoch_index, credits))
                })
                .collect()
        }
    }

    fn successful_fee_result(result: &StateTransitionExecutionResult) -> FeeResult {
        match result {
            StateTransitionExecutionResult::SuccessfulExecution { fee_result, .. } => {
                fee_result.clone()
            }
            other => panic!("expected a successful execution, got {:?}", other),
        }
    }

    /// What the epoch change after a removal did to the epoch storage pools, next to the shares
    /// the removal's refund was priced from.
    struct Clawback {
        refund: Credits,
        /// Per epoch: credits the pool lost to the refund
        clawed_back: BTreeMap<EpochIndex, SignedCredits>,
        /// Per epoch: the share the refund returned for it, with the shares of epochs that
        /// closed before the clawback epoch (skipped epochs) assigned to the clawback epoch
        priced: BTreeMap<EpochIndex, SignedCredits>,
    }

    impl Clawback {
        /// Epochs where the pool lost something other than the share the refund paid out for it
        fn errors(&self) -> BTreeMap<EpochIndex, SignedCredits> {
            let epochs = self
                .clawed_back
                .keys()
                .chain(self.priced.keys())
                .copied()
                .collect::<BTreeSet<_>>();

            epochs
                .into_iter()
                .filter_map(|epoch_index| {
                    let error = self.clawed_back.get(&epoch_index).copied().unwrap_or(0)
                        - self.priced.get(&epoch_index).copied().unwrap_or(0);
                    (error != 0).then_some((epoch_index, error))
                })
                .collect()
        }

        fn moved(&self) -> SignedCredits {
            self.errors()
                .values()
                .filter(|error| **error > 0)
                .sum::<SignedCredits>()
        }

        fn describe(&self) -> String {
            // Group consecutive epochs with the same error into runs
            let mut runs: Vec<(EpochIndex, EpochIndex, SignedCredits)> = vec![];
            for (epoch_index, error) in self.errors() {
                match runs.last_mut() {
                    Some((_, end, run_error)) if *end + 1 == epoch_index && *run_error == error => {
                        *end = epoch_index
                    }
                    _ => runs.push((epoch_index, epoch_index, error)),
                }
            }
            let total_clawed_back: SignedCredits = self.clawed_back.values().sum();
            let mut out = format!(
                "refund {}, clawed back {}, moved {} credits, {} error runs\n",
                self.refund,
                total_clawed_back,
                self.moved(),
                runs.len()
            );
            for (start, end, error) in runs.iter().take(12) {
                out.push_str(&format!(
                    "  epochs {start}..={end}: clawed back {error:+} vs priced share ({})\n",
                    self.priced.get(start).copied().unwrap_or(0)
                ));
            }
            if runs.len() > 12 {
                out.push_str(&format!("  ... {} more runs\n", runs.len() - 12));
            }
            out
        }
    }

    /// Stores a document in `store_epoch`, removes it in `remove_epoch` (in that epoch's first
    /// block when `remove_in_epoch_change_block`) and crosses the epoch change into
    /// `clawback_epoch`, measuring what that change took from each epoch storage pool.
    async fn store_remove_and_claw_back(
        protocol_version: ProtocolVersion,
        store_epoch: EpochIndex,
        remove_epoch: EpochIndex,
        remove_in_epoch_change_block: bool,
        clawback_epoch: EpochIndex,
    ) -> Clawback {
        assert!(store_epoch <= remove_epoch && remove_epoch < clawback_epoch);

        let mut chain = RefundChain::new(protocol_version);
        let platform_version = chain.platform_version();
        let epochs_per_era = chain.epochs_per_era();

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut chain.platform, 958, dash_to_credits!(1));

        let dashpay = setup_contract(
            &chain.platform.drive,
            "tests/supporting_files/contract/dashpay/dashpay-contract-no-indexes.json",
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            Some(platform_version),
        );

        let profile = dashpay
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let mut rng = StdRng::seed_from_u64(433);

        // Genesis block, then the first block of the storage epoch
        chain.run_block(0, &[]);
        if store_epoch > 0 {
            chain.run_block(store_epoch, &[]);
        }

        // The removed document, stored next to others so the epoch pools hold more than its
        // shares, as they do on a network
        let mut documents = vec![];
        let mut creates = vec![];
        for nonce in 2..=6 {
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
            document
                .set_id_for_creation(profile, &entropy.0, nonce, platform_version)
                .expect("expected to set the document id");
            document.set("avatarUrl", "http://test.com/bob.jpg".into());

            creates.push(
                BatchTransition::new_document_creation_transition_from_document(
                    document.clone(),
                    profile,
                    entropy.0,
                    &key,
                    nonce,
                    0,
                    None,
                    &signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expected a create transition"),
            );
            documents.push(document);
        }

        let results = chain.run_block(store_epoch, &creates);
        results.iter().for_each(|result| {
            successful_fee_result(result);
        });

        let delete = BatchTransition::new_document_deletion_transition_from_document(
            documents.remove(0),
            profile,
            &key,
            7,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected a delete transition");

        if remove_epoch > store_epoch && !remove_in_epoch_change_block {
            chain.run_block(remove_epoch, &[]);
        }
        let results = chain.run_block(remove_epoch, &[delete]);
        let delete_fee_result = successful_fee_result(&results[0]);

        let refunds: CreditsPerEpoch = delete_fee_result
            .fee_refunds
            .get(identity.id().as_bytes())
            .cloned()
            .expect("expected a refund for the owner");
        assert_eq!(
            refunds.keys().copied().collect::<Vec<_>>(),
            vec![store_epoch],
            "the refund is keyed by the epoch the storage was paid in"
        );
        let refund = refunds[&store_epoch];

        // Every refund waiting for the next epoch change was priced in the removal epoch
        let pending_refunds = chain
            .platform
            .drive
            .fetch_pending_epoch_refunds(None, &platform_version.drive)
            .expect("expected pending refunds");
        assert_eq!(
            pending_refunds,
            CreditsPerEpoch::from_iter([(store_epoch, refund)])
        );

        // The removed bytes the refund was priced from: the refund is the removal-epoch price of
        // those bytes' shares of every epoch after the removal epoch
        let credits_per_byte = platform_version
            .fee_version
            .storage
            .storage_disk_usage_credit_per_byte;
        let removed_storage_fee_candidates = (1..=10_000u64)
            .map(|bytes| bytes * credits_per_byte)
            .filter(|storage_fee| {
                calculate_storage_fee_refund_amount_and_leftovers(
                    *storage_fee,
                    store_epoch,
                    remove_epoch,
                    epochs_per_era,
                )
                .expect("expected to price the refund")
                .0 == refund
            })
            .collect::<Vec<_>>();
        assert_eq!(
            removed_storage_fee_candidates.len(),
            1,
            "expected one removed byte count to price this refund"
        );
        let removed_storage_fee = removed_storage_fee_candidates[0];

        let mut shares = SignedCreditsPerEpoch::default();
        distribute_storage_fee_to_epochs_collection(
            &mut shares,
            removed_storage_fee,
            store_epoch,
            epochs_per_era,
        )
        .expect("expected to distribute");
        let mut priced = BTreeMap::new();
        for (epoch_index, share) in shares {
            if epoch_index <= remove_epoch {
                continue;
            }
            // A closed epoch's share is taken from the clawback epoch's pool
            *priced.entry(epoch_index.max(clawback_epoch)).or_insert(0) += share;
        }
        assert_eq!(
            priced.values().sum::<SignedCredits>(),
            refund as SignedCredits
        );

        let pools_range =
            remove_epoch + 1..clawback_epoch + perpetual_storage_epochs(epochs_per_era) + 1;

        let storage_fees_to_distribute = chain
            .platform
            .drive
            .get_storage_fees_from_distribution_pool(None, platform_version)
            .expect("expected the storage distribution pool");
        let pools_before = chain.epoch_storage_pools(pools_range.clone());

        chain.run_block(clawback_epoch, &[]);

        let pools_after = chain.epoch_storage_pools(pools_range);

        // The same change also distributes the storage fees collected since the last change
        let mut distributed = SignedCreditsPerEpoch::default();
        distribute_storage_fee_to_epochs_collection(
            &mut distributed,
            storage_fees_to_distribute,
            clawback_epoch,
            epochs_per_era,
        )
        .expect("expected to distribute");

        let clawed_back = pools_after
            .iter()
            .filter_map(|(epoch_index, after)| {
                let before = pools_before.get(epoch_index).copied().unwrap_or(0);
                let clawed_back = before as SignedCredits
                    + distributed.get(epoch_index).copied().unwrap_or(0)
                    - *after as SignedCredits;
                (clawed_back != 0).then_some((*epoch_index, clawed_back))
            })
            .collect();

        Clawback {
            refund,
            clawed_back,
            priced,
        }
    }

    /// Asserts every pool gave back the share the refund returned for it, up to the rounding of
    /// restoring the storage fee from the refund: a later epoch gives back its share or one
    /// credit less, and the clawback epoch gives back the rest. Returns what rounding left to
    /// the clawback epoch.
    fn assert_clawed_back_as_priced(
        clawback: &Clawback,
        clawback_epoch: EpochIndex,
    ) -> SignedCredits {
        let errors = clawback.errors();

        let mut rounding = 0;
        for (epoch_index, error) in &errors {
            if *epoch_index == clawback_epoch {
                continue;
            }
            assert_eq!(
                *error,
                -1,
                "epoch {epoch_index} gave back more than a credit off its share: {}",
                clawback.describe()
            );
            rounding += 1;
        }

        assert_eq!(
            errors.get(&clawback_epoch).copied().unwrap_or(0),
            rounding,
            "the clawback epoch gives back its share and the rounding: {}",
            clawback.describe()
        );
        assert_eq!(
            clawback.clawed_back.values().sum::<SignedCredits>(),
            clawback.refund as SignedCredits
        );

        rounding
    }

    #[tokio::test]
    async fn should_claw_back_each_refunded_share_from_its_epoch() {
        let protocol_version = PlatformVersion::latest().protocol_version;

        for (store_epoch, remove_epoch, remove_in_epoch_change_block, rounding) in [
            (1, 1, false, 1358),
            (1, 3, false, 1356),
            (1, 3, true, 1356),
            (2, 45, false, 1315),
        ] {
            let clawback_epoch = remove_epoch + 1;

            let clawback = store_remove_and_claw_back(
                protocol_version,
                store_epoch,
                remove_epoch,
                remove_in_epoch_change_block,
                clawback_epoch,
            )
            .await;

            assert_eq!(
                assert_clawed_back_as_priced(&clawback, clawback_epoch),
                rounding
            );
            // The epoch the refund is clawed back at gives back its own share too
            assert!(clawback.clawed_back[&clawback_epoch] > clawback.priced[&clawback_epoch]);
        }
    }

    #[tokio::test]
    async fn should_claw_back_the_shares_of_skipped_epochs_from_the_current_epoch() {
        let protocol_version = PlatformVersion::latest().protocol_version;

        // Removed in epoch 3, clawed back after a halt skipped epochs 4 and 5
        let clawback = store_remove_and_claw_back(protocol_version, 1, 3, false, 6).await;

        assert!(!clawback.clawed_back.contains_key(&4) && !clawback.clawed_back.contains_key(&5));
        assert_eq!(assert_clawed_back_as_priced(&clawback, 6), 1354);
        // Epoch 6 gives back the shares of epochs 4, 5 and 6
        assert_eq!(clawback.priced[&6], 3 * 16_267);
    }

    #[tokio::test]
    async fn should_claw_back_a_refund_at_the_end_of_the_storage_window() {
        let protocol_version = PlatformVersion::latest().protocol_version;

        // Stored in epoch 1, the document is paid for until epoch 2000, and removed in epoch
        // 1999 it is refunded the share of epoch 2000 alone
        for clawback_epoch in [2000, 2001] {
            let clawback =
                store_remove_and_claw_back(protocol_version, 1, 1999, false, clawback_epoch).await;

            assert_eq!(assert_clawed_back_as_priced(&clawback, clawback_epoch), 0);
            assert_eq!(
                clawback.clawed_back,
                BTreeMap::from([(clawback_epoch, clawback.refund as SignedCredits)])
            );
        }
    }

    #[tokio::test]
    async fn should_keep_the_clawback_from_the_epoch_after_the_current_one_at_protocol_version_13()
    {
        // Removed in epoch 3 and clawed back at the change into epoch 4, the refund is restored
        // and subtracted from epoch 5: epoch 4 gives back only the leftovers of its share of
        // 16267, and every later epoch more than its share
        let clawback = store_remove_and_claw_back(13, 1, 3, false, 4).await;

        assert_eq!(clawback.clawed_back[&4], 1_023);
        assert_eq!(clawback.priced[&4], 16_267);
        assert_eq!(clawback.clawed_back[&5], 16_286);
        assert_eq!(clawback.moved(), 15_244);
        assert_eq!(
            clawback.clawed_back.values().sum::<SignedCredits>(),
            clawback.refund as SignedCredits
        );

        // After a halt skipped epochs 4 and 5, epoch 6 gives back only leftovers as well
        let clawback = store_remove_and_claw_back(13, 1, 3, false, 6).await;

        assert_eq!(clawback.moved(), 47_840);
        assert_eq!(
            clawback.clawed_back.values().sum::<SignedCredits>(),
            clawback.refund as SignedCredits
        );
    }
}
