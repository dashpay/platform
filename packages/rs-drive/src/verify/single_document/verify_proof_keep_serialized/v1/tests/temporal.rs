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

                let mut bare_bound_query = path_query;
                let mut bare_bound = Query::new_with_direction(false);
                bare_bound.insert_range_to_inclusive(..=encode_u64(4000));
                bare_bound_query.query.query.set_subquery(bare_bound);
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
            let mut document_query = temporal.construct_path_query(current).unwrap();
            document_query.query.limit = None;
            // A merged proof keeps the temporal cap on the history walk itself.
            // Execution-proof helpers clear document caps because their reads are untimed.
            document_query
                .query
                .query
                .default_subquery_branch
                .subquery
                .as_mut()
                .unwrap()
                .limit = Some(1);
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
