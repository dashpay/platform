//! End-to-end coverage for `requiredSince`: a contract update adds a new
//! required property, and every document lifecycle op runs through real
//! state-transition processing before and after — proving grandfathered
//! documents keep working, new writes are held to the new schema, and the
//! contract-version stamp is assigned, preserved, and refreshed where the
//! design says it must be.

use super::*;

mod required_since_tests {
    use super::*;
    use crate::platform_types::platform_state::PlatformState;
    use crate::rpc::core::MockCoreRPCLike;
    use crate::test::helpers::setup::TempPlatform;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;
    use dpp::document::Document;
    use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
    use dpp::platform_value::platform_value;
    use dpp::prelude::DataContract;
    use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
    use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use dpp::state_transition::StateTransition;
    use dpp::tests::fixtures::get_data_contract_fixture;
    use drive::util::storage_flags::StorageFlags;

    /// The full grandfathering story, in order:
    /// 1. contract v1 with a `note` document type (one required property);
    /// 2. two documents created — both stamped with contract version 1;
    /// 3. the update to v2 adds required `extra` with `requiredSince: 2`;
    /// 4. a create omitting `extra` is consensus-rejected, one carrying it is
    ///    accepted and stamped 2;
    /// 5. the grandfathered documents — which do not have `extra` — still
    ///    transfer (stamp 1 preserved through the server-side rewrite) and
    ///    still delete;
    /// 6. replacing a grandfathered document must supply `extra` (rejected
    ///    without it) and re-stamps it to 2 — the lazy migration path.
    #[tokio::test]
    async fn test_contract_update_adding_required_field_grandfathers_existing_documents() {
        let platform_version = PlatformVersion::latest();
        let mut platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();
        let platform_state = platform.state.load();

        let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.5));
        let (receiver, receiver_signer, receiver_key) =
            setup_identity(&mut platform, 450, dash_to_credits!(0.5));

        // ------------------------------------------------------------------
        // Contract v1: `note` has a single required property `message`
        // ------------------------------------------------------------------
        let mut contract =
            get_data_contract_fixture(Some(identity.id()), 0, platform_version.protocol_version)
                .data_contract_owned();

        let note_schema_v1 = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "canBeDeleted": true,
            "transferable": 1,
            "properties": {
                "message": {"type": "string", "position": 0, "maxLength": 100_u32},
            },
            "required": ["message"],
            "additionalProperties": false
        });

        contract
            .set_document_schema(
                "note",
                note_schema_v1,
                true,
                &mut Vec::new(),
                platform_version,
            )
            .expect("expected to add the note document type");

        platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract");

        let note_type_v1 = contract
            .document_type_for_name("note")
            .expect("expected the note document type");

        let mut rng = StdRng::seed_from_u64(433);

        // ------------------------------------------------------------------
        // Two documents under contract v1 — the grandfathered generation
        // ------------------------------------------------------------------
        let mut grandfathered = Vec::new();
        for (nonce, seed_text) in [(1, "first note"), (2, "second note")] {
            let entropy = Bytes32::random_with_rng(&mut rng);
            let mut document = note_type_v1
                .random_document_with_identifier_and_entropy(
                    &mut rng,
                    identity.id(),
                    entropy,
                    DocumentFieldFillType::DoNotFillIfNotRequired,
                    DocumentFieldFillSize::AnyDocumentFillSize,
                    platform_version,
                )
                .expect("expected a random document");
            document.set("message", seed_text.into());

            let transition = BatchTransition::new_document_creation_transition_from_document(
                document.clone(),
                note_type_v1,
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
            .expect("expected a creation transition");

            let result = process_and_commit(&mut platform, &platform_state, &transition).await;
            assert_matches!(
                result,
                StateTransitionExecutionResult::SuccessfulExecution { .. },
                "creating a document under contract v1 must succeed"
            );
            grandfathered.push(document);
        }

        let stored = query_notes(&platform, &contract, platform_version);
        assert_eq!(stored.len(), 2);
        for document in &stored {
            assert_eq!(
                document.contract_version(),
                Some(1),
                "documents created under contract v1 must be stamped 1"
            );
        }

        // ------------------------------------------------------------------
        // The update: v2 adds required `extra` with `requiredSince: 2`
        // ------------------------------------------------------------------
        let note_schema_v2 = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "canBeDeleted": true,
            "transferable": 1,
            "properties": {
                "message": {"type": "string", "position": 0, "maxLength": 100_u32},
                "extra": {"type": "string", "position": 1, "maxLength": 50_u32, "requiredSince": 2},
            },
            "required": ["message", "extra"],
            "additionalProperties": false
        });

        let mut updated_contract = contract.clone();
        updated_contract.set_version(2);
        updated_contract
            .set_document_schema(
                "note",
                note_schema_v2,
                true,
                &mut Vec::new(),
                platform_version,
            )
            .expect("expected to update the note document type");

        let update_transition = DataContractUpdateTransition::new_from_data_contract(
            updated_contract.clone(),
            &identity.clone().into_partial_identity_info(),
            key.id(),
            3,
            0,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected an update transition");

        let result = process_and_commit_serialized(
            &mut platform,
            &platform_state,
            update_transition
                .serialize_to_bytes()
                .expect("expected serialized update"),
        )
        .await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "adding a required property with requiredSince = new version must be accepted"
        );

        let note_type_v2 = updated_contract
            .document_type_for_name("note")
            .expect("expected the updated note document type");

        // ------------------------------------------------------------------
        // New creates: without `extra` rejected, with it accepted + stamped 2
        // ------------------------------------------------------------------
        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut incomplete = note_type_v2
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");
        incomplete.set("message", "no extra".into());
        incomplete.remove("extra");

        let transition = BatchTransition::new_document_creation_transition_from_document(
            incomplete,
            note_type_v2,
            entropy.0,
            &key,
            4,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected a creation transition");

        let result = process_and_commit(&mut platform, &platform_state, &transition).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::PaidConsensusError { ref error, .. }
                if error.to_string().contains("extra"),
            "a create missing the newly required property must be consensus-rejected"
        );

        let entropy = Bytes32::random_with_rng(&mut rng);
        let mut complete = note_type_v2
            .random_document_with_identifier_and_entropy(
                &mut rng,
                identity.id(),
                entropy,
                DocumentFieldFillType::DoNotFillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
                platform_version,
            )
            .expect("expected a random document");
        complete.set("message", "with extra".into());
        complete.set("extra", "present".into());
        let complete_id = complete.id();

        let transition = BatchTransition::new_document_creation_transition_from_document(
            complete,
            note_type_v2,
            entropy.0,
            &key,
            5,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected a creation transition");

        let result = process_and_commit(&mut platform, &platform_state, &transition).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "a create carrying the newly required property must succeed"
        );

        let stored = query_notes(&platform, &updated_contract, platform_version);
        let stored_complete = stored
            .iter()
            .find(|d| d.id() == complete_id)
            .expect("expected the new document to be stored");
        assert_eq!(
            stored_complete.contract_version(),
            Some(2),
            "documents created under contract v2 must be stamped 2"
        );

        // ------------------------------------------------------------------
        // Grandfathered transfer: succeeds, stamp 1 preserved untouched
        // ------------------------------------------------------------------
        let transferred_id = grandfathered[0].id();
        let mut to_transfer = grandfathered[0].clone();
        to_transfer.set_revision(Some(2));
        let transition = BatchTransition::new_document_transfer_transition_from_document(
            to_transfer,
            note_type_v2,
            receiver.id(),
            &key,
            6,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected a transfer transition");

        let result = process_and_commit(&mut platform, &platform_state, &transition).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "a grandfathered document without the new property must stay transferable"
        );

        let stored = query_notes(&platform, &updated_contract, platform_version);
        let transferred = stored
            .iter()
            .find(|d| d.id() == transferred_id)
            .expect("expected the transferred document to be stored");
        assert_eq!(
            transferred.owner_id(),
            receiver.id(),
            "the transfer must have moved ownership"
        );
        assert_eq!(
            transferred.contract_version(),
            Some(1),
            "a transfer re-serializes the fetched document without touching it, \
             so the stamp must stay at the version its bytes conform to"
        );
        assert!(
            !transferred.properties().contains_key("extra"),
            "the grandfathered document must still omit the new property"
        );

        // ------------------------------------------------------------------
        // Grandfathered replace: must supply `extra`, and re-stamps to 2
        // ------------------------------------------------------------------
        let mut replacement_missing_extra = grandfathered[1].clone();
        replacement_missing_extra.set_revision(Some(2));
        replacement_missing_extra.set("message", "still no extra".into());

        let transition = BatchTransition::new_document_replacement_transition_from_document(
            replacement_missing_extra,
            note_type_v2,
            &key,
            7,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected a replacement transition");

        let result = process_and_commit(&mut platform, &platform_state, &transition).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::PaidConsensusError { ref error, .. }
                if error.to_string().contains("extra"),
            "a replace re-supplies full content, so it must carry the newly required property"
        );

        let replaced_id = grandfathered[1].id();
        let mut replacement = grandfathered[1].clone();
        replacement.set_revision(Some(2));
        replacement.set("message", "migrated".into());
        replacement.set("extra", "now present".into());

        let transition = BatchTransition::new_document_replacement_transition_from_document(
            replacement,
            note_type_v2,
            &key,
            8,
            0,
            None,
            &signer,
            platform_version,
            None,
        )
        .await
        .expect("expected a replacement transition");

        let result = process_and_commit(&mut platform, &platform_state, &transition).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "a replace carrying the newly required property must succeed"
        );

        let stored = query_notes(&platform, &updated_contract, platform_version);
        let replaced = stored
            .iter()
            .find(|d| d.id() == replaced_id)
            .expect("expected the replaced document to be stored");
        assert_eq!(
            replaced.contract_version(),
            Some(2),
            "a replace re-supplies content, so the document must be re-stamped — lazy migration"
        );
        assert_eq!(
            replaced
                .properties()
                .get_str("extra")
                .expect("expected the migrated property"),
            "now present"
        );

        // ------------------------------------------------------------------
        // Grandfathered delete: the transferred stamp-1 document still deletes
        // ------------------------------------------------------------------
        let mut to_delete = grandfathered[0].clone();
        to_delete.set_owner_id(receiver.id());

        let transition = BatchTransition::new_document_deletion_transition_from_document(
            to_delete,
            note_type_v2,
            &receiver_key,
            1,
            0,
            None,
            &receiver_signer,
            platform_version,
            None,
        )
        .await
        .expect("expected a deletion transition");

        let result = process_and_commit(&mut platform, &platform_state, &transition).await;
        assert_matches!(
            result,
            StateTransitionExecutionResult::SuccessfulExecution { .. },
            "a grandfathered document must stay deletable under the new schema"
        );

        let stored = query_notes(&platform, &updated_contract, platform_version);
        assert!(
            stored.iter().all(|d| d.id() != transferred_id),
            "the deleted document must be gone"
        );
    }

    async fn process_and_commit(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        transition: &StateTransition,
    ) -> StateTransitionExecutionResult {
        process_and_commit_serialized(
            platform,
            platform_state,
            transition
                .serialize_to_bytes()
                .expect("expected serialized transition"),
        )
        .await
    }

    async fn process_and_commit_serialized(
        platform: &mut TempPlatform<MockCoreRPCLike>,
        platform_state: &PlatformState,
        serialized: Vec<u8>,
    ) -> StateTransitionExecutionResult {
        let platform_version = platform_state
            .current_platform_version()
            .expect("expected the current platform version");
        let transaction = platform.drive.grove.start_transaction();
        let processing_result = platform
            .platform
            .process_raw_state_transitions(
                &[serialized],
                platform_state,
                &BlockInfo::default(),
                &transaction,
                platform_version,
                false,
                None,
            )
            .expect("expected to process the state transition");
        platform
            .drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit the transaction");
        processing_result.into_execution_results().remove(0)
    }

    fn query_notes(
        platform: &TempPlatform<MockCoreRPCLike>,
        contract: &DataContract,
        platform_version: &PlatformVersion,
    ) -> Vec<Document> {
        let query = DriveDocumentQuery::from_sql_expr(
            "select * from note",
            contract,
            Some(&platform.config.drive),
            platform_version,
        )
        .expect("expected a document query");
        platform
            .drive
            .query_documents(query, None, false, None, None)
            .expect("expected a query result")
            .documents()
            .to_vec()
    }
}
