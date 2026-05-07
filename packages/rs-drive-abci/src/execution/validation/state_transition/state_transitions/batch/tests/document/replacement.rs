use super::*;

mod replacement_tests {
    use super::*;
    use crate::test::helpers::fast_forward_to_block::fast_forward_to_block;
    use dpp::identifier::Identifier;
    use dpp::prelude::IdentityNonce;
    use dpp::tokens::token_payment_info::v0::TokenPaymentInfoV0;
    use dpp::tokens::token_payment_info::TokenPaymentInfo;
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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
            BatchTransition::new_document_creation_transition_from_document(
                document,
                profile,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
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
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let documents_batch_update_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_update_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 1399260);

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    async fn perform_document_replace_on_profile_after_epoch_change(
        original_name: &str,
        new_names: Vec<(&str, StorageFlags)>,
    ) {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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

        document.set("displayName", original_name.into());
        document.set("avatarUrl", "http://test.com/bob.jpg".into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                profile,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
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
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        for (i, (new_name, mut expected_flags)) in new_names.into_iter().enumerate() {
            document.increment_revision().unwrap();
            document.set("displayName", new_name.into());

            fast_forward_to_block(
                &platform,
                500_000_000 + i as u64 * 1000,
                900 + i as u64,
                42,
                1 + i as u16,
                true,
            ); //less than a week

            let documents_batch_update_transition =
                BatchTransition::new_document_replacement_transition_from_document(
                    document.clone(),
                    profile,
                    &key,
                    3 + i as IdentityNonce,
                    0,
                    None,
                    &signer,
                    platform_version,
                    None,
                )
                .await
                .expect("expect to create documents batch transition");

            let documents_batch_update_serialized_transition = documents_batch_update_transition
                .serialize_to_bytes()
                .expect("expected documents batch serialized state transition");

            let transaction = platform.drive.grove.start_transaction();

            let platform_state = platform.state.load();

            let processing_result = platform
                .platform
                .process_raw_state_transitions(
                    &vec![documents_batch_update_serialized_transition.clone()],
                    &platform_state,
                    platform_state.last_block_info(),
                    &transaction,
                    platform_version,
                    false,
                    None,
                )
                .expect("expected to process state transition");

            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit transaction");

            assert_eq!(processing_result.invalid_paid_count(), 0);

            assert_eq!(processing_result.invalid_unpaid_count(), 0);

            assert_eq!(
                processing_result.valid_count(),
                1,
                "{:?}",
                processing_result.execution_results()
            );

            let drive_query = DriveDocumentQuery::new_primary_key_single_item_query(
                &dashpay,
                profile,
                document.id(),
            );

            let mut documents = platform
                .drive
                .query_documents_with_flags(drive_query, None, false, None, None)
                .expect("expected to get back documents")
                .documents_owned();

            let (_first_document, storage_flags) = documents.remove(0);

            let storage_flags = storage_flags.expect("expected storage flags");

            expected_flags.set_owner_id(identity.id().to_buffer());

            assert_eq!(storage_flags, expected_flags);
        }

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size() {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![(
                "Samuel",
                StorageFlags::MultiEpochOwned(
                    0,
                    BTreeMap::from([(1, 6)]),
                    Identifier::default().to_buffer(),
                ),
            )],
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_smaller_size() {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![(
                "S",
                StorageFlags::SingleEpochOwned(0, Identifier::default().to_buffer()),
            )],
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_same_size() {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![(
                "Max",
                StorageFlags::SingleEpochOwned(0, Identifier::default().to_buffer()),
            )],
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_bigger_size(
    ) {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![
                (
                    "Samuel",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 6)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
                (
                    "SamuelW",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 6), (2, 4)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_bigger_size_by_3_bytes(
    ) {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![
                (
                    "Samuel",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 6)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
                (
                    "SamuelWes",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 6), (2, 6)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_smaller_size(
    ) {
        // In this case we start with the size Samuell Base epoch 0 epoch 1 added 7 bytes
        // Then we try to update it to         Sami    Base epoch 2
        // Epoch 1 added 7 bytes is itself 3 bytes
        // Sami is 3 bytes less than Samuell
        // First iteration will say we should remove 6 bytes
        // We need to start by calculating the cost of the original storage flags, in this case 5 bytes
        // Then we need to calculate the cost of the new storage flags, in this case 2 bytes
        // We should do the difference, then apply that difference in the combination function
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![
                (
                    "Samuell",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 7)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
                (
                    "Sami",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 4)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_mutable_different_epoch_bigger_size_then_back_to_original(
    ) {
        perform_document_replace_on_profile_after_epoch_change(
            "Sam",
            vec![
                (
                    "Samuel",
                    StorageFlags::MultiEpochOwned(
                        0,
                        BTreeMap::from([(1, 6)]),
                        Identifier::default().to_buffer(),
                    ),
                ),
                (
                    "Sam",
                    StorageFlags::SingleEpochOwned(0, Identifier::default().to_buffer()),
                ),
            ],
        )
        .await;
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_not_mutable() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(437);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (other_identity, ..) = setup_identity(&mut platform, 495, dash_to_credits!(0.1));

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        let contact_request_document_type = dashpay_contract
            .document_type_for_name("contactRequest")
            .expect("expected a profile document type");

        assert!(!contact_request_document_type.documents_mutable());

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = contact_request_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set(
            "toUserId",
            Value::Identifier(other_identity.id().to_buffer()),
        );
        document.set("recipientKeyIndex", Value::U32(1));
        document.set("senderKeyIndex", Value::U32(1));
        document.set("accountReference", Value::U32(0));

        let mut altered_document = document.clone();

        altered_document.set_revision(Some(1));
        altered_document.set("senderKeyIndex", Value::U32(2));

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                contact_request_document_type,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
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
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let documents_batch_update_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                contact_request_document_type,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_update_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        // Fee bumped from the previous 41880 (bare bump-only) to 445700 in
        // PROTOCOL_VERSION_12. Issue #2867 fix: the failure path now emits the
        // bump after running the document fetch + validation work, so the fee
        // covers that work — same drive ops, just charged correctly.
        assert_eq!(processing_result.aggregated_fees().processing_fee, 445700);
    }

    /// Regression test for issue #2867 (mainnet duplicate-tx escalation, 2026-05-04).
    ///
    /// Mainnet ST hash 35C0...313C — a Documents Batch Replace by Techawanka.dash
    /// (6Pp1RFqRnpStnY8vmp5k3ypE6rFBvzPwcoguwDXbRA7F) — landed at block 361774 idx 1
    /// as FAIL with gasUsed 41880, then re-appeared in a later block, panicking the
    /// explorer indexer with `duplicate key value violates unique constraint
    /// state_transition_hash`. The shape: nonce 4 ran successfully *before* nonce
    /// 3 in the same block (out-of-order SDK retries), so when nonce 3's Replace
    /// reached deliver_tx the doc revision had already advanced. The
    /// revision-bump-by-one check in
    /// batch/transformer/v0/mod.rs::check_revision_is_bumped_by_one_during_replace_v0
    /// fails, and the surrounding handler (lines 712–715) returns `Ok(result)`
    /// with errors-only and **no BumpIdentityDataContractNonce action** — unlike
    /// the find_replaced_document_v0 path on the same enum arm (lines 672–686)
    /// which DOES emit a bump.
    ///
    /// Consequence: the user pays the 41880 bump fee (because the empty
    /// BatchTransitionAction still triggers PaidConsensusError accounting), but
    /// the contract nonce IS NEVER ADVANCED in state. The exact same bytes can
    /// then be re-broadcast indefinitely; each retry lands a new failed copy in
    /// a new block, all sharing the same hash.
    ///
    /// What this test pins:
    ///   1. After a Replace that fails the revision-bump-by-one check commits,
    ///      the stored identity_contract_nonce MUST advance past the submitted
    ///      nonce. (Direct invariant — fails fast on RED.)
    ///   2. Re-submitting the same exact bytes through CheckTx FirstTimeCheck
    ///      MUST be rejected with InvalidIdentityNonceError. (Symptom-level —
    ///      this is what lklimek's Feb 10 2026 testnet debug log proved was
    ///      broken.)
    ///
    /// Both assertions fail on v3.1-dev today; both should pass once the bump
    /// is emitted on revision/ownership-mismatch paths in batch/transformer/v0.
    #[tokio::test]
    async fn replayed_failed_replace_with_consumed_nonce_must_be_rejected_at_check_tx() {
        use crate::execution::check_tx::CheckTxLevel;
        use crate::execution::validation::state_transition::check_tx_verification::state_transition_to_execution_event_for_check_tx;
        use crate::platform_types::platform::PlatformRef;
        use dpp::serialization::PlatformDeserializable;
        use dpp::state_transition::StateTransition;

        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(437);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.5));

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        // Use the mutable `profile` doc type — same contract-and-doc-type that
        // mainnet 35C0 was operating on (DPNS-like profile-replace flow).
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
        // Random fillers can produce a non-URI avatarUrl that fails JSON-schema
        // validation on Create. Pin it to a valid URI like the sibling tests do.
        document.set("avatarUrl", "http://test.com/bob.jpg".into());
        document.set("displayName", "Original".into());

        // 1) Create at nonce 2 — consumes nonce 2; doc lands at revision 1.
        let create_transition = BatchTransition::new_document_creation_transition_from_document(
            document.clone(),
            profile,
            entropy.0,
            &key,
            2,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build create transition");

        let create_serialized = create_transition
            .serialize_to_bytes()
            .expect("expected to serialize create");

        let transaction = platform.drive.grove.start_transaction();
        let create_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![create_serialized],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process create");
        assert_eq!(create_result.valid_count(), 1);
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit create");

        let (post_create_nonce_raw, _) = platform
            .drive
            .fetch_identity_contract_nonce_with_fees(
                identity.id().to_buffer(),
                dashpay_contract.id().to_buffer(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to fetch contract nonce after create");
        let post_create_nonce =
            post_create_nonce_raw.expect("contract nonce must be present after create");

        // 2) Build a Replace at nonce 3 with a revision the chain will reject.
        //    Doc at revision 1 → expected revision 2 on replace. We submit
        //    revision 3 → check_revision_is_bumped_by_one_during_replace_v0
        //    returns InvalidDocumentRevisionError(Some(1), 3). On mainnet this
        //    happened naturally because nonce 4 ran first and advanced the doc
        //    revision past what nonce 3's tx expected.
        let mut altered_document = document.clone();
        altered_document.set_revision(Some(3));
        altered_document.set("displayName", "Out of order".into());

        let replace_transition = BatchTransition::new_document_replacement_transition_from_document(
            altered_document,
            profile,
            &key,
            3,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected to build replace transition");

        let replace_serialized = replace_transition
            .serialize_to_bytes()
            .expect("expected to serialize replace");

        let transaction = platform.drive.grove.start_transaction();
        let replace_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![replace_serialized.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process replace");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit failed replace");

        assert_eq!(
            replace_result.invalid_paid_count(),
            1,
            "Replace must commit as invalid_paid (PaidConsensusError); execution_results={:?}",
            replace_result.execution_results()
        );
        assert_eq!(replace_result.valid_count(), 0);

        // 3) Direct invariant: the bump must have advanced the contract nonce
        //    in state. If the stored nonce is still post-create, the bump
        //    silently dropped — that is the bug.
        let (post_replace_nonce_raw, _) = platform
            .drive
            .fetch_identity_contract_nonce_with_fees(
                identity.id().to_buffer(),
                dashpay_contract.id().to_buffer(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("expected to fetch contract nonce after failed replace");
        let post_replace_nonce =
            post_replace_nonce_raw.expect("contract nonce must be present after failed replace");

        assert_ne!(
            post_replace_nonce, post_create_nonce,
            "BUG: failed Replace's bump action did not advance the contract \
             nonce. Stored nonce is still {:#x} (= post-create value), so the \
             same exact serialized bytes can be replayed forever. \
             Root cause: batch/transformer/v0/mod.rs:712-715 (and 697-700) \
             return Ok(result) with errors-only and no BumpIdentityDataContractNonce \
             action when check_revision_is_bumped_by_one_during_replace_v0 (or \
             check_ownership_of_old_replaced_document_v0) fails — unlike the \
             find_replaced_document_v0 failure path on the same arm (lines \
             672-686) which does emit the bump.",
            post_create_nonce
        );

        // 4) Symptom-level: re-submitting identical bytes through CheckTx
        //    FirstTimeCheck must hit the nonce check first and reject. This is
        //    exactly what lklimek's Feb 10 2026 testnet check_tx debug log
        //    showed NOT happening.
        let replayed_state_transition =
            StateTransition::deserialize_from_bytes(&replace_serialized)
                .expect("expected to deserialize replayed transition");

        let platform_state = platform.state.load();
        let platform_ref = PlatformRef {
            drive: &platform.drive,
            state: &platform_state,
            config: &platform.config,
            core_rpc: &platform.core_rpc,
        };

        let check_tx_result = state_transition_to_execution_event_for_check_tx(
            &platform_ref,
            replayed_state_transition,
            CheckTxLevel::FirstTimeCheck,
            platform_version,
        )
        .expect("expected check_tx to not return an Err");

        assert!(
            !check_tx_result.is_valid(),
            "CheckTx FirstTimeCheck MUST reject identical bytes after the \
             failed-Replace bump consumed the nonce — it accepted them on \
             testnet on 2026-02-10."
        );
        assert!(
            check_tx_result.errors.iter().any(|e| matches!(
                e,
                ConsensusError::StateError(StateError::InvalidIdentityNonceError(_))
            )),
            "expected InvalidIdentityNonceError on replay; got {:?}",
            check_tx_result.errors
        );
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_is_not_mutable_but_is_transferable() {
        let platform_version = PlatformVersion::latest();
        let (mut platform, contract) = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure()
            .with_crypto_card_game_transfer_only(Transferable::Always);

        let mut rng = StdRng::seed_from_u64(435);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (receiver, _, _) = setup_identity(&mut platform, 452, dash_to_credits!(0.1));

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
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
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let sender_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", identity.id());

        let query_sender_identity_documents = DriveDocumentQuery::from_sql_expr(
            sender_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let receiver_documents_sql_string =
            format!("select * from card where $ownerId == '{}'", receiver.id());

        let query_receiver_identity_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &contract,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(
                query_sender_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(
                query_receiver_identity_documents.clone(),
                None,
                false,
                None,
                None,
            )
            .expect("expected query result");

        // We expect the sender to have 1 document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);

        document.set_revision(Some(2));

        document.set("attack", 6.into());
        document.set("defense", 0.into());

        let documents_batch_transfer_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                document,
                card_document_type,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition for transfer");

        let documents_batch_transfer_serialized_transition = documents_batch_transfer_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_transfer_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 445700);

        let query_sender_results = platform
            .drive
            .query_documents(query_sender_identity_documents, None, false, None, None)
            .expect("expected query result");

        let query_receiver_results = platform
            .drive
            .query_documents(query_receiver_identity_documents, None, false, None, None)
            .expect("expected query result");

        // We expect the sender to still have their document, and the receiver to have none
        assert_eq!(query_sender_results.documents().len(), 1);

        assert_eq!(query_receiver_results.documents().len(), 0);
    }

    #[tokio::test]
    async fn test_document_replace_that_does_not_yet_exist() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let dashpay = platform.drive.cache.system_data_contracts.load_dashpay();
        let dashpay_contract = dashpay.clone();

        let profile = dashpay_contract
            .document_type_for_name("profile")
            .expect("expected a profile document type");

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

        let documents_batch_update_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_update_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 1);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 0);

        assert_eq!(processing_result.aggregated_fees().processing_fee, 516040);
    }

    #[tokio::test]
    async fn test_double_document_replace() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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

        let mut altered_document_2 = altered_document.clone();

        altered_document_2.increment_revision().unwrap();
        altered_document_2.set("displayName", "Ody".into());
        altered_document_2.set("avatarUrl", "http://test.com/drapes.jpg".into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                profile,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
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
                platform_state.last_block_info(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let receiver_documents_sql_string = "select * from profile".to_string();

        let query_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &dashpay,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

        let documents_batch_update_transition_1 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_1 = documents_batch_update_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_update_transition_2 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document_2,
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

        let documents_batch_update_serialized_transition_2 = documents_batch_update_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![
                    documents_batch_update_serialized_transition_1.clone(),
                    documents_batch_update_serialized_transition_2.clone(),
                ],
                &platform_state,
                platform_state.last_block_info(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 2);

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] displayName:string Ody publicMessage:string 8XG7KBGNvm2  ");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[tokio::test]
    async fn test_double_document_replace_different_height_same_epoch() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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

        let mut altered_document_2 = altered_document.clone();

        altered_document_2.increment_revision().unwrap();
        altered_document_2.set("displayName", "Ody".into());
        altered_document_2.set("avatarUrl", "http://test.com/drapes.jpg".into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                profile,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
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
                platform_state.last_block_info(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let receiver_documents_sql_string = "select * from profile".to_string();

        let query_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &dashpay,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_400_000_000, 901, 43, 1, false); //next epoch

        let platform_state = platform.state.load();

        let documents_batch_update_transition_1 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_1 = documents_batch_update_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_update_transition_2 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document_2,
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

        let documents_batch_update_serialized_transition_2 = documents_batch_update_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_1.clone()],
                &platform_state,
                platform_state.last_block_info(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/cat.[...(23)] displayName:string Samuel publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_600_000_000, 902, 44, 1, false); //next epoch

        let platform_state = platform.state.load();

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_2.clone()],
                &platform_state,
                platform_state.last_block_info(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] displayName:string Ody publicMessage:string 8XG7KBGNvm2  ");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[tokio::test]
    async fn test_double_document_replace_no_change_different_height_same_epoch() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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

        let mut altered_document_2 = altered_document.clone();

        altered_document_2.increment_revision().unwrap();

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                profile,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
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
                platform_state.last_block_info(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let receiver_documents_sql_string = "select * from profile".to_string();

        let query_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &dashpay,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_400_000_000, 901, 43, 1, false); //next epoch

        let platform_state = platform.state.load();

        let documents_batch_update_transition_1 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_1 = documents_batch_update_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_update_transition_2 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document_2,
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

        let documents_batch_update_serialized_transition_2 = documents_batch_update_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_1.clone()],
                &platform_state,
                platform_state.last_block_info(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_600_000_000, 902, 44, 1, false); //next epoch

        let platform_state = platform.state.load();

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_2.clone()],
                &platform_state,
                platform_state.last_block_info(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[tokio::test]
    async fn test_double_document_replace_different_height_different_epoch() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        fast_forward_to_block(&platform, 1_200_000_000, 900, 42, 1, false); //next epoch

        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

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

        let mut altered_document_2 = altered_document.clone();

        altered_document_2.increment_revision().unwrap();
        altered_document_2.set("displayName", "Ody".into());
        altered_document_2.set("avatarUrl", "http://test.com/drapes.jpg".into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                profile,
                entropy.0,
                &key,
                2,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
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
                platform_state.last_block_info(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_eq!(processing_result.valid_count(), 1);

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let receiver_documents_sql_string = "select * from profile".to_string();

        let query_documents = DriveDocumentQuery::from_sql_expr(
            receiver_documents_sql_string.as_str(),
            &dashpay,
            Some(&platform.config.drive),
        )
        .expect("expected document query");

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-14 21:20:00 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/bob.[...(23)] displayName:string QBwBNNXXYCngB0er publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_400_000_000, 901, 43, 1, false); //next epoch

        let platform_state = platform.state.load();

        let documents_batch_update_transition_1 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                profile,
                &key,
                3,
                0,
                None,
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition_1 = documents_batch_update_transition_1
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let documents_batch_update_transition_2 =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document_2,
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

        let documents_batch_update_serialized_transition_2 = documents_batch_update_transition_2
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_1.clone()],
                &platform_state,
                platform_state.last_block_info(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-17 04:53:20 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/cat.[...(23)] displayName:string Samuel publicMessage:string 8XG7KBGNvm2  ");

        fast_forward_to_block(&platform, 1_600_000_000, 905, 44, 2, true); //next epoch

        let platform_state = platform.state.load();

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition_2.clone()],
                &platform_state,
                platform_state.last_block_info(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_eq!(processing_result.invalid_paid_count(), 0);

        assert_eq!(processing_result.invalid_unpaid_count(), 0);

        assert_eq!(processing_result.valid_count(), 1);

        let query_sender_results = platform
            .drive
            .query_documents(query_documents.clone(), None, false, None, None)
            .expect("expected query result");

        let document = query_sender_results
            .documents()
            .first()
            .expect("expected a document");

        assert_eq!(document.to_string(), "v0 : id:GcviwUsEr9Ji4rCrnnsgmVAghNaVPDumsfcagvBbBy45 owner_id:CisQdz2ej7EwWv8JbetSXBNsV4xsf8QsSS8tqp4tEf7V created_at:1970-01-14 21:20:00 updated_at:1970-01-19 12:26:40 avatarFingerprint:bytes d7b0e2b357c10312 avatarHash:bytes32 YonaRoE0hMgat53AYt5LTlQlIkKLReGpB7xNAqJ5HM8= avatarUrl:string http://test.com/drap[...(26)] displayName:string Ody publicMessage:string 8XG7KBGNvm2  ");

        let issues = platform
            .drive
            .grove
            .visualize_verify_grovedb(None, true, false, &platform_version.drive.grove_version)
            .expect("expected to have no issues");

        assert_eq!(
            issues.len(),
            0,
            "issues are {}",
            issues
                .iter()
                .map(|(hash, (a, b, c))| format!("{}: {} {} {}", hash, a, b, c))
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }

    #[tokio::test]
    async fn test_document_replace_on_document_type_that_requires_a_token() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut rng = StdRng::seed_from_u64(433);

        let platform_state = platform.state.load();

        let (contract_owner_id, _, _) = setup_identity(&mut platform, 958, dash_to_credits!(0.1));

        let (creator, signer, key) = setup_identity(&mut platform, 234, dash_to_credits!(0.1));

        let (contract, gold_token_id, gas_token_id) =
            create_card_game_internal_token_contract_with_owner_identity_burn_tokens(
                &mut platform,
                contract_owner_id.id(),
                platform_version,
            );

        let token_supply = platform
            .drive
            .fetch_token_total_supply(gold_token_id.to_buffer(), None, platform_version)
            .expect("expected to fetch total supply");

        assert_eq!(token_supply, Some(0));

        assert_eq!(contract.tokens().len(), 2);

        add_tokens_to_identity(&mut platform, gold_token_id, creator.id(), 15);
        add_tokens_to_identity(&mut platform, gas_token_id, creator.id(), 5);

        let card_document_type = contract
            .document_type_for_name("card")
            .expect("expected a profile document type");

        let entropy = Bytes32::random_with_rng(&mut rng);

        let mut document = card_document_type
            .random_document_with_identifier_and_entropy(
                &mut rng,
                creator.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");

        document.set("attack", 4.into());
        document.set("defense", 7.into());

        let mut altered_document = document.clone();

        altered_document.increment_revision().unwrap();
        altered_document.set("attack", 5.into());

        let documents_batch_create_transition =
            BatchTransition::new_document_creation_transition_from_document(
                document,
                card_document_type,
                entropy.0,
                &key,
                2,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 0,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(10),
                    gas_fees_paid_by: Default::default(),
                })),
                &signer,
                platform_version,
                None,
            )
            .await
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
                false,
                None,
            )
            .expect("expected to process state transition");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        let documents_batch_update_transition =
            BatchTransition::new_document_replacement_transition_from_document(
                altered_document,
                card_document_type,
                &key,
                3,
                0,
                Some(TokenPaymentInfo::V0(TokenPaymentInfoV0 {
                    payment_token_contract_id: None,
                    token_contract_position: 1,
                    minimum_token_cost: None,
                    maximum_token_cost: Some(2),
                    gas_fees_paid_by: Default::default(),
                })),
                &signer,
                platform_version,
                None,
            )
            .await
            .expect("expect to create documents batch transition");

        let documents_batch_update_serialized_transition = documents_batch_update_transition
            .serialize_to_bytes()
            .expect("expected documents batch serialized state transition");

        let transaction = platform.drive.grove.start_transaction();

        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &vec![documents_batch_update_serialized_transition.clone()],
                &platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process state transition");

        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit transaction");

        assert_matches!(
            processing_result.execution_results().as_slice(),
            [StateTransitionExecutionResult::SuccessfulExecution { .. }]
        );

        let token_balance = platform
            .drive
            .fetch_identity_token_balance(
                gas_token_id.to_buffer(),
                creator.id().to_buffer(),
                None,
                platform_version,
            )
            .expect("expected to fetch token balance");

        // He had 5, but spent 2
        assert_eq!(token_balance, Some(3));
    }
}
