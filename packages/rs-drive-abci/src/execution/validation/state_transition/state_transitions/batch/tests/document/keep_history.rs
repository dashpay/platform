use super::*;
use crate::rpc::core::MockCoreRPCLike;
use crate::test::helpers::setup::TempPlatform;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::platform_value::platform_value;
use dpp::state_transition::data_contract_update_transition::methods::DataContractUpdateTransitionMethodsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use drive::util::storage_flags::StorageFlags;

fn process_and_commit(
    platform: &mut TempPlatform<MockCoreRPCLike>,
    serialized: Vec<u8>,
) -> StateTransitionExecutionResult {
    let state = platform.state.load();
    let version = state.current_platform_version().unwrap();
    let transaction = platform.drive.grove.start_transaction();
    let result = platform
        .platform
        .process_raw_state_transitions(
            &[serialized],
            &state,
            &BlockInfo::default(),
            &transaction,
            version,
            false,
            None,
        )
        .expect("expected transition processing");
    platform
        .drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .unwrap();
    assert_eq!(result.execution_results().len(), 1);
    result.into_execution_results().remove(0)
}

/// A persisted v13 contract and document survive the v14 boundary, a repair,
/// and a subsequent ordinary contract update. Both writes use signed raw
/// transitions so parser validation, config compatibility and Drive all run.
#[tokio::test]
async fn should_repair_legacy_keep_history_contract_after_upgrade() {
    let old_version = PlatformVersion::get(13).unwrap();
    let new_version = PlatformVersion::get(14).unwrap();
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(13)
        .build_with_mock_rpc()
        .set_initial_state_structure();
    let (identity, signer, key) = setup_identity(&mut platform, 958, dash_to_credits!(0.5));
    let mut contract = json_document_to_contract(
        "tests/supporting_files/contract/note/note-contract-keep-history-and-can-be-deleted.json",
        true,
        old_version,
    )
    .expect("released protocol 13 accepts the legacy schema with full validation");
    contract.set_owner_id(identity.id());
    contract.set_config(DataContractConfig::default_for_version(old_version).unwrap());
    platform
        .drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            old_version,
        )
        .unwrap();

    let mut rng = StdRng::seed_from_u64(437);
    let entropy = Bytes32::random_with_rng(&mut rng);
    let document_type = contract.document_type_for_name("note").unwrap();
    let document = document_type
        .random_document_with_identifier_and_entropy(
            &mut rng,
            identity.id(),
            entropy,
            DocumentFieldFillType::FillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            old_version,
        )
        .unwrap();
    let create = BatchTransition::new_document_creation_transition_from_document(
        document,
        document_type,
        entropy.0,
        &key,
        1,
        0,
        None,
        &signer,
        old_version,
        None,
    )
    .await
    .unwrap();
    assert_matches!(
        process_and_commit(&mut platform, create.serialize_to_bytes().unwrap()),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );
    let query = DriveDocumentQuery::from_sql_expr(
        "select * from note",
        &contract,
        Some(&platform.config.drive),
        old_version,
    )
    .unwrap();
    let documents_before = platform
        .drive
        .query_documents(query, None, false, None, None)
        .unwrap()
        .documents()
        .to_vec();
    assert_eq!(documents_before.len(), 1);

    let mut upgraded_state = platform.state.load().as_ref().clone();
    upgraded_state.set_current_protocol_version_in_consensus(14);
    upgraded_state.set_next_epoch_protocol_version(14);
    platform.state.store(std::sync::Arc::new(upgraded_state));

    // Re-reading the actual stored contract at v14 must bypass the parser's
    // creation-time rule; do not rely only on the pre-upgrade cached object.
    let fetched = platform
        .drive
        .fetch_contract(contract.id().to_buffer(), None, None, None, new_version)
        .unwrap()
        .expect("legacy contract remains readable after activation")
        .unwrap();
    assert!(fetched
        .contract
        .document_type_for_name("note")
        .unwrap()
        .documents_can_be_deleted());

    let repaired_schema = platform_value!({
        "type": "object",
        "documentsKeepHistory": true,
        "documentsMutable": true,
        "canBeDeleted": false,
        "properties": {
            "message": {"type": "string", "maxLength": 256, "position": 0},
        },
        "required": ["message"],
        "additionalProperties": false,
    });
    contract.set_version(2);
    contract
        .set_document_schema("note", repaired_schema, true, &mut vec![], new_version)
        .unwrap();
    let update = DataContractUpdateTransition::new_from_data_contract(
        contract.clone(),
        &identity.clone().into_partial_identity_info(),
        key.id(),
        2,
        0,
        &signer,
        new_version,
        None,
    )
    .await
    .unwrap();
    assert_matches!(
        process_and_commit(&mut platform, update.serialize_to_bytes().unwrap()),
        StateTransitionExecutionResult::SuccessfulExecution { .. },
        "the repair must pass full validation and persist"
    );

    // A later ordinary update can add an unrelated document type.
    contract.set_version(3);
    contract
        .set_document_schema(
            "extra",
            platform_value!({
                "type": "object",
                "properties": {"label": {"type": "string", "maxLength": 20, "position": 0}},
                "additionalProperties": false,
            }),
            true,
            &mut vec![],
            new_version,
        )
        .unwrap();
    let update = DataContractUpdateTransition::new_from_data_contract(
        contract.clone(),
        &identity.clone().into_partial_identity_info(),
        key.id(),
        3,
        0,
        &signer,
        new_version,
        None,
    )
    .await
    .unwrap();
    assert_matches!(
        process_and_commit(&mut platform, update.serialize_to_bytes().unwrap()),
        StateTransitionExecutionResult::SuccessfulExecution { .. }
    );

    let stored_contract = platform
        .drive
        .fetch_contract(contract.id().to_buffer(), None, None, None, new_version)
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(stored_contract.contract.version(), 3);
    let stored_type = stored_contract
        .contract
        .document_type_for_name("note")
        .unwrap();
    assert!(stored_type.documents_keep_history());
    assert!(!stored_type.documents_can_be_deleted());
    let query = DriveDocumentQuery::from_sql_expr(
        "select * from note",
        &stored_contract.contract,
        Some(&platform.config.drive),
        new_version,
    )
    .unwrap();
    let documents_after = platform
        .drive
        .query_documents(query, None, false, None, None)
        .unwrap()
        .documents()
        .to_vec();
    assert_eq!(
        documents_before, documents_after,
        "repair must preserve existing documents"
    );
}
