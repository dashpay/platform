use super::*;
use crate::drive::document::paths::document_history_path;
use crate::drive::RootTree;
use crate::query::{SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus};
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Setters;
use dpp::data_contract::config::v0::DataContractConfigGettersV0;
use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
use dpp::data_contract::validate_update::DataContractUpdateValidationMethodsV0;
use dpp::data_contract::{DataContract, DataContractFactory};
use dpp::document::document_factory::DocumentFactory;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::{platform_value, Value};
use dpp::tests::json_document::{
    json_document_to_contract, json_document_to_document, json_document_to_json_value,
};
use grovedb::AggregateData;
use grovedb_merk::element::reconstruct::ElementReconstructExtensions;
use std::borrow::Cow;

#[test]
fn should_accept_history_migration_when_only_primary_writer_changes() {
    let mut platform_version = PlatformVersion::latest().clone();
    let drive = setup_drive_with_initial_state_structure(Some(&platform_version));
    platform_version
        .drive
        .methods
        .document
        .insert
        .add_document_to_primary_storage = 2;
    let transaction = drive.grove.start_transaction();

    let stats = drive
        .migrate_document_history_storage(&transaction, &platform_version)
        .expect("the current history layout remains supported");
    assert_eq!(stats.migrated_documents, 0);
}

#[test]
fn should_reject_history_migration_for_legacy_layout_with_current_writer() {
    let mut platform_version = PlatformVersion::latest().clone();
    let drive = setup_drive_with_initial_state_structure(Some(&platform_version));
    platform_version
        .drive
        .methods
        .document
        .document_history_storage_layout = 0;
    let transaction = drive.grove.start_transaction();

    assert!(matches!(
        drive.migrate_document_history_storage(&transaction, &platform_version),
        Err(Error::Drive(DriveError::CorruptedDriveState(message)))
            if message == "document history migration requires the composite-key layout"
    ));
}

#[test]
fn should_reject_history_migration_for_unknown_layout() {
    let mut platform_version = PlatformVersion::latest().clone();
    let drive = setup_drive_with_initial_state_structure(Some(&platform_version));
    platform_version
        .drive
        .methods
        .document
        .document_history_storage_layout = 2;
    let transaction = drive.grove.start_transaction();

    assert!(matches!(
        drive.migrate_document_history_storage(&transaction, &platform_version),
        Err(Error::Drive(DriveError::UnknownVersionMismatch {
            method,
            known_versions,
            received: 2,
        })) if method == "document_history_storage_layout" && known_versions == vec![0, 1]
    ));
}

fn migrate_summable_history(revision_count: u64) -> DocumentHistoryMigrationStats {
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_factory::DocumentFactory;
    use dpp::platform_value::platform_value;
    use std::borrow::Cow;

    let old = PlatformVersion::get(14).unwrap();
    let new = PlatformVersion::get(15).unwrap();
    let schema = platform_value!({
        "type": "object", "documentsKeepHistory": true, "canBeDeleted": false,
        "documentsCountable": true, "documentsSummable": "amount",
        "properties": {"amount": {"type": "integer", "minimum": 0, "maximum": 4294967295i64, "position": 0}},
        "required": ["amount"], "additionalProperties": false,
        "indices": [{"name": "amount", "properties": [{"amount": "asc"}]}]
    });
    let contract = DataContractFactory::new(14)
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
    let mut document = DocumentFactory::new(14)
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
    let old_entries = drive
        .history_migration_entries(&old_path, &transaction, old)
        .unwrap();
    let legacy_tree = drive
        .grove
        .get_raw(
            (&old_path[..old_path.len() - 1]).into(),
            document.id().as_slice(),
            Some(&transaction),
            &old.drive.grove_version,
        )
        .value
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
        )
        .unwrap();
    assert_eq!(
        old_entries.iter().filter(|(key, _)| key != &[0]).count(),
        history.len(),
        "every retained source revision must have a destination revision",
    );
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
        legacy_tree.get_flags(),
        "the history tree keeps the flags of the per-document tree it replaces"
    );
    assert_eq!(
        legacy_tree.get_flags(),
        &None,
        "the legacy writer leaves a history tree unflagged when the contract can not be deleted"
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
}

