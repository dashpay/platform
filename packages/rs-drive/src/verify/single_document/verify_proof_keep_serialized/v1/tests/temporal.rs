use super::*;
use crate::query::document_history_drive_query::{
    DocumentHistoryDriveQuery, DocumentHistoryFilter,
};
use crate::util::common::encode::encode_u64;
use dpp::data_contract::DataContractFactory;
use dpp::document::document_factory::DocumentFactory;
use dpp::platform_value::platform_value;
use grovedb::{GroveDb, PathQuery, Query};
use std::collections::BTreeSet;

fn temporal_fixture(initial: &PlatformVersion, summable: bool) -> (Drive, DataContract, Document) {
    let mut schema = platform_value!({
        "type": "object", "documentsKeepHistory": true, "canBeDeleted": false,
        "properties": {"age": {"type": "integer", "minimum": 0, "maximum": 100, "position": 0}},
        "required": ["age"], "additionalProperties": false
    });
    if summable {
        schema.set_value("documentsCountable", true.into()).unwrap();
        schema.set_value("documentsSummable", "age".into()).unwrap();
    }
    let contract = DataContractFactory::new(initial.protocol_version)
        .unwrap()
        .create_with_value_config(
            [7; 32].into(),
            0,
            platform_value!({"person": schema}),
            None,
            None,
        )
        .unwrap()
        .data_contract_owned();
    let drive = setup_drive_with_initial_state_structure(Some(initial));
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, initial)
        .unwrap();
    let document = DocumentFactory::new(initial.protocol_version)
        .unwrap()
        .create_document(
            &contract,
            [7; 32].into(),
            "person".into(),
            platform_value!({"age": 1}),
        )
        .unwrap();
    (drive, contract, document)
}

fn write_revision(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    time: u64,
    first: bool,
    version: &PlatformVersion,
) {
    if first {
        drive
            .add_document_for_contract(
                info(document, contract, "person"),
                false,
                BlockInfo::default_with_time(time),
                true,
                None,
                version,
            )
            .unwrap();
    } else {
        drive
            .update_document_for_contract(
                document,
                contract,
                contract.document_type_for_name("person").unwrap(),
                Some(document.owner_id().to_buffer()),
                BlockInfo::default_with_time(time),
                true,
                None,
                None,
                version,
            )
            .unwrap();
    }
}

