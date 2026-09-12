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

/// Storage refunds across a fee-generation boundary, driven through the real
/// block loop (`process_raw_state_transitions` with the state's fee history).
///
/// The doubled-storage test generation (`TEST_PLATFORM_V4`) differs from the
/// latest shipped schedule only in its storage disk usage rate, so every
/// control run below is the same workload under `PlatformVersion::latest()`:
/// storage fees must double, processing fees must not move, and a refund
/// priced at the wrong generation is off by exactly a factor of two.
#[cfg(test)]
mod fee_generation_boundary {
    use crate::execution::validation::state_transition::tests::{
        fetch_expected_identity_balance, process_state_transitions_with_platform_version,
        setup_identity_with_system_credits_with_platform_version,
    };
    use crate::platform_types::platform_state::{PlatformState, PlatformStateV0Methods};
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
    use dpp::data_contract::document_type::DocumentTypeRef;
    use dpp::data_contract::DataContract;
    use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
    use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
    use dpp::fee::epoch::distribution::calculate_storage_fee_refund_amount_and_leftovers;
    use dpp::fee::epoch::{CreditsPerEpoch, DEFAULT_EPOCHS_PER_ERA, GENESIS_EPOCH_INDEX};
    use dpp::fee::fee_result::FeeResult;
    use dpp::fee::Credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::Bytes32;
    use dpp::prelude::IdentityNonce;
    use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::batch_transition::BatchTransition;
    use dpp::util::deserializer::ProtocolVersion;
    use drive::drive::document::query::QueryDocumentsWithFlagsOutcomeV0Methods;
    use drive::drive::Drive;
    use drive::error::drive::DriveError;
    use drive::error::Error as DriveCrateError;
    use drive::fees::op::LowLevelDriveOperation;
    use drive::grovedb_costs::storage_cost::removal::{
        StorageRemovalPerEpochByIdentifier, StorageRemovedBytes,
    };
    use drive::grovedb_costs::storage_cost::StorageCost;
    use drive::grovedb_costs::OperationCost;
    use drive::query::DriveDocumentQuery;
    use drive::util::storage_flags::StorageFlags;
    use drive::util::test_helpers::setup_contract;
    use platform_version::version::mocks::fee_doubled_storage_test::TEST_FEE_VERSION_DOUBLED_STORAGE;
    use platform_version::version::mocks::v4_test::{TEST_PLATFORM_V4, TEST_PROTOCOL_VERSION_4};
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;

    const CONTRACT_PATH: &str =
        "tests/supporting_files/contract/dashpay/dashpay-contract-no-indexes.json";
    const IDENTITY_SEED: u64 = 958;
    const DOCUMENT_SEED: u64 = 433;

    /// Everything a boundary vector needs to compare one run against another.
    struct Workload {
        identity_balance_before: Credits,
        /// The first document of the identity on the contract, which also pays
        /// for joining the contract (the identity contract nonce).
        join: FeeResult,
        /// The document whose deletion is measured.
        insertion: FeeResult,
        deletion: FeeResult,
        /// The deletion's refund to the identity, per storage epoch.
        refunds: CreditsPerEpoch,
        epochs_per_era: u16,
    }

    fn assert_credits_balanced(
        platform: &TempPlatform<MockCoreRPCLike>,
        platform_version: &PlatformVersion,
    ) {
        let credits_verified = platform
            .drive
            .calculate_total_credits_balance(None, &platform_version.drive)
            .expect("expected to check sum trees");

        let balanced = credits_verified
            .ok()
            .expect("expected the credit sums to be checkable");

        assert!(balanced, "platform should be balanced {}", credits_verified);
    }

    /// The storage flags the stored document carries, read back from the tree
    /// so a write that silently landed in the wrong epoch fails on the flags
    /// rather than on the arithmetic.
    fn stored_document_flags(
        platform: &TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        document_type: DocumentTypeRef<'_>,
        document: &Document,
        platform_version: &PlatformVersion,
    ) -> StorageFlags {
        let query = DriveDocumentQuery::new_primary_key_single_item_query(
            contract,
            document_type,
            document.id(),
        );
        let mut documents = platform
            .drive
            .query_documents_with_flags(
                query,
                None,
                false,
                None,
                Some(platform_version.protocol_version),
            )
            .expect("expected to query the stored document")
            .documents_owned();
        let (_, storage_flags) = documents.pop().expect("expected the document to be stored");
        storage_flags.expect("expected the stored document to carry storage flags")
    }