#[test]
fn should_migrate_revisions_and_indexes_without_recovering_overwritten_revisions() {
    let drive = setup_drive_with_initial_state_structure(None);
    let old = PlatformVersion::get(14).unwrap();
    let new = PlatformVersion::get(15).unwrap();
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
    assert_eq!(
        assert_index_references_rewritten(&drive, &contract, "profile", &transaction),
        stats.index_entries
    );
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
    let history = drive
        .history_migration_entries(
            &crate::drive::document::paths::document_history_path(
                contract.id().as_slice(),
                "profile",
                document.id().as_slice(),
            ),
            &transaction,
            new,
        )
        .unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(&history[1].0[8..], encode_u64(3));
    use crate::query::document_history_drive_query::{
        DocumentHistoryDriveQuery, DocumentHistoryFilter,
    };
    let mut history_query = DocumentHistoryDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "profile".into(),
        document_id: document.id().to_buffer(),
        filter: DocumentHistoryFilter::StartAtTime(0),
        limit: None,
    };
    let page = drive
        .fetch_document_history(&history_query, document_type, Some(&transaction), new)
        .unwrap();
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.revision)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
    assert_eq!(page.lifecycle.as_ref().unwrap().remaining_revisions, 2);
    history_query.filter = DocumentHistoryFilter::Revision(2);
    assert!(
        drive
            .fetch_document_history(&history_query, document_type, Some(&transaction), new)
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
    history_query.filter = DocumentHistoryFilter::StartAtTime(0);
    let committed_page = drive
        .fetch_document_history(&history_query, document_type, None, new)
        .unwrap();
    let proof = drive
        .prove_document_history(&history_query, document_type, None, new)
        .unwrap();
    assert_eq!(committed_page, page);
    assert_eq!(
        Drive::verify_document_history(&history_query, &proof, document_type, new,)
            .unwrap()
            .1,
        page
    );
    let legacy_entries =
        crate::util::test_helpers::history_proof::downgrade_history_count(&proof, None, new);
    let error = Drive::verify_document_history(&history_query, &legacy_entries, document_type, new)
        .expect_err("entries require a GroveDB v1 envelope");
    assert!(matches!(
        error,
        Error::Proof(
            crate::error::proof::ProofError::UnsupportedGroveDBProofEnvelopeVersion { .. }
        )
    ));
    for filter in [
        DocumentHistoryFilter::Revision(3),
        DocumentHistoryFilter::StartAtRevision(3),
    ] {
        history_query.filter = filter;
        let fetched = drive.fetch_document_history(&history_query, document_type, None, new);
        let proved = drive.prove_document_history(&history_query, document_type, None, new);
        let dishonest = drive
            .grove_get_proved_path_query(
                &history_query.construct_path_query(new).unwrap(),
                None,
                &mut vec![],
                &new.drive,
            )
            .unwrap();
        let verified =
            Drive::verify_document_history(&history_query, &dishonest, document_type, new);
        let downgraded = crate::util::test_helpers::history_proof::downgrade_history_count(
            &dishonest,
            Some(3),
            new,
        );
        let error = Drive::verify_document_history(&history_query, &downgraded, document_type, new)
            .expect_err("document history rejects legacy metadata envelopes");
        assert!(matches!(
            error,
            Error::Proof(
                crate::error::proof::ProofError::UnsupportedGroveDBProofEnvelopeVersion { .. }
            )
        ));

        assert!(fetched.is_err() && proved.is_err() && verified.is_err(), "revision 3 exists in retained [1,3], so an empty ordinal page must be rejected: fetch rejected={}, prove rejected={}, verify rejected={}", fetched.is_err(), proved.is_err(), verified.is_err());
    }
    println!("{stats:#?}");
}