#[test]
fn should_match_history_pages_at_every_temporal_boundary_before_and_after_migration() {
    let current = PlatformVersion::latest();
    let old = PlatformVersion::get(14).unwrap();
    let times = [1000, 2000, 3000, 4000, 4000, 4000, 5000, 6000, 7000, 8000];
    let mut grid = BTreeSet::from([0, i64::MAX as u64, u64::MAX]);
    for time in times {
        grid.extend([time - 1, time, time + 1]);
    }
    for migrated in [false, true] {
        for summable in [false, true] {
            let initial = if migrated { old } else { current };
            let (drive, contract, mut document) = temporal_fixture(initial, summable);
            for (index, time) in times.into_iter().enumerate() {
                if migrated && index == 5 {
                    let mut legacy = current.clone();
                    legacy
                        .drive
                        .methods
                        .document
                        .document_history_storage_layout = 0;
                    let before = grid
                        .iter()
                        .map(|time| {
                            let query =
                                person_at_block_time(&contract, document.id().to_buffer(), *time);
                            let found = prove_and_verify_at(&drive, &contract, &query, &legacy);
                            (*time, found.map(|doc| serialized(&doc, &contract)))
                        })
                        .collect::<Vec<_>>();
                    let transaction = drive.grove.start_transaction();
                    drive
                        .migrate_document_history_storage(&transaction, current)
                        .unwrap();
                    drive
                        .grove
                        .commit_transaction(transaction)
                        .unwrap()
                        .unwrap();
                    for (time, expected) in before {
                        let query =
                            person_at_block_time(&contract, document.id().to_buffer(), time);
                        let found = prove_and_verify_at(&drive, &contract, &query, current);
                        assert_eq!(
                            found.map(|doc| serialized(&doc, &contract)),
                            expected,
                            "migration preserves the last write at {time}, summable {summable}"
                        );
                    }
                }
                document.set_revision(Some(index as u64 + 1));
                document.set("age", Value::U64(index as u64 + 1));
                let writer = if migrated && index < 5 { old } else { current };
                write_revision(&drive, &contract, &document, time, index == 0, writer);
            }
            let page_query = DocumentHistoryDriveQuery {
                contract_id: contract.id().to_buffer(),
                document_type_name: "person".into(),
                document_id: document.id().to_buffer(),
                filter: DocumentHistoryFilter::StartAtTime(0),
                limit: Some(10),
            };
            let page = drive
                .fetch_document_history(
                    &page_query,
                    contract.document_type_for_name("person").unwrap(),
                    None,
                    current,
                )
                .unwrap();
            assert_eq!(page.entries.len(), if migrated { 9 } else { 10 });
            for time in &grid {
                let expected = page
                    .entries
                    .iter()
                    .rev()
                    .find(|entry| entry.time_ms <= *time)
                    .map(|entry| serialized(&entry.document, &contract));
                let query = person_at_block_time(&contract, document.id().to_buffer(), *time);
                assert_eq!(direct_read(&drive, &query, current), expected);
                let found = prove_and_verify_at(&drive, &contract, &query, current);
                assert_eq!(
                    found.map(|doc| serialized(&doc, &contract)),
                    expected,
                    "time {time}, migrated {migrated}, summable {summable}"
                );
            }
            let temporal = person_at_block_time(&contract, document.id().to_buffer(), 4000);
            if !migrated && !summable {
                let path_query = temporal.construct_path_query(current).unwrap();
                let proof = drive
                    .grove_get_proved_path_query(&path_query, None, &mut vec![], &current.drive)
                    .unwrap();
                let (_, found) = temporal
                    .verify_proof_keep_serialized(false, &proof, current)
                    .unwrap();
                assert_eq!(found, direct_read(&drive, &temporal, current));
                assert!(found.is_some());

                let mut bare_bound = Query::new_with_direction(false);
                bare_bound.insert_range_to_inclusive(..=encode_u64(4000));
                bare_bound.limit = Some(1);
                let mut selection = Query::new();
                selection.insert_key(temporal.document_id.to_vec());
                selection.set_subquery(bare_bound);
                let mut history_path =
                    crate::drive::document::paths::contract_document_type_path_vec(
                        &temporal.contract_id,
                        &temporal.document_type_name,
                    );
                history_path.push(vec![2]);
                let bare_bound_query = PathQuery::new(
                    history_path,
                    grovedb::SizedQuery::new(selection, None, None),
                );
                let bare_bound_proof = drive
                    .grove_get_proved_path_query(
                        &bare_bound_query,
                        None,
                        &mut vec![],
                        &current.drive,
                    )
                    .unwrap();
                let (_, mut bare_bound_results) = GroveDb::verify_query(
                    &bare_bound_proof,
                    &bare_bound_query,
                    &current.drive.grove_version,
                )
                .unwrap();
                assert_eq!(bare_bound_results.len(), 1);
                let (_, key, element) = bare_bound_results.pop().unwrap();
                assert_eq!(key, [encode_u64(3000), encode_u64(3)].concat());
                let bare_bound_document = Document::from_bytes(
                    &element.unwrap().into_item_bytes().unwrap(),
                    contract.document_type_for_name("person").unwrap(),
                    current,
                )
                .unwrap();
                assert_eq!(bare_bound_document.revision(), Some(3));
                for is_subset in [false, true] {
                    assert!(temporal
                        .verify_proof_keep_serialized(is_subset, &bare_bound_proof, current)
                        .is_err());
                }

                let other = person_at_block_time(&contract, [9; 32], 4000);
                assert_ne!(other.document_id, temporal.document_id);
                for is_subset in [false, true] {
                    assert!(
                        temporal
                            .verify_proof_keep_serialized(
                                is_subset,
                                &proof[..proof.len() - 1],
                                current,
                            )
                            .is_err()
                    );
                    if let Ok((root, found)) =
                        other.verify_proof_keep_serialized(is_subset, &proof, current)
                    {
                        assert_eq!(found, direct_read(&drive, &other, current));
                        assert_eq!(
                            root,
                            drive
                                .grove
                                .root_hash(None, &current.drive.grove_version)
                                .unwrap()
                                .unwrap()
                        );
                    }
                }
            }
            let document_query = temporal.construct_path_query(current).unwrap();
            let identity_query = Drive::identity_balance_query(&[3; 32]);
            let merged = PathQuery::merge(
                vec![&document_query, &identity_query],
                &current.drive.grove_version,
            )
            .unwrap();
            let larger_proof = drive
                .grove_get_proved_path_query(&merged, None, &mut vec![], &current.drive)
                .unwrap();
            let (root, subset) = temporal
                .verify_proof_keep_serialized(true, &larger_proof, current)
                .unwrap();
            assert_eq!(subset, direct_read(&drive, &temporal, current));
            assert_eq!(
                root,
                drive
                    .grove
                    .root_hash(None, &current.drive.grove_version)
                    .unwrap()
                    .unwrap()
            );
            let latest = SingleDocumentDriveQuery {
                block_time_ms: None,
                ..person_at_block_time(&contract, document.id().to_buffer(), u64::MAX)
            };
            assert_eq!(
                direct_read(&drive, &latest, current),
                direct_read(
                    &drive,
                    &person_at_block_time(&contract, document.id().to_buffer(), u64::MAX),
                    current
                )
            );
            assert_eq!(
                prove_and_verify_at(
                    &drive,
                    &contract,
                    &person_at_block_time(&contract, [9; 32], 4000),
                    current
                ),
                None
            );
        }
    }
}