    fn assert_stored_in_epoch(
        platform: &TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        document_type: DocumentTypeRef<'_>,
        document: &Document,
        epoch_index: EpochIndex,
        owner: &Identity,
        platform_version: &PlatformVersion,
    ) {
        let storage_flags = stored_document_flags(
            platform,
            contract,
            document_type,
            document,
            platform_version,
        );
        assert_eq!(
            *storage_flags.base_epoch(),
            epoch_index,
            "the document must be stored in epoch {epoch_index}, got {storage_flags:?}"
        );
        assert_eq!(
            storage_flags.owner_id(),
            Some(&owner.id().to_buffer()),
            "the document's bytes must be owned by the identity that pays for them"
        );
    }

    #[allow(clippy::too_many_arguments)]
    async fn store_document(
        platform: &TempPlatform<MockCoreRPCLike>,
        document_type: DocumentTypeRef<'_>,
        rng: &mut StdRng,
        identity: &Identity,
        key: &IdentityPublicKey,
        signer: &SimpleSigner,
        identity_contract_nonce: IdentityNonce,
        block_info: BlockInfo,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> (Document, FeeResult) {
        let entropy = Bytes32::random_with_rng(rng);

        let mut document = document_type
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

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                document_type,
                entropy.0,
                key,
                identity_contract_nonce,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let (mut fee_results, _) = process_state_transitions_with_platform_version(
            platform,
            &[documents_batch_create_transition],
            block_info,
            platform_state,
            platform_version,
        );

        assert_credits_balanced(platform, platform_version);

        (document, fee_results.remove(0))
    }

    #[allow(clippy::too_many_arguments)]
    async fn delete_document(
        platform: &TempPlatform<MockCoreRPCLike>,
        document: Document,
        document_type: DocumentTypeRef<'_>,
        key: &IdentityPublicKey,
        signer: &SimpleSigner,
        identity_contract_nonce: IdentityNonce,
        block_info: BlockInfo,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> FeeResult {
        let documents_batch_delete_transition =
            BatchTransition::new_document_deletion_transition_from_document(
                document,
                document_type,
                key,
                identity_contract_nonce,
                0,
                None,
                signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let (mut fee_results, _) = process_state_transitions_with_platform_version(
            platform,
            &[documents_batch_delete_transition],
            block_info,
            platform_state,
            platform_version,
        );

        assert_credits_balanced(platform, platform_version);

        fee_results.remove(0)
    }

    /// The block state the hook would leave behind after the doubled-storage
    /// generation activated at `activation_epoch`: the mock protocol version
    /// is current, and the fee history carries the activation entry.
    fn state_after_upgrade_to_doubled_storage(
        platform: &TempPlatform<MockCoreRPCLike>,
        activation_epoch: EpochIndex,
    ) -> PlatformState {
        let mut platform_state = platform.state.load().as_ref().clone();
        platform_state.set_current_protocol_version_in_consensus(TEST_PROTOCOL_VERSION_4);
        platform_state.set_next_epoch_protocol_version(TEST_PROTOCOL_VERSION_4);
        platform_state
            .previous_fee_versions_mut()
            .insert(activation_epoch, &TEST_FEE_VERSION_DOUBLED_STORAGE);
        platform_state
    }

    /// Recovers the number of bytes a refund paid for.
    ///
    /// GroveDB derives the removed byte count from the element it deletes and
    /// the fee result only carries the priced refund, so the count is taken
    /// from a refund known to be priced at `rate`: the refund grows by almost
    /// a full rate per byte while the rounding drift of the era distribution
    /// is bounded by the number of epochs, so exactly one count reproduces it.
    fn removed_bytes_priced_at(
        refund: Credits,
        rate: Credits,
        storage_epoch: EpochIndex,
        current_epoch: EpochIndex,
        epochs_per_era: u16,
        max_bytes: u64,
    ) -> u64 {
        (1..=max_bytes)
            .find(|bytes| {
                expected_refund(bytes * rate, storage_epoch, current_epoch, epochs_per_era)
                    == refund
            })
            .unwrap_or_else(|| {
                panic!("no byte count up to {max_bytes} priced at {rate} refunds {refund} credits")
            })
    }

    fn expected_refund(
        storage_fee: Credits,
        storage_epoch: EpochIndex,
        current_epoch: EpochIndex,
        epochs_per_era: u16,
    ) -> Credits {
        calculate_storage_fee_refund_amount_and_leftovers(
            storage_fee,
            storage_epoch,
            current_epoch,
            epochs_per_era,
        )
        .expect("expected to compute the refund")
        .0
    }

    fn refunds_for(fee_result: &FeeResult, identity: &Identity) -> CreditsPerEpoch {
        fee_result
            .fee_refunds
            .get(&identity.id().to_buffer())
            .expect("expected refunds for the identity")
            .clone()
    }

    fn refund_amount(refunds: &CreditsPerEpoch, storage_epoch: EpochIndex) -> Credits {
        assert_eq!(
            refunds.len(),
            1,
            "the refund must be sectioned into the single storage epoch {storage_epoch}, got {refunds:?}"
        );
        *refunds
            .get(&storage_epoch)
            .expect("expected the refund to be keyed by the storage epoch")
    }

    /// One run on either side of a boundary compared to the same run at the
    /// latest schedule: storage doubles, processing does not move, and the
    /// refund is exactly the refund of the same bytes at twice the rate.
    fn assert_doubled_storage_generation(
        doubled: &Workload,
        control: &Workload,
        storage_epoch: EpochIndex,
        current_epoch: EpochIndex,
    ) {
        let control_rate = PlatformVersion::latest()
            .fee_version
            .storage
            .storage_disk_usage_credit_per_byte;
        let doubled_rate = TEST_FEE_VERSION_DOUBLED_STORAGE
            .storage
            .storage_disk_usage_credit_per_byte;
        assert_eq!(doubled_rate, 2 * control_rate);

        for (doubled_fee, control_fee) in [
            (&doubled.join, &control.join),
            (&doubled.insertion, &control.insertion),
        ] {
            assert_eq!(
                doubled_fee.storage_fee,
                2 * control_fee.storage_fee,
                "bytes stored under the doubled generation cost twice the latest rate"
            );
            assert_eq!(
                doubled_fee.processing_fee, control_fee.processing_fee,
                "only the storage rate differs between the generations"
            );
        }
        assert_eq!(doubled.deletion.storage_fee, 0);
        assert_eq!(control.deletion.storage_fee, 0);
        assert_eq!(doubled.epochs_per_era, control.epochs_per_era);

        let control_refund = refund_amount(&control.refunds, storage_epoch);
        let doubled_refund = refund_amount(&doubled.refunds, storage_epoch);

        let removed_bytes = removed_bytes_priced_at(
            control_refund,
            control_rate,
            storage_epoch,
            current_epoch,
            control.epochs_per_era,
            control.insertion.storage_fee / control_rate,
        );
        assert_eq!(
            doubled_refund,
            expected_refund(
                removed_bytes * doubled_rate,
                storage_epoch,
                current_epoch,
                doubled.epochs_per_era
            ),
            "the refund of {removed_bytes} bytes stored in epoch {storage_epoch} and removed in \
             epoch {current_epoch} must be priced at the doubled generation"
        );
    }

    /// Bytes stored in epoch 2 and removed in epoch 3, optionally after the
    /// doubled-storage generation activated in epoch 2.
    async fn store_after_boundary_delete_next_epoch(activation: Option<EpochIndex>) -> Workload {
        const STORAGE_EPOCH: EpochIndex = 2;
        const REMOVAL_EPOCH: EpochIndex = 3;

        let latest = PlatformVersion::latest();
        let processing_version = if activation.is_some() {
            PlatformVersion::get(TEST_PROTOCOL_VERSION_4).expect("expected the mock version")
        } else {
            latest
        };

        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let contract = setup_contract(
            &platform.drive,
            CONTRACT_PATH,
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            None,
        );
        let profile = contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let mut rng = StdRng::seed_from_u64(DOCUMENT_SEED);

        let (identity, signer, key) = setup_identity_with_system_credits_with_platform_version(
            &mut platform,
            IDENTITY_SEED,
            dash_to_credits!(1),
            latest,
        );
        let identity_balance_before = dash_to_credits!(1);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, STORAGE_EPOCH, false);
        let block_state = |platform: &TempPlatform<MockCoreRPCLike>| match activation {
            Some(activation_epoch) => {
                state_after_upgrade_to_doubled_storage(platform, activation_epoch)
            }
            None => platform.state.load().as_ref().clone(),
        };
        let platform_state = block_state(&platform);
        let block_info = *platform_state.last_block_info();
        assert_eq!(block_info.epoch.index, STORAGE_EPOCH);

        let (join_document, join) = store_document(
            &platform,
            profile,
            &mut rng,
            &identity,
            &key,
            &signer,
            1,
            block_info,
            &platform_state,
            processing_version,
        )
        .await;
        let (document, insertion) = store_document(
            &platform,
            profile,
            &mut rng,
            &identity,
            &key,
            &signer,
            2,
            block_info,
            &platform_state,
            processing_version,
        )
        .await;
        for stored in [&join_document, &document] {
            assert_stored_in_epoch(
                &platform,
                &contract,
                profile,
                stored,
                STORAGE_EPOCH,
                &identity,
                processing_version,
            );
        }

        fast_forward_to_block(&platform, 1_800_000_000, 1_300, 42, REMOVAL_EPOCH, false);
        let platform_state = block_state(&platform);
        let block_info = *platform_state.last_block_info();
        assert_eq!(block_info.epoch.index, REMOVAL_EPOCH);

        let deletion = delete_document(
            &platform,
            document,
            profile,
            &key,
            &signer,
            3,
            block_info,
            &platform_state,
            processing_version,
        )
        .await;
        let refunds = refunds_for(&deletion, &identity);

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            processing_version,
            identity_balance_before
                - join.total_base_fee()
                - insertion.total_base_fee()
                - deletion.total_base_fee()
                + refunds.values().sum::<Credits>(),
        );

        Workload {
            identity_balance_before,
            join,
            insertion,
            deletion,
            refunds,
            epochs_per_era: platform.config.drive.epochs_per_era,
        }
    }

    /// Bytes stored in the genesis epoch of a chain started at
    /// `protocol_version` and removed in epoch 1, with the fee history the
    /// initial state records.
    async fn store_at_genesis_delete_in_epoch_one(protocol_version: ProtocolVersion) -> Workload {
        const REMOVAL_EPOCH: EpochIndex = 1;

        let platform_version =
            PlatformVersion::get(protocol_version).expect("expected the protocol version");

        let mut platform = TestPlatformBuilder::new()
            .with_initial_protocol_version(protocol_version)
            .build_with_mock_rpc()
            .set_genesis_state();

        let contract = setup_contract(
            &platform.drive,
            CONTRACT_PATH,
            None,
            None,
            None::<fn(&mut DataContract)>,
            None,
            Some(platform_version),
        );
        let profile = contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

        let mut rng = StdRng::seed_from_u64(DOCUMENT_SEED);

        let (identity, signer, key) = setup_identity_with_system_credits_with_platform_version(
            &mut platform,
            IDENTITY_SEED,
            dash_to_credits!(1),
            platform_version,
        );
        let identity_balance_before = dash_to_credits!(1);

        let platform_state = platform.state.load().as_ref().clone();
        assert_eq!(
            platform_state.previous_fee_versions(),
            &CachedEpochIndexFeeVersions::from([(
                GENESIS_EPOCH_INDEX,
                platform_version.fee_version.as_static()
            )]),
            "the initial state records the genesis fee generation"
        );
        let block_info = BlockInfo::default();
        assert_eq!(block_info.epoch.index, GENESIS_EPOCH_INDEX);

        let (join_document, join) = store_document(
            &platform,
            profile,
            &mut rng,
            &identity,
            &key,
            &signer,
            1,
            block_info,
            &platform_state,
            platform_version,
        )
        .await;
        let (document, insertion) = store_document(
            &platform,
            profile,
            &mut rng,
            &identity,
            &key,
            &signer,
            2,
            block_info,
            &platform_state,
            platform_version,
        )
        .await;
        for stored in [&join_document, &document] {
            assert_stored_in_epoch(
                &platform,
                &contract,
                profile,
                stored,
                GENESIS_EPOCH_INDEX,
                &identity,
                platform_version,
            );
        }

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, REMOVAL_EPOCH, false);
        let platform_state = platform.state.load().as_ref().clone();
        let block_info = *platform_state.last_block_info();
        assert_eq!(block_info.epoch.index, REMOVAL_EPOCH);

        let deletion = delete_document(
            &platform,
            document,
            profile,
            &key,
            &signer,
            3,
            block_info,
            &platform_state,
            platform_version,
        )
        .await;
        let refunds = refunds_for(&deletion, &identity);

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            identity_balance_before
                - join.total_base_fee()
                - insertion.total_base_fee()
                - deletion.total_base_fee()
                + refunds.values().sum::<Credits>(),
        );