#[test]
fn should_reject_unrecognised_type_children_during_migration_inventory() {
    let old = PlatformVersion::get(14).unwrap();
    let new = PlatformVersion::get(15).unwrap();
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

/// A keep-history type with a contested index could be registered before
/// protocol 15. Activation loads every stored contract through the structural
/// parse, which must keep reading that combination, or the migration would
/// abort on the first such contract before touching any history.
#[test]
fn should_migrate_a_legacy_contested_keep_history_contract() {
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_factory::DocumentFactory;
    use dpp::platform_value::platform_value;

    let old = PlatformVersion::get(14).unwrap();
    let new = PlatformVersion::get(15).unwrap();
    let schema = platform_value!({
        "type": "object",
        "documentsKeepHistory": true,
        "canBeDeleted": false,
        "documentsMutable": false,
        "properties": {
            "label": {"type": "string", "maxLength": 50, "position": 0},
        },
        "required": ["label"],
        "additionalProperties": false,
        "indices": [{
            "name": "byLabel",
            "properties": [{"label": "asc"}],
            "unique": true,
            "contested": {
                "fieldMatches": [{"field": "label", "regexPattern": "^[a-z]{3,10}$"}],
                "resolution": 0,
            },
        }],
    });
    let contract = DataContractFactory::new(14)
        .unwrap()
        .create_with_value_config(
            [7; 32].into(),
            0,
            platform_value!({"name": schema}),
            None,
            None,
        )
        .expect("protocol 14 admits a contested keep-history type")
        .data_contract_owned();
    let drive = setup_drive_with_initial_state_structure(None);
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, old)
        .unwrap();
    let document_type = contract.document_type_for_name("name").unwrap();
    let mut document = DocumentFactory::new(14)
        .unwrap()
        .create_document(
            &contract,
            [7; 32].into(),
            "name".into(),
            platform_value!({"label": "alice"}),
        )
        .unwrap();
    // An immutable type carries no revision; its history is keyed by time.
    document.set_id([9; 32].into());
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::default_with_time(1_000),
            true,
            None,
            old,
        )
        .unwrap();

    let transaction = drive.grove.start_transaction();
    let stats = drive
        .migrate_document_history_storage(&transaction, new)
        .expect("a stored contested keep-history contract must load and migrate");
    assert_eq!(stats.contracts, 1);
    assert_eq!(stats.types, 1);
    assert_eq!(stats.migrated_documents, 1);
}

/// Writes `document` of type `name` as protocol 14 does, in the block at
/// `time_ms`, with `flags`.
fn add_legacy_revision(
    drive: &Drive,
    contract: &DataContract,
    name: &str,
    document: &Document,
    flags: StorageFlags,
    time_ms: u64,
) {
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((
                        document,
                        Some(Cow::Owned(flags)),
                    )),
                    owner_id: None,
                },
                contract,
                document_type: contract.document_type_for_name(name).unwrap(),
            },
            true,
            BlockInfo::default_with_time(time_ms),
            true,
            None,
            PlatformVersion::get(14).unwrap(),
        )
        .unwrap();
}

/// Checks that every index reference of type `name` targets its document's
/// current pointer, two hops from the revision it resolves to, and returns
/// how many there are.
fn assert_index_references_rewritten(
    drive: &Drive,
    contract: &DataContract,
    name: &str,
    transaction: &Transaction,
) -> u64 {
    let version = PlatformVersion::latest();
    let type_path = contract_document_type_path_vec(contract.id().as_slice(), name);
    let mut stats = DocumentHistoryMigrationStats::default();
    let mut inventory = IndexEntries::new();
    for index in contract
        .document_type_for_name(name)
        .unwrap()
        .index_structure()
        .sub_levels()
        .keys()
    {
        let mut path = type_path.clone();
        path.push(index.as_bytes().to_vec());
        drive
            .history_migration_index_entries(path, transaction, &mut stats, &mut inventory, version)
            .unwrap();
    }
    for (document_id, references) in inventory {
        let current_pointer = vec![vec![0], document_id];
        for (path, key, element) in references {
            assert!(
                matches!(
                    &element,
                    Element::Reference(UpstreamRootHeightReference(4, target), Some(2), _)
                    | Element::ReferenceWithSumItem(
                        UpstreamRootHeightReference(4, target),
                        Some(2),
                        _,
                        _
                    ) if target == &current_pointer
                ),
                "index reference {path:?}/{key:?} was not rewritten: {element:?}"
            );
        }
    }
    stats.index_entries
}

