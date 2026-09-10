use super::*;
use crate::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::document::DocumentV0Setters;
use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};

fn migrate_summable_history(revision_count: u64) -> DocumentHistoryMigrationStats {
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_factory::DocumentFactory;
    use dpp::platform_value::platform_value;
    use std::borrow::Cow;

    let old = PlatformVersion::get(13).unwrap();
    let new = PlatformVersion::get(14).unwrap();
    let schema = platform_value!({
        "type": "object", "documentsKeepHistory": true, "canBeDeleted": false,
        "documentsCountable": true, "documentsSummable": "amount",
        "properties": {"amount": {"type": "integer", "minimum": 0, "maximum": 4294967295i64, "position": 0}},
        "required": ["amount"], "additionalProperties": false,
        "indices": [{"name": "amount", "properties": [{"amount": "asc"}]}]
    });
    let contract = DataContractFactory::new(13)
        .unwrap()
        .create_with_value_config(
            [7; 32].into(),
            0,
            platform_value!({"tip": schema.clone(), "empty": schema}),
            None,
            None,
        )
        .unwrap()
        .data_contract_owned();
    let contract_flags = StorageFlags::new_single_epoch(7, Some([66; 32]));
    let directory = tempfile::TempDir::new().unwrap();
    let (drive, _) = Drive::open(
        directory.path(),
        Some(crate::config::DriveConfig {
            batching_consistency_verification: true,
            ..Default::default()
        }),
    )
    .unwrap();
    drive.create_initial_state_structure(None, old).unwrap();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            Some(Cow::Borrowed(&contract_flags)),
            None,
            old,
        )
        .unwrap();
    let document_type = contract.document_type_for_name("tip").unwrap();
    let mut document = DocumentFactory::new(13)
        .unwrap()
        .create_document(
            &contract,
            [7; 32].into(),
            "tip".into(),
            platform_value!({"amount": 1}),
        )
        .unwrap();
    document.set_id([9; 32].into());
    for revision in 1..=revision_count {
        document.set_revision(Some(revision));
        document.set("amount", revision.into());
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((
                            &document,
                            Some(Cow::Owned(StorageFlags::new_single_epoch(
                                (revision % 3) as u16,
                                Some([revision as u8; 32]),
                            ))),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::default_with_time(revision * 1000),
                true,
                None,
                old,
                None,
            )
            .unwrap();
    }
    if revision_count == 256 {
        if let Some(path) = std::env::var_os("DOCUMENT_HISTORY_MIGRATION_FIXTURE") {
            assert!(
                !std::path::Path::new(&path).exists(),
                "fixture destination must not exist"
            );
            drive.grove.create_checkpoint(path).unwrap();
        }
    }
    let transaction = drive.grove.start_transaction();
    let type_path = contract_document_type_path_vec(contract.id().as_slice(), "tip");
    let mut old_path = type_path.clone();
    old_path.extend([vec![0], document.id().to_vec()]);
    let mut ignored = DocumentHistoryMigrationStats::default();
    let old_entries = drive
        .history_migration_entries(&old_path, &transaction, old, &mut ignored)
        .unwrap();
    let stats = drive
        .migrate_document_history_storage(&transaction, new)
        .unwrap();
    assert_eq!(stats.types, 2);
    assert_eq!(stats.documents, 1);
    assert_eq!(stats.revisions, revision_count);
    let history = drive
        .history_migration_entries(
            &crate::drive::document::paths::document_history_path(
                contract.id().as_slice(),
                "tip",
                document.id().as_slice(),
            ),
            &transaction,
            new,
            &mut ignored,
        )
        .unwrap();
    for ((_, original), (_, migrated)) in old_entries
        .iter()
        .filter(|(key, _)| key != &[0])
        .zip(&history)
    {
        assert_eq!(
            original, migrated,
            "revision bytes and beneficiary flags must be preserved"
        );
    }
    let parent = drive
        .grove
        .get_raw(
            type_path.as_slice().into(),
            &[0],
            Some(&transaction),
            &new.drive.grove_version,
        )
        .value
        .unwrap();
    assert!(matches!(parent, Element::CountSumTree(_, 1, sum, _) if sum == revision_count as i64));
    let primary_path = old_path[..old_path.len() - 1].to_vec();
    let pointer = drive
        .grove
        .get_raw(
            primary_path.as_slice().into(),
            document.id().as_slice(),
            Some(&transaction),
            &new.drive.grove_version,
        )
        .value
        .unwrap();
    assert_eq!(
        pointer.get_flags(),
        old_entries
            .iter()
            .find(|(key, _)| key == &[0])
            .unwrap()
            .1
            .get_flags()
    );
    let history_parent = type_path
        .iter()
        .cloned()
        .chain([vec![DOCUMENT_HISTORY_TREE_KEY]])
        .collect::<Vec<_>>();
    let history_tree = drive
        .grove
        .get_raw(
            history_parent.as_slice().into(),
            document.id().as_slice(),
            Some(&transaction),
            &new.drive.grove_version,
        )
        .value
        .unwrap();
    assert_eq!(
        history_tree.get_flags(),
        &StorageFlags::map_to_some_element_flags(Some(&contract_flags)),
        "new structural bytes inherit the contract's flags"
    );
    let history_root = drive
        .grove
        .get_raw(
            type_path.as_slice().into(),
            &[DOCUMENT_HISTORY_TREE_KEY],
            Some(&transaction),
            &new.drive.grove_version,
        )
        .value
        .unwrap();
    assert_eq!(
        history_root.get_flags(),
        &StorageFlags::map_to_some_element_flags(Some(&contract_flags))
    );
    let empty_type = contract_document_type_path_vec(contract.id().as_slice(), "empty");
    let empty_history = drive
        .grove
        .get_raw(
            empty_type.as_slice().into(),
            &[DOCUMENT_HISTORY_TREE_KEY],
            Some(&transaction),
            &new.drive.grove_version,
        )
        .value
        .unwrap();
    assert_eq!(
        empty_history.get_flags(),
        &StorageFlags::map_to_some_element_flags(Some(&contract_flags))
    );
    assert!(matches!(empty_history, Element::Tree(..)));
    println!("revisions={revision_count} {stats:#?}");
    stats
}

