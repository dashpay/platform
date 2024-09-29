#[cfg(test)]
mod refund_tests {
    use crate::execution::validation::state_transition::tests::{
        fetch_expected_identity_balance, process_state_transitions,
        setup_identity_with_system_credits,
    };
    use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
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
    use dpp::document::document_methods::DocumentMethodsV0;
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::{Document, DocumentV0Setters};
    use dpp::fee::fee_result::FeeResult;
    use dpp::fee::Credits;
    use dpp::identity::accessors::IdentityGettersV0;
    use dpp::identity::{Identity, IdentityPublicKey};
    use dpp::platform_value::Bytes32;
    use dpp::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
    use dpp::state_transition::documents_batch_transition::DocumentsBatchTransition;
    use drive::util::test_helpers::setup_contract;
    use platform_version::version::PlatformVersion;
    use rand::prelude::StdRng;
    use rand::SeedableRng;
    use simple_signer::signer::SimpleSigner;
    use std::ops::Deref;

    // There's a fee for the first document that a user creates on a contract as they add space
    // For the identity data contract nonce
    fn setup_join_contract_document<'a>(
        platform: &TempPlatform<MockCoreRPCLike>,
        profile: DocumentTypeRef,
        rng: &mut StdRng,
        identity: &'a Identity,
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

        let (mut fee_results, processed_block_fee_outcome) = process_state_transitions(
            &platform,
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
            &platform,
            identity.id(),
            platform_version,
            expected_user_balance_after_creation,
        );

        expected_user_balance_after_creation
    }

    fn setup_initial_document<'a>(
        platform: &TempPlatform<MockCoreRPCLike>,
        profile: DocumentTypeRef,
        rng: &mut StdRng,
        identity: &'a Identity,
        key: &IdentityPublicKey,
        signer: &SimpleSigner,
    ) -> (Document, FeeResult, Credits) {
        // Let's make another document first just so the operations of joining a contract are out of the way
        // (A user pays to add some data to the state on the first time they make their first document for a contract)
        let user_credits_left =
            setup_join_contract_document(platform, profile, rng, identity, key, signer);

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
            .serialize(profile, platform_version)
            .expect("expected to serialize")
            .len() as u64;

        assert_eq!(serialized_len, 173);

        let documents_batch_create_transition =
            DocumentsBatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                profile,
                entropy.0,
                key,
                3,
                0,
                signer,
                platform_version,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

        let (mut fee_results, _) = process_state_transitions(
            &platform,
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
            &platform,
            identity.id(),
            platform_version,
            expected_user_balance_after_creation,
        );

        (document, fee_result, expected_user_balance_after_creation)
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

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            dash_to_credits!(1),
        );

        let (document, insertion_fee_result, current_user_balance) =
            setup_initial_document(&platform, profile, &mut rng, &identity, &key, &signer);

        let documents_batch_delete_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
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
    #[test]
    fn test_document_refund_after_an_epoch() {
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

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            dash_to_credits!(1),
        );

        let (document, insertion_fee_result, current_user_balance) =
            setup_initial_document(&platform, profile, &mut rng, &identity, &key, &signer);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let documents_batch_delete_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
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

    #[test]
    fn test_document_refund_after_a_year() {
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

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            dash_to_credits!(1),
        );

        let (document, insertion_fee_result, current_user_balance) =
            setup_initial_document(&platform, profile, &mut rng, &identity, &key, &signer);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 40, false); //a year later

        let documents_batch_delete_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
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

    #[test]
    fn test_document_refund_after_25_years() {
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

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            dash_to_credits!(1),
        );

        let (document, insertion_fee_result, current_user_balance) =
            setup_initial_document(&platform, profile, &mut rng, &identity, &key, &signer);

        fast_forward_to_block(&platform, 10_200_000_000, 9000, 42, 40 * 25, false); //25 years later

        let documents_batch_delete_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
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

    #[test]
    fn test_document_refund_after_50_years() {
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

        let (identity, signer, key) =
            setup_identity_with_system_credits(&mut platform, 958, dash_to_credits!(1));

        fetch_expected_identity_balance(
            &platform,
            identity.id(),
            platform_version,
            dash_to_credits!(1),
        );

        let (document, _, current_user_balance) =
            setup_initial_document(&platform, profile, &mut rng, &identity, &key, &signer);

        fast_forward_to_block(&platform, 10_200_000_000, 9000, 42, 40 * 50, false); //50 years later

        let documents_batch_delete_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                &signer,
                platform_version,
                None,
                None,
                None,
            )
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

    #[test]
    fn test_document_refund_after_10_epochs_on_different_fee_version_increasing_fees() {
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

        let (document, insertion_fee_result, current_user_balance) =
            setup_initial_document(&platform, profile, &mut rng, &identity, &key, &signer);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 10, false); //next epoch

        let documents_batch_delete_transition =
            DocumentsBatchTransition::new_document_deletion_transition_from_document(
                document,
                profile,
                &key,
                4,
                0,
                &signer,
                &platform_version_with_higher_fees,
                None,
                None,
                None,
            )
            .expect("expect to create documents batch transition");

        let mut platform_state = platform.state.load().clone().deref().clone();

        platform_state
            .previous_fee_versions_mut()
            .insert(5, platform_version_with_higher_fees.fee_version.clone());

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

        // println!("{}", insertion_fee_result.storage_fee);
        // println!("{}", refund_amount);

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