/// A keep-history type `note` whose property `o` is an object holding
/// `members`, in this order, in contract version `version`.
fn nested_member_contract(members: Vec<(&str, Value)>, version: u32) -> DataContract {
    let members = Value::Map(
        members
            .into_iter()
            .map(|(name, schema)| (Value::Text(name.to_string()), schema))
            .collect(),
    );
    let schema = platform_value!({
        "type": "object", "documentsKeepHistory": true, "documentsMutable": true,
        "canBeDeleted": false,
        "properties": {
            "o": {
                "type": "object", "position": 0, "properties": members,
                "required": ["b", "c"], "additionalProperties": false,
            },
        },
        "required": ["o"], "additionalProperties": false,
    });
    let mut contract = DataContractFactory::new(14)
        .unwrap()
        .create_with_value_config(
            [7; 32].into(),
            0,
            platform_value!({"note": schema}),
            None,
            None,
        )
        .unwrap()
        .data_contract_owned();
    contract.set_version(version);
    contract
}

/// Protocol 14 accepts a contract update that reorders a nested object's
/// members or prepends an optional one, and properties are encoded in member
/// order. Revisions stored before such an update keep their bytes, so the
/// migration moves them without reading their properties.
#[test]
fn should_migrate_revisions_stored_before_a_nested_property_change() {
    let old = PlatformVersion::get(14).unwrap();
    let new = PlatformVersion::latest();
    let a = platform_value!({"type": "string", "maxLength": 20, "position": 2});
    let b = platform_value!({"type": "string", "maxLength": 20, "position": 0});
    let c = platform_value!({"type": "integer", "minimum": 0, "maximum": 10, "position": 1});
    let original = nested_member_contract(vec![("b", b.clone()), ("c", c.clone())], 1);
    for (change, updated) in [
        (
            "reordered members",
            nested_member_contract(vec![("c", c.clone()), ("b", b.clone())], 2),
        ),
        (
            "prepended optional member",
            nested_member_contract(
                vec![("a", a.clone()), ("b", b.clone()), ("c", c.clone())],
                2,
            ),
        ),
    ] {
        assert!(
            original
                .validate_update(&updated, &BlockInfo::default(), old)
                .unwrap()
                .is_valid(),
            "protocol 14 accepts the update with {change}"
        );
        let drive = setup_drive_with_initial_state_structure(Some(old));
        drive
            .apply_contract(&original, BlockInfo::default(), true, None, None, old)
            .unwrap();
        let mut document = DocumentFactory::new(14)
            .unwrap()
            .create_document(
                &original,
                [7; 32].into(),
                "note".into(),
                platform_value!({"o": {"b": "hello", "c": 5}}),
            )
            .unwrap();
        document.set_id([9; 32].into());
        for revision in 1..=2u64 {
            document.set_revision(Some(revision));
            document.set("o", platform_value!({"b": "hello", "c": revision + 3}));
            add_legacy_revision(
                &drive,
                &original,
                "note",
                &document,
                StorageFlags::new_single_epoch(revision as u16, Some([7; 32])),
                revision * 1000,
            );
        }
        drive
            .apply_contract(
                &updated,
                BlockInfo::default_with_time(5000),
                true,
                None,
                None,
                old,
            )
            .unwrap();

        let transaction = drive.grove.start_transaction();
        let mut legacy_path = contract_document_type_path_vec(original.id().as_slice(), "note");
        legacy_path.extend([vec![0], document.id().to_vec()]);
        let legacy = drive
            .history_migration_entries(&legacy_path, &transaction, old)
            .unwrap()
            .into_iter()
            .filter(|(key, _)| key != &[0])
            .collect::<Vec<_>>();
        assert_eq!(legacy.len(), 2);
        let Element::Item(stored, _) = &legacy[0].1 else {
            panic!("a retained revision is an item");
        };
        assert!(
            Document::from_bytes(stored, updated.document_type_for_name("note").unwrap(), new)
                .is_err(),
            "the fixture must hold revisions the updated type no longer reads ({change})"
        );

        let stats = drive
            .migrate_document_history_storage(&transaction, new)
            .unwrap_or_else(|error| panic!("migration after {change} failed: {error}"));
        assert_eq!(stats.migrated_documents, 1);
        let migrated = drive
            .history_migration_entries(
                &document_history_path(original.id().as_slice(), "note", document.id().as_slice()),
                &transaction,
                new,
            )
            .unwrap();
        let expected = legacy
            .into_iter()
            .zip(1u64..)
            .map(|((time, element), revision)| {
                let mut key =
                    encode_u64(DocumentPropertyType::decode_date_timestamp(&time).unwrap());
                key.extend(encode_u64(revision));
                (key, element)
            })
            .collect::<Vec<_>>();
        assert_eq!(
            migrated, expected,
            "every revision keeps its bytes and flags under its time and revision ({change})"
        );
    }
}