#[test]
fn should_verify_temporal_absence_in_an_empty_history_type() {
    let current = PlatformVersion::latest();
    for summable in [false, true] {
        let (drive, contract, document) = temporal_fixture(current, summable);
        let query = person_at_block_time(&contract, document.id().to_buffer(), 4000);
        assert_eq!(direct_read(&drive, &query, current), None);
        assert_eq!(prove_and_verify(&drive, &contract, &query), None);
    }
}

#[test]
fn should_choose_the_highest_revision_among_writes_at_one_time() {
    let current = PlatformVersion::latest();
    let (drive, contract, mut document) = temporal_fixture(current, false);
    for revision in 1..=8 {
        document.set_revision(Some(revision));
        document.set("age", Value::U64(revision));
        write_revision(&drive, &contract, &document, 1000, revision == 1, current);
        let found = prove_and_verify(
            &drive,
            &contract,
            &person_at_block_time(&contract, document.id().to_buffer(), 1000),
        )
        .unwrap();
        assert_eq!(found.revision(), Some(revision));
    }
}

#[test]
fn should_find_a_document_created_at_zero_at_time_zero() {
    let current = PlatformVersion::latest();
    let (drive, contract, document) = temporal_fixture(current, false);
    write_revision(&drive, &contract, &document, 0, true, current);
    let found = prove_and_verify(
        &drive,
        &contract,
        &person_at_block_time(&contract, document.id().to_buffer(), 0),
    )
    .unwrap();
    assert_eq!(
        serialized(&found, &contract),
        serialized(&document, &contract)
    );
}