#[test]
fn should_preserve_sums_counts_flags_and_empty_types_during_migration() {
    migrate_summable_history(8);
}

#[test]
fn should_measure_history_depth_instead_of_only_document_count() {
    let shallow = migrate_summable_history(16);
    let deep = migrate_summable_history(256);
    assert_eq!(shallow.documents, deep.documents);
    assert_eq!(deep.revisions, shallow.revisions * 16);
    assert!(deep.cost.storage_loaded_bytes > shallow.cost.storage_loaded_bytes * 8);
    assert!(deep.cost.hash_node_calls > shallow.cost.hash_node_calls);
}

#[test]
fn should_migrate_revisions_and_indexes_without_recovering_overwritten_revisions() {
    let drive = setup_drive_with_initial_state_structure(None);
    let old = PlatformVersion::get(13).unwrap();
    let new = PlatformVersion::get(14).unwrap();
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
        false,
        old,
    )
    .unwrap();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, old)
        .unwrap();
    let document_type = contract.document_type_for_name("profile").unwrap();
    let mut document = json_document_to_document(
        "tests/supporting_files/contract/dashpay/profile0.json",
        Some([7; 32].into()),
        document_type,
        old,
    )
    .unwrap();
    for (revision, time) in [(1, 1000), (2, 2000), (3, 2000)] {
        document.set_revision(Some(revision));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((
                            &document,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                true,
                BlockInfo::default_with_time(time),
                true,
                None,
                old,
                None,
            )
            .unwrap();
    }
    let root_before = drive
        .grove
        .root_hash(None, &old.drive.grove_version)
        .unwrap()
        .unwrap();
    let transaction = drive.grove.start_transaction();
    let stats = drive
        .migrate_document_history_storage(&transaction, new)
        .unwrap();
    assert_eq!(stats.documents, 1);
    assert_eq!(stats.revisions, 2);
    assert_eq!(stats.migrated_documents, 1);
    assert!(stats.index_entries > 0);
    assert_eq!(stats.rewritten_index_entries, stats.index_entries);
    let root_migrated = drive
        .grove
        .root_hash(Some(&transaction), &new.drive.grove_version)
        .unwrap()
        .unwrap();
    assert_ne!(root_before, root_migrated);
    assert_eq!(
        drive
            .grove
            .root_hash(None, &new.drive.grove_version)
            .unwrap()
            .unwrap(),
        root_before
    );

    let query = SingleDocumentDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "profile".into(),
        document_type_keeps_history: true,
        document_id: document.id().to_buffer(),
        block_time_ms: None,
        contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
    }
    .construct_path_query(new)
    .unwrap();
    let (current, _) = drive
        .grove_get_path_query(
            &query,
            Some(&transaction),
            QueryResultType::QueryElementResultType,
            &mut vec![],
            &new.drive,
        )
        .unwrap();
    assert_eq!(current.elements.len(), 1);
    let mut ignored_stats = DocumentHistoryMigrationStats::default();
    let history = drive
        .history_migration_entries(
            &crate::drive::document::paths::document_history_path(
                contract.id().as_slice(),
                "profile",
                document.id().as_slice(),
            ),
            &transaction,
            new,
            &mut ignored_stats,
        )
        .unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(&history[1].0[8..], encode_u64(3));
    use crate::drive::document::history::{DocumentHistoryQueryV1, DocumentHistorySelector};
    let mut history_query = DocumentHistoryQueryV1 {
        contract_id: contract.id().to_buffer(),
        document_type_name: "profile".into(),
        document_id: document.id().to_buffer(),
        selector: DocumentHistorySelector::StartAtTime(0),
        limit: None,
    };
    let page = drive
        .fetch_document_history_v1(&history_query, document_type, Some(&transaction), new)
        .unwrap();
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.revision)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert_eq!(page.lifecycle.remaining_revisions, 2);
    history_query.selector = DocumentHistorySelector::Revision(2);
    assert!(
        drive
            .fetch_document_history_v1(&history_query, document_type, Some(&transaction), new)
            .is_err(),
        "an overwritten revision must not return the next retained document"
    );
    let repeated = drive
        .migrate_document_history_storage(&transaction, new)
        .unwrap();
    assert_eq!(repeated.migrated_documents, 0);
    assert_eq!(repeated.revisions, 2);
    assert_eq!(
        drive
            .grove
            .root_hash(Some(&transaction), &new.drive.grove_version)
            .unwrap()
            .unwrap(),
        root_migrated
    );
    drop(transaction);
    let retry = drive.grove.start_transaction();
    drive.migrate_document_history_storage(&retry, new).unwrap();
    assert_eq!(
        drive
            .grove
            .root_hash(Some(&retry), &new.drive.grove_version)
            .unwrap()
            .unwrap(),
        root_migrated
    );
    drive.grove.commit_transaction(retry).unwrap().unwrap();
    history_query.selector = DocumentHistorySelector::StartAtTime(0);
    let (committed_page, proof) = drive
        .prove_document_history_v1(&history_query, document_type, None, new)
        .unwrap();
    assert_eq!(committed_page, page);
    assert_eq!(
        Drive::verify_document_history_v1(&history_query, &proof, document_type, new)
            .unwrap()
            .1,
        page
    );
    let mut legacy_entries = proof.clone();
    legacy_entries.entries_proof = Some(
        crate::util::test_helpers::history_proof::downgrade_history_count(
            proof.entries_proof.as_ref().unwrap(),
            None,
            new,
        ),
    );
    let error =
        Drive::verify_document_history_v1(&history_query, &legacy_entries, document_type, new)
            .expect_err("entries require a GroveDB v1 envelope");
    assert!(matches!(
        error,
        Error::Query(crate::error::query::QuerySyntaxError::Unsupported(_))
    ));
    use crate::drive::document::history::DocumentHistoryProofV1;
    for selector in [
        DocumentHistorySelector::Revision(3),
        DocumentHistorySelector::StartAtRevision(3),
    ] {
        history_query.selector = selector;
        let fetched = drive.fetch_document_history_v1(&history_query, document_type, None, new);
        let proved = drive.prove_document_history_v1(&history_query, document_type, None, new);
        let dishonest = DocumentHistoryProofV1 {
            entries_proof: Some(
                drive
                    .grove_get_proved_path_query(
                        &history_query.entries_query(new).unwrap(),
                        None,
                        &mut vec![],
                        &new.drive,
                    )
                    .unwrap(),
            ),
            metadata_proof: proof.metadata_proof.clone(),
        };
        let verified =
            Drive::verify_document_history_v1(&history_query, &dishonest, document_type, new);
        let mut downgraded = dishonest.clone();
        downgraded.metadata_proof =
            crate::util::test_helpers::history_proof::downgrade_history_count(
                &proof.metadata_proof,
                Some(3),
                new,
            );
        let (legacy_root, _) = grovedb::GroveDb::verify_query_with_options(
            &downgraded.metadata_proof,
            &history_query.metadata_query(new).unwrap(),
            grovedb::VerifyOptions {
                absence_proofs_for_non_existing_searched_keys: false,
                verify_proof_succinctness: true,
                include_empty_trees_in_result: true,
            },
            &new.drive.grove_version,
        )
        .unwrap();
        assert_eq!(
            legacy_root, root_migrated,
            "legacy terminal counts do not change the committed root"
        );
        let error =
            Drive::verify_document_history_v1(&history_query, &downgraded, document_type, new)
                .expect_err("history v1 rejects legacy metadata envelopes");
        assert!(matches!(
            error,
            Error::Query(crate::error::query::QuerySyntaxError::Unsupported(_))
        ));

        assert!(fetched.is_err() && proved.is_err() && verified.is_err(), "revision 3 exists in retained [1,3], so an empty ordinal page must be rejected: fetch rejected={}, prove rejected={}, verify rejected={}", fetched.is_err(), proved.is_err(), verified.is_err());
    }
    println!("{stats:#?}");
}

#[test]
fn should_reject_unrecognised_type_children_during_migration_inventory() {
    let old = PlatformVersion::get(13).unwrap();
    let new = PlatformVersion::get(14).unwrap();
    for (key, element) in [
        (vec![0, 9], Element::empty_tree()),
        (b"unknown".to_vec(), Element::empty_tree()),
        (b"item".to_vec(), Element::new_item(vec![])),
    ] {
        let drive = setup_drive_with_initial_state_structure(Some(old));
        let contract = json_document_to_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-with-profile-history.json",
            false,
            old,
        )
        .unwrap();
        drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, old)
            .unwrap();
        let path = contract_document_type_path_vec(contract.id().as_slice(), "profile");
        drive
            .grove
            .insert(
                path.as_slice(),
                &key,
                element,
                None,
                None,
                &old.drive.grove_version,
            )
            .value
            .unwrap();
        let transaction = drive.grove.start_transaction();
        assert!(
            drive
                .migrate_document_history_storage(&transaction, new)
                .is_err(),
            "unrecognised type child {key:?} cannot be silently skipped"
        );
    }
}