/// The legacy writer gave each document's history tree the document's own
/// flags when the contract can be deleted. The migrated tree keeps them, so
/// erasing the document's history credits its owner, not the contract's owner.
#[test]
fn should_keep_the_owner_flags_on_the_migrated_history_tree() {
    let old = PlatformVersion::get(14).unwrap();
    let new = PlatformVersion::latest();
    let contract_owner = [66; 32];
    let document_owner = [14; 32];
    // A deletable keep-history type is refused at registration since protocol
    // 14, but stored contracts from before hold them, so the stored contract is
    // read without validation.
    let mut schema = json_document_to_json_value(
        "tests/supporting_files/contract/family/family-contract-with-history.json",
    )
    .unwrap();
    schema["config"] = serde_json::json!({"$formatVersion": "0", "canBeDeleted": true});
    let contract = DataContract::from_json(schema, false, old).unwrap();
    assert!(contract.config().can_be_deleted());
    let drive = setup_drive_with_initial_state_structure(Some(old));
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            Some(Cow::Owned(StorageFlags::new_single_epoch(
                0,
                Some(contract_owner),
            ))),
            None,
            old,
        )
        .unwrap();
    let document_type = contract.document_type_for_name("person").unwrap();
    let mut document = json_document_to_document(
        "tests/supporting_files/contract/family/person0.json",
        Some(document_owner.into()),
        document_type,
        old,
    )
    .unwrap();
    let owner_flags = StorageFlags::new_single_epoch(0, Some(document_owner));
    for revision in 1..=2u64 {
        document.set_revision(Some(revision));
        add_legacy_revision(
            &drive,
            &contract,
            "person",
            &document,
            owner_flags.clone(),
            revision * 1000,
        );
    }
    let type_path = contract_document_type_path_vec(contract.id().as_slice(), "person");
    let mut primary_path = type_path.clone();
    primary_path.push(vec![0]);
    let legacy_tree = drive
        .grove
        .get_raw(
            primary_path.as_slice().into(),
            document.id().as_slice(),
            None,
            &old.drive.grove_version,
        )
        .value
        .unwrap();
    assert_eq!(
        legacy_tree.get_flags(),
        &StorageFlags::map_to_some_element_flags(Some(&owner_flags)),
        "the legacy writer flags a deletable contract's history tree with its document's flags"
    );

    let transaction = drive.grove.start_transaction();
    drive
        .migrate_document_history_storage(&transaction, new)
        .unwrap();
    drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .unwrap();
    let mut history_root = type_path.clone();
    history_root.push(vec![DOCUMENT_HISTORY_TREE_KEY]);
    let history_tree = drive
        .grove
        .get_raw(
            history_root.as_slice().into(),
            document.id().as_slice(),
            None,
            &new.drive.grove_version,
        )
        .value
        .unwrap();
    assert!(matches!(history_tree, Element::ProvableCountTree(..)));
    assert_eq!(history_tree.get_flags(), legacy_tree.get_flags());

    let mut operations = vec![];
    let delete = drive
        .delete_document_for_contract_operations(
            document.id(),
            &contract,
            document_type,
            &BlockInfo::default_with_time(5_000),
            Some(document_owner.into()),
            None,
            &mut None,
            None,
            new,
        )
        .unwrap();
    drive
        .apply_batch_low_level_drive_operations(None, None, delete, &mut operations, &new.drive)
        .unwrap();
    let erase_block = BlockInfo::default_with_time(6_000);
    let erase = drive
        .erase_document_for_contract_operations(
            document.id(),
            &contract,
            document_type,
            &erase_block,
            &mut None,
            None,
            new,
        )
        .unwrap();
    let mut operations = vec![];
    drive
        .apply_batch_low_level_drive_operations(None, None, erase, &mut operations, &new.drive)
        .unwrap();
    let refunds = drive
        .calculate_fee(None, Some(operations), &erase_block.epoch, new)
        .unwrap()
        .fee_refunds;
    assert!(
        drive
            .grove
            .get_raw(
                history_root.as_slice().into(),
                document.id().as_slice(),
                None,
                &new.drive.grove_version,
            )
            .value
            .is_err(),
        "the erase removes the history tree"
    );
    assert!(refunds.0.contains_key(&document_owner));
    assert!(
        !refunds.0.contains_key(&contract_owner),
        "nothing the erase removes was paid for by the contract's owner: {refunds:?}"
    );
}