#[test]
fn should_hide_a_deleted_document_at_and_after_its_delete_time() {
    let version = PlatformVersion::latest();
    let (drive, mut contract, mut document) = temporal_fixture(version, false);
    let mut schema = contract
        .document_type_for_name("person")
        .unwrap()
        .schema()
        .clone();
    schema.set_value("canBeDeleted", true.into()).unwrap();
    schema.set_value("canBeErased", false.into()).unwrap();
    contract
        .set_document_schema("person", schema, true, &mut vec![], version)
        .unwrap();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    document.set_revision(Some(1));
    write_revision(&drive, &contract, &document, 1000, true, version);
    document.set_revision(Some(2));
    document.set("age", Value::U64(2));
    write_revision(&drive, &contract, &document, 2000, false, version);
    drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "person",
            BlockInfo::default_with_time(2000),
            true,
            None,
            version,
        )
        .unwrap();
    for (time, revision) in [
        (0, None),
        (999, None),
        (1000, Some(1)),
        (1999, Some(1)),
        (2000, None),
        (2001, None),
        (u64::MAX, None),
    ] {
        let query = person_at_block_time(&contract, document.id().to_buffer(), time);
        let found = prove_and_verify_at(&drive, &contract, &query, version);
        assert_eq!(
            found.and_then(|doc| doc.revision()),
            revision,
            "delete takes effect at {time}"
        );
    }
    let mut schema = contract
        .document_type_for_name("person")
        .unwrap()
        .schema()
        .clone();
    schema.set_value("canBeDeleted", false.into()).unwrap();
    contract
        .set_document_schema("person", schema, true, &mut vec![], version)
        .unwrap();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default_with_time(3000),
            true,
            None,
            None,
            version,
        )
        .unwrap();
    let query = person_at_block_time(&contract, document.id().to_buffer(), u64::MAX);
    assert_eq!(
        prove_and_verify_at(&drive, &contract, &query, version),
        None,
        "narrowing deletion permission cannot remove lifecycle evidence from a timed lookup"
    );
}

fn overwrite_retained_bytes(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    bytes: Vec<u8>,
) {
    use crate::drive::document::paths::document_history_path;
    let version = PlatformVersion::latest();
    let path = document_history_path(
        contract.id_ref().as_bytes(),
        "person",
        document.id().as_slice(),
    );
    drive
        .grove
        .insert(
            path.as_slice(),
            &[encode_u64(1000), encode_u64(1)].concat(),
            grovedb::Element::new_item(bytes),
            None,
            None,
            &version.drive.grove_version,
        )
        .value
        .unwrap();
}

#[test]
fn should_reject_a_returned_document_body_that_does_not_match_its_history_key() {
    let version = PlatformVersion::latest();
    for mismatch_id in [true, false] {
        let (drive, contract, mut document) = temporal_fixture(version, false);
        document.set_revision(Some(1));
        write_revision(&drive, &contract, &document, 1000, true, version);
        let query = person_at_block_time(&contract, document.id().to_buffer(), 1000);
        let original = document.clone();
        if mismatch_id {
            document.set_id([9; 32].into());
        } else {
            document.set_revision(Some(2));
        }
        overwrite_retained_bytes(
            &drive,
            &contract,
            &original,
            serialized(&document, &contract),
        );
        let proof = drive
            .grove_get_proved_path_query(
                &query.construct_path_query(version).unwrap(),
                None,
                &mut vec![],
                &version.drive,
            )
            .unwrap();
        for subset in [false, true] {
            assert!(
                query
                    .verify_proof(
                        subset,
                        &proof,
                        contract.document_type_for_name("person").unwrap(),
                        version
                    )
                    .is_err(),
                "the returned document must match its authenticated path and revision"
            );
        }
    }
}

#[test]
fn should_discard_a_deleted_revision_without_decoding_its_body() {
    let version = PlatformVersion::latest();
    let (drive, mut contract, mut document) = temporal_fixture(version, false);
    let mut schema = contract
        .document_type_for_name("person")
        .unwrap()
        .schema()
        .clone();
    schema.set_value("canBeDeleted", true.into()).unwrap();
    contract
        .set_document_schema("person", schema, true, &mut vec![], version)
        .unwrap();
    document.set_revision(Some(1));
    write_revision(&drive, &contract, &document, 1000, true, version);
    drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "person",
            BlockInfo::default_with_time(2000),
            true,
            None,
            version,
        )
        .unwrap();
    overwrite_retained_bytes(&drive, &contract, &document, vec![0xff]);
    let query = person_at_block_time(&contract, document.id().to_buffer(), 2000);
    let proof = drive
        .grove_get_proved_path_query(
            &query.construct_path_query(version).unwrap(),
            None,
            &mut vec![],
            &version.drive,
        )
        .unwrap();
    for subset in [false, true] {
        assert_eq!(
            query
                .verify_proof_keep_serialized(subset, &proof, version)
                .unwrap()
                .1,
            None
        );
        assert_eq!(
            query
                .verify_proof(
                    subset,
                    &proof,
                    contract.document_type_for_name("person").unwrap(),
                    version
                )
                .unwrap()
                .1,
            None
        );
    }
}

