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
        .query_documents(query, None, false, None, Some(13))
        .unwrap()
        .documents()
        .to_vec();
    assert_eq!(documents_before.len(), 1);

    let mut upgraded_state = platform.state.load().as_ref().clone();
    upgraded_state.set_current_protocol_version_in_consensus(14);
    upgraded_state.set_next_epoch_protocol_version(14);
    let migration = platform.drive.grove.start_transaction();
    platform
        .perform_events_on_first_block_of_protocol_change(
            &upgraded_state,
            &BlockInfo::default(),
            &migration,
            13,
            new_version,
        )
        .unwrap();
    platform
        .drive
        .grove
        .commit_transaction(migration)
        .value
        .unwrap();
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
        .query_documents(query, None, false, None, Some(14))
        .unwrap()
        .documents()
        .to_vec();
    assert_eq!(
        documents_before, documents_after,
        "repair must preserve existing documents"
    );
}

#[tokio::test]
async fn should_retain_replacements_transfers_prices_and_purchases_at_one_timestamp() {
    run_history_write_sequence(false).await;
}

#[tokio::test]
async fn should_replace_and_transfer_migrated_history_through_signed_transitions() {
    run_history_write_sequence(true).await;
}

async fn run_history_write_sequence(migrated: bool) {
    use dpp::data_contract::DataContractFactory;
    use drive::drive::document::history::{DocumentHistoryQueryV1, DocumentHistorySelector};
    let version = PlatformVersion::get(14).unwrap();
    let initial_version = PlatformVersion::get(if migrated { 13 } else { 14 }).unwrap();
    let mut platform = TestPlatformBuilder::new()
        .with_initial_protocol_version(initial_version.protocol_version)
        .build_with_mock_rpc()
        .set_initial_state_structure();
    let (owner, owner_signer, owner_key) = setup_identity(&mut platform, 958, dash_to_credits!(1));
    let (buyer, buyer_signer, buyer_key) = setup_identity(&mut platform, 450, dash_to_credits!(1));
    let contract = DataContractFactory::new(initial_version.protocol_version).unwrap().create_with_value_config(owner.id(), 0, platform_value!({
        "note": {
            "type": "object", "documentsKeepHistory": true, "documentsMutable": true,
            "canBeDeleted": false, "transferable": 1, "tradeMode": 1, "documentsCountable": true, "documentsSummable": "amount",
            "properties": { "message": { "type": "string", "maxLength": 256, "position": 0 }, "amount": {"type": "integer", "minimum": 0, "maximum": 4294967295i64, "position": 1} },
            "required": ["message", "amount"], "additionalProperties": false,
            "indices": [{"name": "owner", "properties": [{"$ownerId": "asc"}]}]
        }
    }), None, None).unwrap().data_contract_owned();
    platform
        .drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            None,
            None,
            initial_version,
        )
        .unwrap();
    let document_type = contract.document_type_for_name("note").unwrap();
    let mut rng = StdRng::seed_from_u64(937);
    let entropy = Bytes32::random_with_rng(&mut rng);
    let mut document = document_type
        .random_document_with_identifier_and_entropy(
            &mut rng,
            owner.id(),
            entropy,
            DocumentFieldFillType::DoNotFillIfNotRequired,
            DocumentFieldFillSize::AnyDocumentFillSize,
            version,
        )
        .unwrap();
    document.set("amount", 100u64.into());
    let query = DocumentHistoryQueryV1 {
        contract_id: contract.id().to_buffer(),
        document_type_name: "note".into(),
        document_id: document.id().to_buffer(),
        selector: DocumentHistorySelector::StartAtTime(0),
        limit: None,
    };
    for revision in 1..=5 {
        let write_version = if revision == 1 {
            initial_version
        } else {
            version
        };
        document.set_revision(Some(revision));
        let transition = match revision {
            1 => {
                BatchTransition::new_document_creation_transition_from_document(
                    document.clone(),
                    document_type,
                    entropy.0,
                    &owner_key,
                    1,
                    0,
                    None,
                    &owner_signer,
                    write_version,
                    None,
                )
                .await
            }
            2 => {
                document.set("message", "replaced".into());
                document.set("amount", 250u64.into());
                BatchTransition::new_document_replacement_transition_from_document(
                    document.clone(),
                    document_type,
                    &owner_key,
                    2,
                    0,
                    None,
                    &owner_signer,
                    write_version,
                    None,
                )
                .await
            }
            3 => {
                BatchTransition::new_document_transfer_transition_from_document(
                    document.clone(),
                    document_type,
                    buyer.id(),
                    &owner_key,
                    3,
                    0,
                    None,
                    &owner_signer,
                    write_version,
                    None,
                )
                .await
            }
            4 => {
                BatchTransition::new_document_update_price_transition_from_document(
                    document.clone(),
                    document_type,
                    100,
                    &buyer_key,
                    1,
                    0,
                    None,
                    &buyer_signer,
                    write_version,
                    None,
                )
                .await
            }
            5 => {
                BatchTransition::new_document_purchase_transition_from_document(
                    document.clone(),
                    document_type,
                    owner.id(),
                    100,
                    &owner_key,
                    4,
                    0,
                    None,
                    &owner_signer,
                    write_version,
                    None,
                )
                .await
            }
            _ => unreachable!(),
        }
        .unwrap();
        assert_matches!(
            process_and_commit(&mut platform, transition.serialize_to_bytes().unwrap()),
            StateTransitionExecutionResult::SuccessfulExecution { .. }
        );
        if migrated && revision == 1 {
            let mut upgraded = platform.state.load().as_ref().clone();
            upgraded.set_current_protocol_version_in_consensus(14);
            upgraded.set_next_epoch_protocol_version(14);
            let transaction = platform.drive.grove.start_transaction();
            platform
                .perform_events_on_first_block_of_protocol_change(
                    &upgraded,
                    &BlockInfo::default(),
                    &transaction,
                    13,
                    version,
                )
                .unwrap();
            platform
                .drive
                .grove
                .commit_transaction(transaction)
                .value
                .unwrap();
            platform.state.store(std::sync::Arc::new(upgraded));
        }
        let type_path = vec![
            vec![drive::drive::RootTree::DataContractDocuments as u8],
            contract.id().to_vec(),
            vec![1],
            b"note".to_vec(),
        ];
        let primary = platform
            .drive
            .grove
            .get_raw(
                type_path.as_slice().into(),
                &[0],
                None,
                &version.drive.grove_version,
            )
            .value
            .unwrap();
        assert!(matches!(primary, drive::grovedb::Element::CountSumTree(_, 1, sum, _) if sum == if revision == 1 {100} else {250}), "one live document contributes its current amount after each signed action: {primary:?}");
        let (history, proof) = platform
            .drive
            .prove_document_history_v1(&query, document_type, None, version)
            .unwrap();
        assert_eq!(history.lifecycle.remaining_revisions, revision);
        assert_eq!(
            history
                .entries
                .iter()
                .map(|entry| entry.revision)
                .collect::<Vec<_>>(),
            (1..=revision).collect::<Vec<_>>()
        );
        assert!(history.entries.iter().all(|entry| entry.time_ms == 0));
        let (_, verified) =
            drive::drive::Drive::verify_document_history_v1(&query, &proof, document_type, version)
                .unwrap();
        assert_eq!(verified, history);
        document = history.entries.last().unwrap().document.clone();
        assert_eq!(
            document.owner_id(),
            if [3, 4].contains(&revision) {
                buyer.id()
            } else {
                owner.id()
            }
        );
    }
}