/// Two contracts holding legacy histories of every shape the migration moves:
/// a summable type, an indexed type with several revisions (two of them
/// written in one block, so one is overwritten), an immutable type, and a type
/// with no documents.
fn legacy_mixed_histories() -> (Drive, [DataContract; 2]) {
    let old = PlatformVersion::get(14).unwrap();
    let drive = setup_drive_with_initial_state_structure(Some(old));
    let note = platform_value!({
        "type": "object", "documentsKeepHistory": true, "documentsMutable": true,
        "canBeDeleted": false,
        "properties": {
            "label": {"type": "string", "maxLength": 32, "position": 0},
            "rank": {"type": "integer", "minimum": 0, "maximum": 1000, "position": 1},
        },
        "required": ["label", "rank"], "additionalProperties": false,
        "indices": [
            {"name": "byLabel", "properties": [{"label": "asc"}]},
            {"name": "byRank", "properties": [{"rank": "asc"}]},
        ],
    });
    let tip = platform_value!({
        "type": "object", "documentsKeepHistory": true, "canBeDeleted": false,
        "documentsCountable": true, "documentsSummable": "amount",
        "properties": {"amount": {"type": "integer", "minimum": 0, "maximum": 4294967295i64, "position": 0}},
        "required": ["amount"], "additionalProperties": false,
        "indices": [{"name": "amount", "properties": [{"amount": "asc"}]}],
    });
    let stamp = platform_value!({
        "type": "object", "documentsKeepHistory": true, "documentsMutable": false,
        "canBeDeleted": false,
        "properties": {"label": {"type": "string", "maxLength": 32, "position": 0}},
        "required": ["label"], "additionalProperties": false,
        "indices": [{"name": "byLabel", "properties": [{"label": "asc"}]}],
    });
    let factory = DataContractFactory::new(14).unwrap();
    let first = factory
        .create_with_value_config(
            [7; 32].into(),
            0,
            platform_value!({"note": note.clone(), "tip": tip, "stamp": stamp, "empty": note.clone()}),
            None,
            None,
        )
        .unwrap()
        .data_contract_owned();
    let second = factory
        .create_with_value_config(
            [8; 32].into(),
            1,
            platform_value!({"note": note}),
            None,
            None,
        )
        .unwrap()
        .data_contract_owned();
    for (contract, epoch) in [(&first, 1), (&second, 2)] {
        drive
            .apply_contract(
                contract,
                BlockInfo::default(),
                true,
                Some(Cow::Owned(StorageFlags::new_single_epoch(
                    epoch,
                    Some(contract.owner_id().to_buffer()),
                ))),
                None,
                old,
            )
            .unwrap();
    }
    // (contract, type, id byte, each write's block time and values)
    let histories = vec![
        (
            &first,
            "note",
            1,
            vec![(1_000, platform_value!({"label": "a", "rank": 1}))],
        ),
        (
            &first,
            "note",
            2,
            vec![
                (1_000, platform_value!({"label": "b", "rank": 2})),
                (2_000, platform_value!({"label": "b", "rank": 3})),
                (2_000, platform_value!({"label": "b2", "rank": 4})),
                (3_000, platform_value!({"label": "b3", "rank": 5})),
            ],
        ),
        (
            &first,
            "note",
            3,
            vec![(1_500, platform_value!({"label": "c", "rank": 2}))],
        ),
        (
            &first,
            "tip",
            4,
            vec![
                (1_000, platform_value!({"amount": 5})),
                (4_000, platform_value!({"amount": 7})),
            ],
        ),
        (
            &first,
            "tip",
            5,
            vec![(1_000, platform_value!({"amount": 11}))],
        ),
        (
            &first,
            "stamp",
            6,
            vec![(2_500, platform_value!({"label": "s"}))],
        ),
        (
            &second,
            "note",
            7,
            vec![
                (1_000, platform_value!({"label": "x", "rank": 9})),
                (5_000, platform_value!({"label": "y", "rank": 9})),
            ],
        ),
        (
            &second,
            "note",
            8,
            vec![(1_000, platform_value!({"label": "z", "rank": 0}))],
        ),
    ];
    for (contract, name, id, writes) in histories {
        let mut document = DocumentFactory::new(14)
            .unwrap()
            .create_document(
                contract,
                contract.owner_id(),
                name.into(),
                writes[0].1.clone(),
            )
            .unwrap();
        document.set_id([id; 32].into());
        for (index, (time, value)) in writes.into_iter().enumerate() {
            if document.revision().is_some() {
                document.set_revision(Some(index as u64 + 1));
            }
            if let Value::Map(fields) = value {
                for (field, value) in fields {
                    document.set(field.as_text().unwrap(), value);
                }
            }
            add_legacy_revision(
                &drive,
                contract,
                name,
                &document,
                StorageFlags::new_single_epoch((time / 1_000) as u16, Some([id + 100; 32])),
                time,
            );
        }
    }
    (drive, [first, second])
}