#[test]
fn should_prove_absence_for_every_time_once_erasing_starts_and_after_it_finishes() {
    let mut version = PlatformVersion::latest().clone();
    version
        .system_limits
        .max_document_revisions_erased_per_transition = Some(2);
    let (drive, mut contract, mut document) = temporal_fixture(&version, false);
    let mut schema = contract
        .document_type_for_name("person")
        .unwrap()
        .schema()
        .clone();
    schema.set_value("canBeDeleted", true.into()).unwrap();
    schema.set_value("canBeErased", true.into()).unwrap();
    contract
        .set_document_schema("person", schema, true, &mut vec![], &version)
        .unwrap();
    for revision in 1..=3 {
        document.set_revision(Some(revision));
        document.set("age", Value::U64(revision));
        write_revision(
            &drive,
            &contract,
            &document,
            revision * 1000,
            revision == 1,
            &version,
        );
    }
    drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "person",
            BlockInfo::default_with_time(3000),
            true,
            None,
            &version,
        )
        .unwrap();
    for terminal in [false, true] {
        let operations = drive
            .erase_document_for_contract_operations(
                document.id(),
                &contract,
                contract.document_type_for_name("person").unwrap(),
                &BlockInfo::default_with_time(4000),
                &mut None,
                None,
                &version,
            )
            .unwrap();
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &version.drive,
            )
            .unwrap();
        let page = drive
            .fetch_document_history(
                &DocumentHistoryDriveQuery {
                    contract_id: contract.id().to_buffer(),
                    document_type_name: "person".into(),
                    document_id: document.id().to_buffer(),
                    filter: DocumentHistoryFilter::StartAtTime(0),
                    limit: None,
                },
                contract.document_type_for_name("person").unwrap(),
                None,
                &version,
            )
            .unwrap();
        assert_eq!(page.entries.len(), if terminal { 0 } else { 1 });
        for time in [0, 999, 1000, 2000, 2999, 3000, 4000, u64::MAX] {
            let query = person_at_block_time(&contract, document.id().to_buffer(), time);
            assert_eq!(
                prove_and_verify_at(&drive, &contract, &query, &version),
                None,
                "erase hides every time {time}, terminal {terminal}"
            );
        }
        let latest = SingleDocumentDriveQuery {
            block_time_ms: None,
            ..person_at_block_time(&contract, document.id().to_buffer(), u64::MAX)
        };
        assert_eq!(
            prove_and_verify_at(&drive, &contract, &latest, &version),
            None
        );
    }
}

#[test]
fn should_not_decode_a_record_only_result_as_a_document_and_should_ignore_other_ids() {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use dpp::document::lifecycle::DocumentLifecycleRecord;
    use dpp::serialization::PlatformSerializable;
    let version = PlatformVersion::latest();
    let (drive, contract, mut document) = temporal_fixture(version, false);
    let type_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "person");
    drive
        .grove
        .insert(
            type_path.as_slice(),
            &[1],
            grovedb::Element::empty_tree(),
            None,
            None,
            &version.drive.grove_version,
        )
        .value
        .unwrap();
    let mut record_path = type_path;
    record_path.push(vec![1]);
    let absent_id = [9; 32];
    let only_record = person_at_block_time(&contract, absent_id, 0);
    // The existing empty lifecycle container proves absence for an unrelated id.
    assert_eq!(
        prove_and_verify_at(&drive, &contract, &only_record, version),
        None
    );
    drive
        .grove
        .insert(
            record_path.as_slice(),
            &absent_id,
            grovedb::Element::new_item(
                DocumentLifecycleRecord::deleted_at(0, 1, 1)
                    .serialize_to_bytes()
                    .unwrap(),
            ),
            None,
            None,
            &version.drive.grove_version,
        )
        .value
        .unwrap();
    for time in [0, u64::MAX] {
        let query = person_at_block_time(&contract, absent_id, time);
        let proof = drive
            .grove_get_proved_path_query(
                &query.construct_path_query(version).unwrap(),
                None,
                &mut vec![],
                &version.drive,
            )
            .unwrap();
        for subset in [false, true] {
            assert_eq!(
                query
                    .verify_proof_keep_serialized(subset, &proof, version)
                    .unwrap()
                    .1,
                None
            );
            assert_eq!(
                query
                    .verify_proof(
                        subset,
                        &proof,
                        contract.document_type_for_name("person").unwrap(),
                        version
                    )
                    .unwrap()
                    .1,
                None
            );
        }
    }
    document.set_revision(Some(1));
    write_revision(&drive, &contract, &document, 1000, true, version);
    let found = prove_and_verify_at(
        &drive,
        &contract,
        &person_at_block_time(&contract, document.id().to_buffer(), 1000),
        version,
    )
    .unwrap();
    assert_eq!(
        serialized(&found, &contract),
        serialized(&document, &contract)
    );
}