        Workload {
            identity_balance_before,
            join,
            insertion,
            deletion,
            refunds,
            epochs_per_era: platform.config.drive.epochs_per_era,
        }
    }

    #[tokio::test]
    async fn should_refund_bytes_stored_after_a_fee_generation_boundary_at_the_new_rate() {
        let control = store_after_boundary_delete_next_epoch(None).await;
        let doubled = store_after_boundary_delete_next_epoch(Some(2)).await;

        assert_eq!(
            doubled.identity_balance_before,
            control.identity_balance_before
        );
        assert_doubled_storage_generation(&doubled, &control, 2, 3);
    }

    #[tokio::test]
    async fn should_refund_genesis_epoch_bytes_at_the_genesis_fee_generation_rate() {
        // Without the genesis entry in the fee history the lookup would fall
        // back to the first registered generation and refund at half the rate
        // the bytes were paid for.
        let control =
            store_at_genesis_delete_in_epoch_one(PlatformVersion::latest().protocol_version).await;
        let doubled = store_at_genesis_delete_in_epoch_one(TEST_PROTOCOL_VERSION_4).await;

        assert_eq!(
            doubled.identity_balance_before,
            control.identity_balance_before
        );
        assert_doubled_storage_generation(&doubled, &control, GENESIS_EPOCH_INDEX, 1);
    }

    #[test]
    fn should_reject_a_non_first_fee_generation_removal_without_fee_history() {
        const REMOVED_BYTES: u32 = 1_000;
        let owner = [7u8; 32];
        let epoch = Epoch::new(3).expect("expected an epoch");

        let removal = || {
            let mut removal_per_epoch_by_identifier = StorageRemovalPerEpochByIdentifier::new();
            removal_per_epoch_by_identifier
                .entry(owner)
                .or_default()
                .insert(GENESIS_EPOCH_INDEX, REMOVED_BYTES);
            LowLevelDriveOperation::CalculatedCostOperation(OperationCost {
                storage_cost: StorageCost {
                    added_bytes: 0,
                    replaced_bytes: 0,
                    removed_bytes: StorageRemovedBytes::SectionedStorageRemoval(
                        removal_per_epoch_by_identifier,
                    ),
                },
                ..Default::default()
            })
        };

        // A generation other than the first cannot price a sectioned removal
        // without the fee history: the lifecycle callers that pass `None` may
        // only remove unflagged bytes.
        let without_history = Drive::calculate_fee(
            None,
            Some(vec![removal()]),
            &epoch,
            DEFAULT_EPOCHS_PER_ERA,
            &TEST_PLATFORM_V4,
            None,
        );
        assert!(
            matches!(
                without_history,
                Err(DriveCrateError::Drive(DriveError::CorruptedCodeExecution(
                    _
                )))
            ),
            "expected a corrupted code execution error, got {without_history:?}"
        );

        // With the history the same removal is refunded at the generation
        // active for the storage epoch's lookup.
        let history = CachedEpochIndexFeeVersions::from([(
            GENESIS_EPOCH_INDEX,
            &TEST_FEE_VERSION_DOUBLED_STORAGE,
        )]);
        let fee_result = Drive::calculate_fee(
            None,
            Some(vec![removal()]),
            &epoch,
            DEFAULT_EPOCHS_PER_ERA,
            &TEST_PLATFORM_V4,
            Some(&history),
        )
        .expect("expected the removal to be priced with the fee history");
        assert_eq!(
            fee_result
                .fee_refunds
                .get(&owner)
                .and_then(|refunds| refunds.get(&GENESIS_EPOCH_INDEX))
                .copied(),
            Some(expected_refund(
                Credits::from(REMOVED_BYTES)
                    * TEST_FEE_VERSION_DOUBLED_STORAGE
                        .storage
                        .storage_disk_usage_credit_per_byte,
                GENESIS_EPOCH_INDEX,
                epoch.index,
                DEFAULT_EPOCHS_PER_ERA,
            ))
        );
    }
}