/// Every element under `path`, by its path below `path`, with each tree's
/// root key cleared and its count and sum kept: the root key records the
/// shape of the tree under it, which depends on how its writes were grouped
/// into batches.
fn stored_elements(
    drive: &Drive,
    path: &[Vec<u8>],
    transaction: &Transaction,
) -> BTreeMap<Vec<Vec<u8>>, Element> {
    let version = PlatformVersion::latest();
    let mut elements = BTreeMap::new();
    for (key, element) in drive
        .history_migration_entries(path, transaction, version)
        .unwrap()
    {
        if !element.is_any_tree() {
            elements.insert(vec![key], element);
            continue;
        }
        let mut child = path.to_vec();
        child.push(key.clone());
        for (below, element) in stored_elements(drive, &child, transaction) {
            elements.insert([vec![key.clone()], below].concat(), element);
        }
        let aggregate = AggregateData::CountAndSum(
            element.count_value_or_default(),
            element.sum_value_or_default(),
        );
        elements.insert(
            vec![key],
            element.reconstruct_with_root_key(None, aggregate).unwrap(),
        );
    }
    elements
}

/// Migrates in batches of `max_batch_operations` operations, commits, and
/// returns every element the contracts' documents tree holds afterwards.
fn migrated_elements(
    drive: &Drive,
    contracts: &[DataContract],
    max_batch_operations: u32,
) -> (
    BTreeMap<Vec<Vec<u8>>, Element>,
    DocumentHistoryMigrationStats,
) {
    let mut version = PlatformVersion::latest().clone();
    version.system_limits.max_history_migration_batch_operations = Some(max_batch_operations);
    let transaction = drive.grove.start_transaction();
    let stats = drive
        .migrate_document_history_storage(&transaction, &version)
        .unwrap();
    drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .unwrap();
    let issues = drive
        .grove
        .verify_grovedb(None, true, false, &version.drive.grove_version)
        .unwrap();
    assert!(
        issues.is_empty(),
        "batches of {max_batch_operations} operations leave inconsistent hashes: {issues:?}"
    );
    let transaction = drive.grove.start_transaction();
    let mut index_entries = 0;
    for contract in contracts {
        for name in contract.document_types().keys() {
            index_entries += assert_index_references_rewritten(drive, contract, name, &transaction);
        }
    }
    assert_eq!(index_entries, stats.rewritten_index_entries);
    let elements = stored_elements(
        drive,
        &[vec![RootTree::DataContractDocuments as u8]],
        &transaction,
    );
    (elements, stats)
}