#[test]
fn should_reject_a_history_only_proof_that_omits_an_existing_deletion_record() {
    let version = PlatformVersion::latest();
    let (drive, mut contract, mut document) = temporal_fixture(version, false);
    let mut schema = contract
        .document_type_for_name("person")
        .unwrap()
        .schema()
        .clone();
    schema.set_value("canBeDeleted", true.into()).unwrap();
    contract
        .set_document_schema("person", schema, true, &mut vec![], version)
        .unwrap();
    document.set_revision(Some(1));
    write_revision(&drive, &contract, &document, 1000, true, version);
    drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "person",
            BlockInfo::default_with_time(2000),
            true,
            None,
            version,
        )
        .unwrap();
    let query = person_at_block_time(&contract, document.id().to_buffer(), 2000);
    let mut history_path = crate::drive::document::paths::contract_document_type_path_vec(
        &query.contract_id,
        &query.document_type_name,
    );
    history_path.push(vec![2]);
    let mut selection = Query::new();
    selection.insert_key(query.document_id.to_vec());
    let mut revisions = Query::new_with_direction(false);
    revisions.insert_range_inclusive(encode_u64(0)..=[encode_u64(2000), vec![0xff; 8]].concat());
    revisions.limit = Some(1);
    selection.set_subquery(revisions);
    let old_query = PathQuery::new(
        history_path,
        grovedb::SizedQuery::new(selection, None, None),
    );
    let old_proof = drive
        .grove_get_proved_path_query(&old_query, None, &mut vec![], &version.drive)
        .unwrap();
    let (_, rows) =
        GroveDb::verify_query(&old_proof, &old_query, &version.drive.grove_version).unwrap();
    assert_eq!(
        rows.len(),
        1,
        "the history-only proof authenticates the stale retained revision"
    );
    for subset in [false, true] {
        assert!(
            query
                .verify_proof_keep_serialized(subset, &old_proof, version)
                .is_err(),
            "the lifecycle-aware query must also authenticate the existing record"
        );
    }
}

#[test]
fn should_compare_the_unclamped_delete_time_and_allow_deletion_at_zero() {
    let version = PlatformVersion::latest();
    for (created_at, deleted_at) in [(0, 0), (1000, u64::MAX)] {
        let (drive, mut contract, mut document) = temporal_fixture(version, false);
        let mut schema = contract
            .document_type_for_name("person")
            .unwrap()
            .schema()
            .clone();
        schema.set_value("canBeDeleted", true.into()).unwrap();
        contract
            .set_document_schema("person", schema, true, &mut vec![], version)
            .unwrap();
        document.set_revision(Some(1));
        write_revision(&drive, &contract, &document, created_at, true, version);
        drive
            .delete_document_for_contract(
                document.id(),
                &contract,
                "person",
                BlockInfo::default_with_time(deleted_at),
                true,
                None,
                version,
            )
            .unwrap();
        for time in [0, 1000, i64::MAX as u64, u64::MAX - 1, u64::MAX] {
            let found = prove_and_verify_at(
                &drive,
                &contract,
                &person_at_block_time(&contract, document.id().to_buffer(), time),
                version,
            );
            assert_eq!(
                found.is_some(),
                time >= created_at && time < deleted_at,
                "created {created_at}, deleted {deleted_at}, requested {time}"
            );
        }
    }
}