/// The batch size decides only how the writes are grouped, so every size
/// leaves the same elements, and GroveDB's own hashes agree with them, also
/// when a batch ends inside a type or between contracts, holds several
/// documents, or holds less than one document's writes.
#[test]
fn should_store_the_same_histories_whatever_the_batch_size() {
    let default = PlatformVersion::latest()
        .system_limits
        .max_history_migration_batch_operations
        .unwrap();
    let (drive, contracts) = legacy_mixed_histories();
    let (expected, expected_stats) = migrated_elements(&drive, &contracts, 1);
    assert_eq!(expected_stats.migrated_documents, 8);
    assert_eq!(expected_stats.revisions, 12);
    assert_eq!(expected_stats.rewritten_index_entries, 13);
    assert_eq!(
        expected_stats.batches,
        5 + 8 * 2,
        "one batch per history tree a type lacks, then per document its moves and its links"
    );
    // Every document here writes at least seven operations, so up to seven a
    // batch holds one document; from sixteen a batch holds several and can end
    // inside a type; from seventy-four one pair of batches holds everything.
    for (max_batch_operations, batches) in [
        (2, 17),
        (7, 16),
        (16, 8),
        (20, 6),
        (default, 2),
        (u32::MAX, 2),
    ] {
        let (drive, contracts) = legacy_mixed_histories();
        let (elements, stats) = migrated_elements(&drive, &contracts, max_batch_operations);
        assert_eq!(
            elements, expected,
            "batches of {max_batch_operations} operations"
        );
        assert_eq!(
            stats.batches, batches,
            "batches of {max_batch_operations} operations"
        );
        assert_eq!(stats.migrated_documents, expected_stats.migrated_documents);
        assert_eq!(stats.revisions, expected_stats.revisions);
        assert_eq!(
            stats.rewritten_index_entries,
            expected_stats.rewritten_index_entries
        );
    }
}

/// A migration that stops after some batches were applied leaves nothing
/// once its transaction is dropped, and a run in a new transaction stores
/// what an uninterrupted one does.
#[test]
fn should_store_the_same_histories_after_a_run_that_stopped_part_way() {
    let (drive, contracts) = legacy_mixed_histories();
    let (reference, _) = {
        let (drive, contracts) = legacy_mixed_histories();
        migrated_elements(&drive, &contracts, 1)
    };
    let mut version = PlatformVersion::latest().clone();
    version.system_limits.max_history_migration_batch_operations = Some(1);
    let last = contracts
        .iter()
        .max_by_key(|contract| contract.id())
        .unwrap();
    let last_type = contract_document_type_path_vec(last.id().as_slice(), "note");
    drive
        .grove
        .insert(
            last_type.as_slice(),
            b"unknown",
            Element::empty_tree(),
            None,
            None,
            &version.drive.grove_version,
        )
        .unwrap()
        .unwrap();
    let root_before = drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .unwrap()
        .unwrap();
    let transaction = drive.grove.start_transaction();
    let stopped = drive.migrate_document_history_storage(&transaction, &version);
    assert!(stopped.is_err());
    assert_ne!(
        drive
            .grove
            .root_hash(Some(&transaction), &version.drive.grove_version)
            .unwrap()
            .unwrap(),
        root_before,
        "the first contract's documents were written before the run stopped"
    );
    drop(transaction);
    assert_eq!(
        drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .unwrap()
            .unwrap(),
        root_before,
        "nothing the stopped run wrote outlives its transaction"
    );
    drive
        .grove
        .delete(
            last_type.as_slice(),
            b"unknown",
            None,
            None,
            &version.drive.grove_version,
        )
        .unwrap()
        .unwrap();
    let (elements, stats) = migrated_elements(&drive, &contracts, 1);
    assert_eq!(stats.migrated_documents, 8);
    assert_eq!(elements, reference);
}
