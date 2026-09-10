//! The storage side of the keep-history document lifecycle: what a delete
//! leaves behind, what an erase removes, and what a chunked erasure looks like
//! from one transition to the next.

use super::fetch::DocumentLifecycleState;
use super::DocumentLifecycleRecord;
use crate::drive::document::history::{
    DocumentHistoryQueryV1, DocumentHistorySelector, DocumentHistoryState,
};
use crate::drive::document::paths::{document_history_path, document_lifecycle_path};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::{
    DriveDocumentQuery, SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus,
};
use crate::util::grove_operations::BatchDeleteApplyType::StatefulBatchDelete;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::identifier::Identifier;
use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
use dpp::version::PlatformVersion;
use grovedb::{MaybeTree, TransactionArg, TreeType};
use std::borrow::Cow;
use std::collections::HashMap;

const FAMILY_HISTORY_CONTRACT: &str =
    "tests/supporting_files/contract/family/family-contract-with-history.json";
const PERSON: &str = "tests/supporting_files/contract/family/person0.json";

fn latest() -> &'static PlatformVersion {
    PlatformVersion::latest()
}

fn chunk_size() -> u64 {
    latest()
        .system_limits
        .max_document_revisions_erased_per_transition
        .expect("protocol 14 bounds the erase chunk") as u64
}

/// Applies the family contract and writes `revisions` revisions of one person,
/// each at its own block time so every revision key is distinct.
fn setup_history(revisions: u64, owner: [u8; 32]) -> (Drive, DataContract, Identifier) {
    let version = latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = json_document_to_contract(FAMILY_HISTORY_CONTRACT, false, version)
        .expect("expected the family history contract");
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .expect("expected to apply the contract");
    let document_type = contract
        .document_type_for_name("person")
        .expect("expected the person type");
    let mut document =
        json_document_to_document(PERSON, Some(owner.into()), document_type, version)
            .expect("expected a person document");
    let flags = Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(owner))));
    for revision in 1..=revisions {
        document.set_revision(Some(revision));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((&document, flags.clone())),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                revision > 1,
                BlockInfo::default_with_time(1_000 + revision),
                true,
                None,
                version,
                None,
            )
            .expect("expected to write a revision");
    }
    let id = document.id();
    (drive, contract, id)
}

fn document_type_of(contract: &DataContract) -> DocumentTypeRef<'_> {
    contract
        .document_type_for_name("person")
        .expect("expected the person type")
}

fn lifecycle_of(drive: &Drive, contract: &DataContract, id: Identifier) -> DocumentLifecycleState {
    drive
        .fetch_document_lifecycle(
            contract,
            document_type_of(contract),
            id,
            None,
            None,
            latest(),
        )
        .expect("expected to read the lifecycle")
        .0
}

fn record_of(
    drive: &Drive,
    contract: &DataContract,
    id: Identifier,
) -> Option<DocumentLifecycleRecord> {
    let path = document_lifecycle_path(contract.id_ref().as_bytes(), "person");
    drive
        .grove_get_raw_optional_item(
            path.as_slice().into(),
            id.as_slice(),
            crate::util::grove_operations::DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &latest().drive,
        )
        .expect("expected to read the lifecycle tree")
        .map(|bytes| DocumentLifecycleRecord::deserialize(&bytes).expect("expected a valid record"))
}

fn visible_document_count(drive: &Drive, contract: &DataContract) -> usize {
    let query = DriveDocumentQuery::all_items_query(contract, document_type_of(contract), None);
    query
        .execute_raw_results_no_proof(drive, None, None, latest())
        .expect("expected to query documents")
        .0
        .len()
}

fn history_metadata(
    drive: &Drive,
    contract: &DataContract,
    id: Identifier,
) -> crate::drive::document::history::DocumentHistoryLifecycle {
    let query = DocumentHistoryQueryV1 {
        contract_id: contract.id().to_buffer(),
        document_type_name: "person".into(),
        document_id: id.to_buffer(),
        selector: DocumentHistorySelector::StartAtTime(0),
        limit: Some(10),
    };
    let (page, proof) = drive
        .prove_document_history_v1(&query, document_type_of(contract), None, latest())
        .expect("expected to prove history");
    let (_, verified) =
        Drive::verify_document_history_v1(&query, &proof, document_type_of(contract), latest())
            .expect("expected the proof to verify");
    assert_eq!(
        verified, page,
        "the proved history must match the read history"
    );
    verified.lifecycle
}

fn delete(
    drive: &Drive,
    contract: &DataContract,
    id: Identifier,
    deleter: Identifier,
    time_ms: u64,
) -> dpp::fee::fee_result::FeeResult {
    let mut operations = vec![];
    let block_info = BlockInfo::default_with_time(time_ms);
    let batch = drive
        .delete_document_for_contract_operations(
            id,
            contract,
            document_type_of(contract),
            &block_info,
            Some(deleter),
            None,
            &mut None,
            None,
            latest(),
        )
        .expect("expected to build the delete operations");
    drive
        .apply_batch_low_level_drive_operations(None, None, batch, &mut operations, &latest().drive)
        .expect("expected to apply the delete");
    Drive::calculate_fee(
        None,
        Some(operations),
        &block_info.epoch,
        drive.config.epochs_per_era,
        latest(),
        None,
    )
    .expect("expected a fee")
}

fn erase(
    drive: &Drive,
    contract: &DataContract,
    id: Identifier,
    time_ms: u64,
) -> dpp::fee::fee_result::FeeResult {
    let mut operations = vec![];
    let block_info = BlockInfo::default_with_time(time_ms);
    let batch = drive
        .erase_document_for_contract_operations(
            id,
            contract,
            document_type_of(contract),
            &block_info,
            &mut None,
            None,
            latest(),
        )
        .expect("expected to build the erase operations");
    drive
        .apply_batch_low_level_drive_operations(None, None, batch, &mut operations, &latest().drive)
        .expect("expected to apply the erase");
    Drive::calculate_fee(
        None,
        Some(operations),
        &block_info.epoch,
        drive.config.epochs_per_era,
        latest(),
        None,
    )
    .expect("expected a fee")
}

/// The admission estimate of an erase, which knows nothing about the document
/// it will act on.
fn estimated_erase_fee(
    drive: &Drive,
    contract: &DataContract,
    id: Identifier,
) -> dpp::fee::fee_result::FeeResult {
    let mut operations = vec![];
    let mut layers = Some(HashMap::new());
    let block_info = BlockInfo::default_with_time(9_000);
    let batch = drive
        .erase_document_for_contract_operations(
            id,
            contract,
            document_type_of(contract),
            &block_info,
            &mut layers,
            None,
            latest(),
        )
        .expect("expected to build the estimated erase operations");
    drive
        .apply_batch_low_level_drive_operations(
            layers,
            None,
            batch,
            &mut operations,
            &latest().drive,
        )
        .expect("expected to price the erase");
    Drive::calculate_fee(
        None,
        Some(operations),
        &block_info.epoch,
        drive.config.epochs_per_era,
        latest(),
        None,
    )
    .expect("expected a fee")
}

#[test]
fn should_hide_a_deleted_document_while_keeping_every_revision_readable() {
    let owner = [7u8; 32];
    let (drive, contract, id) = setup_history(3, owner);
    assert_eq!(visible_document_count(&drive, &contract), 1);

    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    assert_eq!(
        visible_document_count(&drive, &contract),
        0,
        "a deleted document must be absent from every ordinary read"
    );
    let record = record_of(&drive, &contract, id).expect("a delete writes a lifecycle record");
    assert_eq!(record.deleted_at_ms(), 5_000);
    assert!(!record.is_erasing());

    let lifecycle = history_metadata(&drive, &contract, id);
    assert_eq!(lifecycle.state, DocumentHistoryState::Deleted);
    assert_eq!(
        lifecycle.remaining_revisions, 3,
        "a delete removes no revision"
    );
    assert_eq!(lifecycle.times.deleted_at_ms, 5_000);
    assert_eq!(lifecycle.times.erasing_started_at_ms, 0);

    assert!(matches!(
        lifecycle_of(&drive, &contract, id),
        DocumentLifecycleState::Deleted(_)
    ));
}

#[test]
fn should_erase_a_history_shorter_than_a_chunk_in_one_transition() {
    let owner = [8u8; 32];
    let (drive, contract, id) = setup_history(1, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    erase(&drive, &contract, id, 6_000);

    assert!(
        record_of(&drive, &contract, id).is_none(),
        "the terminal chunk removes the record"
    );
    assert!(matches!(
        lifecycle_of(&drive, &contract, id),
        DocumentLifecycleState::Absent
    ));
    let lifecycle = history_metadata(&drive, &contract, id);
    assert_eq!(lifecycle.state, DocumentHistoryState::Absent);
    assert_eq!(lifecycle.remaining_revisions, 0);
}

#[test]
fn should_erase_a_history_of_exactly_one_chunk_in_one_transition() {
    let chunk = chunk_size();
    let owner = [9u8; 32];
    let (drive, contract, id) = setup_history(chunk, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    erase(&drive, &contract, id, 6_000);

    assert!(record_of(&drive, &contract, id).is_none());
    assert_eq!(
        history_metadata(&drive, &contract, id).state,
        DocumentHistoryState::Absent
    );
}

#[test]
fn should_commit_a_longer_history_to_erasure_and_finish_it_in_the_next_chunk() {
    let chunk = chunk_size();
    let owner = [10u8; 32];
    let (drive, contract, id) = setup_history(chunk + 1, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    erase(&drive, &contract, id, 6_000);

    let record = record_of(&drive, &contract, id).expect("a partial chunk keeps the record");
    assert!(record.is_erasing(), "the first chunk commits the erasure");
    assert_eq!(record.deleted_at_ms(), 5_000, "the deletion time survives");
    assert_eq!(record.erasing_started_at_ms(), 6_000);
    assert_eq!(
        record.erasing_from_revision(),
        chunk + 1,
        "the erasure started from the newest revision"
    );
    assert_eq!(record.erasing_from_time_ms(), 1_000 + chunk + 1);

    let lifecycle = history_metadata(&drive, &contract, id);
    assert_eq!(lifecycle.state, DocumentHistoryState::Erasing);
    assert_eq!(
        lifecycle.remaining_revisions, 1,
        "the oldest revision is what a partial erasure leaves behind"
    );
    assert_eq!(lifecycle.times.erasing_from_revision, chunk + 1);
    assert!(matches!(
        lifecycle_of(&drive, &contract, id),
        DocumentLifecycleState::Erasing
    ));

    erase(&drive, &contract, id, 7_000);

    assert!(record_of(&drive, &contract, id).is_none());
    assert_eq!(
        history_metadata(&drive, &contract, id).state,
        DocumentHistoryState::Absent
    );
}

#[test]
fn should_leave_the_record_untouched_across_a_continuation() {
    let chunk = chunk_size();
    let owner = [11u8; 32];
    let (drive, contract, id) = setup_history(chunk * 2 + 1, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    erase(&drive, &contract, id, 6_000);
    let after_start = record_of(&drive, &contract, id).expect("the erasure is committed");

    erase(&drive, &contract, id, 7_000);
    let after_continuation =
        record_of(&drive, &contract, id).expect("the erasure is not finished yet");

    assert_eq!(
        after_start, after_continuation,
        "a continuation carries no authorization of its own and writes nothing to the record"
    );
    assert_eq!(
        history_metadata(&drive, &contract, id).remaining_revisions,
        1
    );

    erase(&drive, &contract, id, 8_000);
    assert_eq!(
        history_metadata(&drive, &contract, id).state,
        DocumentHistoryState::Absent
    );
}

#[test]
fn should_refuse_to_create_a_document_whose_revisions_are_still_retained() {
    let owner = [12u8; 32];
    let (drive, contract, id) = setup_history(2, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    let version = latest();
    let document_type = document_type_of(&contract);
    let mut document =
        json_document_to_document(PERSON, Some(owner.into()), document_type, version).unwrap();
    document.set_revision(Some(1));

    let error = drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            // The contested award path reaches the writer this way: nothing
            // above it has checked whether the id is free.
            false,
            BlockInfo::default_with_time(6_000),
            true,
            None,
            version,
            None,
        )
        .expect_err("a reserved id must not be creatable");
    assert!(matches!(
        error,
        Error::Drive(DriveError::CorruptedDocumentAlreadyExists(_))
    ));

    // Once the revisions are gone the id is free again.
    erase(&drive, &contract, id, 7_000);
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
            false,
            BlockInfo::default_with_time(8_000),
            true,
            None,
            version,
            None,
        )
        .expect("an erased id is reusable");
    assert_eq!(visible_document_count(&drive, &contract), 1);
}

#[test]
fn should_refuse_to_drop_a_history_subtree_while_a_revision_survives() {
    let owner = [13u8; 32];
    let (drive, contract, id) = setup_history(2, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    // The removal of the subtree on its own, without the revision deletes an
    // erase puts in the same batch. GroveDB has nothing to conclude emptiness
    // from and must refuse rather than orphan the revisions' storage.
    let mut history_root = document_history_path(contract.id_ref().as_bytes(), "person", &[]);
    history_root.pop();
    let mut operations = vec![];
    let batch_result = drive.batch_delete(
        history_root.as_slice().into(),
        id.as_slice(),
        StatefulBatchDelete {
            is_known_to_be_subtree_with_sum: Some(MaybeTree::Tree(TreeType::ProvableCountTree)),
        },
        None,
        &mut operations,
        &latest().drive,
    );
    assert!(
        batch_result.is_err(),
        "a populated history subtree must not be removable"
    );
}

#[test]
fn should_refund_each_removed_revision_to_the_identity_that_wrote_it() {
    let owner = [14u8; 32];
    let (drive, contract, id) = setup_history(3, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    let fee = erase(&drive, &contract, id, 6_000);

    let refunded: u64 = fee
        .fee_refunds
        .0
        .get(&owner)
        .map(|per_epoch| per_epoch.values().sum())
        .unwrap_or_default();
    assert!(
        refunded > 0,
        "the revisions' writer must be credited for the bytes the erase removed"
    );
}

#[test]
fn should_price_an_erase_for_a_full_chunk_whatever_the_document_retains() {
    let chunk = chunk_size();
    for revisions in [1, chunk, chunk + 1] {
        let owner = [15u8; 32];
        let (drive, contract, id) = setup_history(revisions, owner);
        delete(&drive, &contract, id, Identifier::new(owner), 5_000);

        let estimated = estimated_erase_fee(&drive, &contract, id);
        let actual = erase(&drive, &contract, id, 6_000);

        assert!(
            estimated.processing_fee >= actual.processing_fee,
            "the admission estimate under-charged a {revisions}-revision erase: {} < {}",
            estimated.processing_fee,
            actual.processing_fee
        );
    }
}

/// The history a delete leaves behind is still fully readable, page by page and
/// with a proof, including the page that runs past its end.
#[test]
fn should_page_and_prove_the_history_of_a_deleted_document() {
    let owner = [16u8; 32];
    let (drive, contract, id) = setup_history(12, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    let mut selector = DocumentHistorySelector::StartAtTime(0);
    let mut seen = vec![];
    loop {
        let query = DocumentHistoryQueryV1 {
            contract_id: contract.id().to_buffer(),
            document_type_name: "person".into(),
            document_id: id.to_buffer(),
            selector,
            limit: Some(10),
        };
        let (page, proof) = drive
            .prove_document_history_v1(&query, document_type_of(&contract), None, latest())
            .expect("expected to prove a page");
        let (_, verified) = Drive::verify_document_history_v1(
            &query,
            &proof,
            document_type_of(&contract),
            latest(),
        )
        .expect("expected the page proof to verify");
        assert_eq!(verified.lifecycle.state, DocumentHistoryState::Deleted);
        assert_eq!(verified.lifecycle.remaining_revisions, 12);
        assert_eq!(verified.lifecycle.times.deleted_at_ms, 5_000);
        seen.extend(page.entries.iter().map(|entry| entry.revision));
        let Some(last) = page.entries.last() else {
            break;
        };
        selector = DocumentHistorySelector::StartAfter {
            time_ms: last.time_ms,
            revision: last.revision,
        };
    }
    assert_eq!(seen, (1..=12).collect::<Vec<_>>());
}

/// Every document type parsed by an earlier grammar has no lifecycle tree, so
/// the read has to treat a missing tree the same as a missing key.
#[test]
fn should_report_a_document_with_no_lifecycle_tree_as_absent() {
    let owner = [17u8; 32];
    let (drive, contract, _) = setup_history(1, owner);
    let unknown = Identifier::new([99u8; 32]);
    assert!(matches!(
        lifecycle_of(&drive, &contract, unknown),
        DocumentLifecycleState::Absent
    ));
}

#[test]
fn should_reject_an_erase_of_a_document_that_retains_nothing() {
    let owner = [18u8; 32];
    let (drive, contract, id) = setup_history(1, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);
    erase(&drive, &contract, id, 6_000);

    let error = drive
        .erase_document_for_contract_operations(
            id,
            &contract,
            document_type_of(&contract),
            &BlockInfo::default_with_time(7_000),
            &mut None,
            None as TransactionArg,
            latest(),
        )
        .expect_err("there is nothing left to erase");
    assert!(matches!(
        error,
        Error::Drive(DriveError::CorruptedDriveState(_))
    ));
}

/// The two record shapes must occupy the same number of bytes: an erase start
/// overwrites the record in place, and a different size would re-price its
/// storage and drop the deleter's flags.
#[test]
fn should_keep_the_deleter_as_the_records_beneficiary_across_an_erase_start() {
    let chunk = chunk_size();
    let owner = [19u8; 32];
    let deleter = Identifier::new([20u8; 32]);
    let (drive, contract, id) = setup_history(chunk + 1, owner);
    delete(&drive, &contract, id, deleter, 5_000);

    let path = document_lifecycle_path(contract.id_ref().as_bytes(), "person");
    let flags_before = drive
        .grove
        .get_raw(
            path.as_slice().into(),
            id.as_slice(),
            None,
            &latest().drive.grove_version,
        )
        .unwrap()
        .expect("the record exists");

    erase(&drive, &contract, id, 6_000);

    let flags_after = drive
        .grove
        .get_raw(
            path.as_slice().into(),
            id.as_slice(),
            None,
            &latest().drive.grove_version,
        )
        .unwrap()
        .expect("the record survives a partial erasure");
    let (grovedb::Element::Item(before, before_flags), grovedb::Element::Item(after, after_flags)) =
        (flags_before, flags_after)
    else {
        panic!("a lifecycle record is an item");
    };
    assert_eq!(
        before.len(),
        after.len(),
        "the overwrite must be the same size"
    );
    assert_eq!(
        before_flags, after_flags,
        "the deleter must stay the beneficiary of the record's bytes"
    );
}

/// A revision written by one identity and a revision written by another are
/// refunded separately, so an erasure credits whoever paid for each byte.
#[test]
fn should_refund_distinct_writers_separately() {
    let first = [21u8; 32];
    let second = [22u8; 32];
    let version = latest();
    let (drive, contract, id) = setup_history(1, first);
    let document_type = document_type_of(&contract);
    let mut document =
        json_document_to_document(PERSON, Some(first.into()), document_type, version).unwrap();
    document.set_revision(Some(2));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((
                        &document,
                        Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(second)))),
                    )),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            true,
            BlockInfo::default_with_time(2_000),
            true,
            None,
            version,
            None,
        )
        .expect("expected a second writer's revision");

    delete(&drive, &contract, id, Identifier::new(first), 5_000);
    let fee = erase(&drive, &contract, id, 6_000);

    assert!(
        fee.fee_refunds.0.contains_key(&first) && fee.fee_refunds.0.contains_key(&second),
        "each writer must be credited for the revision they paid for; got {:?}",
        fee.fee_refunds.0.keys().collect::<Vec<_>>()
    );
}

#[test]
fn should_read_the_newest_retained_revision_of_a_deleted_document() {
    let owner = [23u8; 32];
    let (drive, contract, id) = setup_history(4, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    let DocumentLifecycleState::Deleted(newest) = lifecycle_of(&drive, &contract, id) else {
        panic!("expected a deleted document");
    };
    assert_eq!(
        newest.revision(),
        Some(4),
        "an erase start is authorized against the newest revision that survives"
    );
    assert_eq!(newest.owner_id().to_buffer(), owner);
}

/// A `Document` is only ever written under a keep-history type through the
/// history tree, so a deleted document's revisions must not be reachable by id.
#[test]
fn should_not_resolve_a_deleted_document_by_id() {
    let owner = [24u8; 32];
    let (drive, contract, id) = setup_history(2, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    let query = SingleDocumentDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "person".into(),
        document_type_keeps_history: true,
        document_id: id.to_buffer(),
        block_time_ms: None,
        contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
    };
    let path_query = query
        .construct_path_query(latest())
        .expect("expected a by-id path query");
    let (results, _) = drive
        .grove_get_raw_path_query(
            &path_query,
            None,
            grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &latest().drive,
        )
        .expect("expected the by-id read to run");
    assert!(
        results.to_key_elements().is_empty(),
        "a deleted document is not fetchable by id"
    );
}

/// The counted and summed entry of a keep-history type is its current pointer,
/// so removing it decrements both aggregates by construction and a deleted
/// document stops contributing without any bookkeeping of its own.
#[test]
fn should_decrement_the_count_and_the_sum_when_a_keep_history_document_is_deleted() {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_factory::DocumentFactory;
    use dpp::platform_value::platform_value;
    use grovedb::Element;

    let version = latest();
    let schema = platform_value!({
        "type": "object",
        "documentsKeepHistory": true,
        "canBeDeleted": true,
        "documentsCountable": true,
        "documentsSummable": "amount",
        "properties": {
            "amount": {"type": "integer", "minimum": 0, "maximum": 4294967295i64, "position": 0},
        },
        "required": ["amount"],
        "additionalProperties": false,
        "indices": [{"name": "amount", "properties": [{"amount": "asc"}]}],
    });
    let contract = DataContractFactory::new(version.protocol_version)
        .expect("expected a contract factory")
        .create_with_value_config(
            [7; 32].into(),
            0,
            platform_value!({ "tip": schema }),
            None,
            None,
        )
        .expect("a countable summable keep-history type must parse")
        .data_contract_owned();

    let drive = setup_drive_with_initial_state_structure(None);
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .expect("expected to apply the contract");
    let document_type = contract
        .document_type_for_name("tip")
        .expect("expected the tip type");
    assert!(document_type.documents_countable());

    let mut document = DocumentFactory::new(version.protocol_version)
        .expect("expected a document factory")
        .create_document(
            &contract,
            [7; 32].into(),
            "tip".into(),
            platform_value!({"amount": 5}),
        )
        .expect("expected a tip");
    document.set_id([9; 32].into());
    for revision in 1..=3u64 {
        document.set_revision(Some(revision));
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
                revision > 1,
                BlockInfo::default_with_time(1_000 + revision),
                true,
                None,
                version,
                None,
            )
            .expect("expected to write a revision");
    }

    let type_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "tip");
    let aggregates = || {
        let element = drive
            .grove
            .get_raw(
                type_path.as_slice().into(),
                &[0],
                None,
                &latest().drive.grove_version,
            )
            .value
            .expect("expected the primary-key tree");
        let Element::CountSumTree(_, count, sum, _) = element else {
            panic!("a countable summable type stores its documents in a count-sum tree");
        };
        (count, sum)
    };
    assert_eq!(
        aggregates(),
        (1, 5),
        "one live document contributing its amount, whatever its history holds"
    );

    let batch = drive
        .delete_document_for_contract_operations(
            document.id(),
            &contract,
            document_type,
            &BlockInfo::default_with_time(5_000),
            Some(Identifier::new([7; 32])),
            None,
            &mut None,
            None,
            version,
        )
        .expect("expected to delete");
    drive
        .apply_batch_low_level_drive_operations(None, None, batch, &mut vec![], &version.drive)
        .expect("expected to apply the delete");

    assert_eq!(
        aggregates(),
        (0, 0),
        "a deleted document contributes to neither aggregate, though its revisions remain"
    );

    // Its index entries are gone too, so the value it held is free for another
    // document to take.
    let mut index_path = type_path.clone();
    index_path.push(b"amount".to_vec());
    let mut query = grovedb::Query::new();
    query.insert_all();
    let (results, _) = drive
        .grove_get_raw_path_query(
            &grovedb::PathQuery::new(index_path, grovedb::SizedQuery::new(query, Some(10), None)),
            None,
            grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &version.drive,
        )
        .expect("expected to read the index");
    assert!(
        results.to_key_elements().is_empty(),
        "a delete removes every index reference that led to the document"
    );
}

/// The per-type container is shared by every document of the type and outlives
/// any one of them, so it belongs to nobody: an erase removes records and
/// per-document history trees, never this. The record inside it is the byte an
/// erase does refund, and that one names the deleter.
#[test]
fn should_leave_the_lifecycle_container_unflagged_while_the_record_names_the_deleter() {
    use crate::drive::document::paths::contract_document_type_path_vec;

    let owner = [25u8; 32];
    let deleter = Identifier::new([26u8; 32]);
    let (drive, contract, id) = setup_history(2, owner);
    delete(&drive, &contract, id, deleter, 5_000);

    let mut type_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "person");
    let container = drive
        .grove
        .get_raw(
            type_path.as_slice().into(),
            &[crate::drive::document::paths::DOCUMENT_LIFECYCLE_TREE_KEY],
            None,
            &latest().drive.grove_version,
        )
        .value
        .expect("the first delete creates the container");
    assert_eq!(
        container.get_flags(),
        &None,
        "the shared container must belong to nobody"
    );

    type_path.push(vec![
        crate::drive::document::paths::DOCUMENT_LIFECYCLE_TREE_KEY,
    ]);
    let record = drive
        .grove
        .get_raw(
            type_path.as_slice().into(),
            id.as_slice(),
            None,
            &latest().drive.grove_version,
        )
        .value
        .expect("the record exists");
    let flags = StorageFlags::map_cow_some_element_flags_ref(record.get_flags())
        .expect("the record's flags must decode")
        .expect("the record names its deleter");
    assert_eq!(
        flags.owner_id(),
        Some(&deleter.to_buffer()),
        "the record is the byte an erase refunds, and it belongs to the deleter"
    );
}

/// Only the first delete of a type pays for the container. A second one finds
/// it already there and pays for its record alone, so the storage difference
/// between the two is exactly the container.
#[test]
fn should_charge_the_container_to_the_first_delete_of_a_type_only() {
    let owner = [27u8; 32];
    let version = latest();
    let (drive, contract, first_id) = setup_history(1, owner);
    let document_type = document_type_of(&contract);

    // A second document of the same type, so the second delete finds the
    // container already in place.
    let mut second =
        json_document_to_document(PERSON, Some(owner.into()), document_type, version).unwrap();
    second.set_id(Identifier::new([28u8; 32]));
    second.set_revision(Some(1));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((
                        &second,
                        Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(owner)))),
                    )),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default_with_time(2_000),
            true,
            None,
            version,
            None,
        )
        .expect("expected a second document");

    let deleter = Identifier::new([29u8; 32]);
    let first = delete(&drive, &contract, first_id, deleter, 5_000);
    let later = delete(&drive, &contract, second.id(), deleter, 6_000);

    assert!(
        first.storage_fee > later.storage_fee,
        "the first delete pays for the container as well as its record: {} vs {}",
        first.storage_fee,
        later.storage_fee
    );
}

/// The declared feature slot has to actually route: a table that selects a
/// version this code does not implement must fail loudly rather than silently
/// keep using the old estimate.
#[test]
fn should_reject_an_unsupported_erase_estimation_version() {
    use std::collections::HashMap;

    let owner = [30u8; 32];
    let (_drive, contract, id) = setup_history(1, owner);
    let mut version = latest().clone();
    assert_eq!(
        version
            .drive
            .methods
            .document
            .delete
            .add_estimation_costs_for_erase_document,
        0,
        "protocol 14 selects the only implementation there is"
    );
    version
        .drive
        .methods
        .document
        .delete
        .add_estimation_costs_for_erase_document = 1;

    let error = Drive::add_estimation_costs_for_erase_document(
        id,
        &contract,
        document_type_of(&contract),
        &mut HashMap::new(),
        &version,
    )
    .expect_err("an unimplemented version must be refused");
    assert!(matches!(
        error,
        Error::Drive(DriveError::UnknownVersionMismatch { .. })
    ));
}

/// The recipient work is priced from the ordinary balance-update path and
/// scales with the number of beneficiaries one erase can credit, without
/// touching any identity's balance.
#[test]
fn should_price_the_refund_recipients_an_erase_can_credit() {
    use dpp::block::epoch::Epoch;

    let drive = setup_drive_with_initial_state_structure(None);
    let epoch = Epoch::new(0).unwrap();
    let version = latest();

    let priced = drive
        .erase_refund_recipient_cost(&epoch, version)
        .expect("expected a price for the recipient work");
    assert!(
        priced.processing_fee > 0,
        "crediting a beneficiary is work somebody has to pay for"
    );
    assert!(
        priced.fee_refunds.0.is_empty() && priced.storage_fee == 0,
        "pricing must not invent a refund or a storage charge of its own"
    );

    // The same shape at half the bound costs proportionally less, which is what
    // makes this a price for the bound rather than a constant.
    let mut halved = version.clone();
    halved
        .system_limits
        .max_document_revisions_erased_per_transition = Some(chunk_size() as u16 / 2);
    let smaller = drive
        .erase_refund_recipient_cost(&epoch, &halved)
        .expect("expected a price at the smaller bound");
    assert!(
        smaller.processing_fee < priced.processing_fee,
        "a smaller chunk credits fewer beneficiaries: {} is not below {}",
        smaller.processing_fee,
        priced.processing_fee
    );

    // Every balance in the drive is untouched: this is a measurement, not a
    // mutation.
    assert!(
        drive
            .fetch_identity_balance(default_owner_id(), None, version)
            .expect("expected the balance read to run")
            .is_none(),
        "pricing must not create the identity it measures against"
    );
}

/// The synthetic identity the price is measured against.
fn default_owner_id() -> [u8; 32] {
    [0u8; 32]
}

/// Revisions written in different epochs are refunded under the epoch each was
/// written in, and the payout tail decides the amount: a byte written recently
/// has more of its one-time charge still unspent than one written long ago, so
/// it refunds more.
#[test]
fn should_refund_each_writer_under_the_epoch_they_wrote_in() {
    use dpp::block::epoch::Epoch;

    let early_writer = [40u8; 32];
    let late_writer = [41u8; 32];
    let version = latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = json_document_to_contract(FAMILY_HISTORY_CONTRACT, false, version).unwrap();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .expect("expected to apply the contract");
    let document_type = document_type_of(&contract);
    let mut document =
        json_document_to_document(PERSON, Some(early_writer.into()), document_type, version)
            .unwrap();

    let write = |revision: u64, writer: [u8; 32], epoch_index: u16, first: bool| {
        let mut document = document.clone();
        document.set_revision(Some(revision));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((
                            &document,
                            Some(Cow::Owned(StorageFlags::new_single_epoch(
                                epoch_index,
                                Some(writer),
                            ))),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                !first,
                BlockInfo {
                    epoch: Epoch::new(epoch_index).unwrap(),
                    ..BlockInfo::default_with_time(1_000 + revision)
                },
                true,
                None,
                version,
                None,
            )
            .expect("expected to write a revision");
    };
    // The first revision also creates the document's history subtree and its
    // index references, so it is not the same amount of storage as an appended
    // one. A third identity pays for it, leaving the two compared below as one
    // appended revision each and differing only in the epoch they landed in.
    write(1, [39u8; 32], 0, true);
    write(2, early_writer, 0, false);
    write(3, late_writer, 6, false);
    document.set_revision(Some(3));
    let id = document.id();

    let block_info = BlockInfo {
        epoch: Epoch::new(6).unwrap(),
        ..BlockInfo::default_with_time(5_000)
    };
    let batch = drive
        .delete_document_for_contract_operations(
            id,
            &contract,
            document_type,
            &block_info,
            Some(Identifier::new(early_writer)),
            None,
            &mut None,
            None,
            version,
        )
        .expect("expected to delete");
    drive
        .apply_batch_low_level_drive_operations(None, None, batch, &mut vec![], &version.drive)
        .expect("expected to apply the delete");

    let fee = erase_at(&drive, &contract, id, &block_info);

    let early = fee
        .fee_refunds
        .0
        .get(&early_writer)
        .expect("the first writer must be credited");
    let late = fee
        .fee_refunds
        .0
        .get(&late_writer)
        .expect("the second writer must be credited");
    // Every byte is refunded under the epoch it was written in, so the first
    // writer appears twice: once for the revision it wrote in epoch 0, and once
    // for the lifecycle record it wrote as the deleter in epoch 6.
    assert_eq!(
        early.keys().copied().collect::<Vec<_>>(),
        vec![0, 6],
        "the first writer's revision and the record it wrote as deleter"
    );
    assert_eq!(
        late.keys().copied().collect::<Vec<_>>(),
        vec![6],
        "the second writer wrote only its revision, in epoch 6"
    );

    let early_revision = early[&0];
    let late_revision = late[&6];
    assert!(
        late_revision > early_revision,
        "the same revision written six epochs later has more of its one-time \
         charge still unspent, so it refunds more: {late_revision} is not above {early_revision}"
    );
}

/// Erases at a caller-chosen block, so a test can put the write and the erase
/// in different epochs.
fn erase_at(
    drive: &Drive,
    contract: &DataContract,
    id: Identifier,
    block_info: &BlockInfo,
) -> dpp::fee::fee_result::FeeResult {
    let mut operations = vec![];
    let batch = drive
        .erase_document_for_contract_operations(
            id,
            contract,
            document_type_of(contract),
            block_info,
            &mut None,
            None,
            latest(),
        )
        .expect("expected to build the erase operations");
    drive
        .apply_batch_low_level_drive_operations(None, None, batch, &mut operations, &latest().drive)
        .expect("expected to apply the erase");
    Drive::calculate_fee(
        None,
        Some(operations),
        &block_info.epoch,
        drive.config.epochs_per_era,
        latest(),
        None,
    )
    .expect("expected a fee")
}

/// A refund is a credit to a real identity, not just a map entry: the amount
/// the fee result names is the amount the beneficiary's balance gains.
#[test]
fn should_credit_a_refund_to_the_beneficiarys_balance() {
    let owner = [42u8; 32];
    let version = latest();
    let (drive, contract, id) = setup_history(3, owner);
    drive
        .add_new_identity(
            dpp::identity::Identity::create_basic_identity(Identifier::new(owner), version)
                .expect("expected a basic identity"),
            false,
            &BlockInfo::default(),
            true,
            None,
            version,
        )
        .expect("expected to create the beneficiary");
    let before = drive
        .fetch_identity_balance(owner, None, version)
        .expect("expected to read the balance")
        .expect("the beneficiary exists");

    delete(&drive, &contract, id, Identifier::new(owner), 5_000);
    let fee = erase(&drive, &contract, id, 6_000);
    let refunded: u64 = fee
        .fee_refunds
        .0
        .get(&owner)
        .map(|per_epoch| per_epoch.values().sum())
        .expect("the writer must be credited");

    drive
        .add_to_identity_balance(owner, refunded, &BlockInfo::default(), true, None, version)
        .expect("expected to credit the refund");
    let after = drive
        .fetch_identity_balance(owner, None, version)
        .expect("expected to read the balance")
        .expect("the beneficiary exists");
    assert_eq!(
        after - before,
        refunded,
        "the credited amount is exactly what the fee result named"
    );
}

/// The estimate an erase is admitted against is sized for a full chunk, so a
/// one-revision erase needs the same balance as a hundred-revision one. That is
/// the point of the bound: it is what an erase can cost, not what this one will.
#[test]
fn should_admit_a_one_revision_erase_only_against_the_full_chunk_estimate() {
    let owner = [43u8; 32];
    let (drive, contract, id) = setup_history(1, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);

    let estimated = estimated_erase_fee(&drive, &contract, id);
    let actual = erase(&drive, &contract, id, 6_000);
    assert!(
        estimated.processing_fee > actual.processing_fee * 4,
        "the estimate must be sized for the bound, not for the one revision this          document holds: {} against {}",
        estimated.processing_fee,
        actual.processing_fee
    );
}

/// GroveDB's batch consistency checking is off by default and on in some
/// deployments. A lifecycle batch has to produce the same state and the same
/// fees either way, or the two disagree about consensus.
#[test]
fn should_erase_identically_with_batch_consistency_checking_on() {
    let chunk = chunk_size();
    let mut roots = vec![];
    let mut fees = vec![];
    for verify in [false, true] {
        let owner = [44u8; 32];
        let version = latest();
        let directory = tempfile::TempDir::new().unwrap();
        let (drive, _) = Drive::open(
            directory.path(),
            Some(crate::config::DriveConfig {
                batching_consistency_verification: verify,
                ..Default::default()
            }),
        )
        .unwrap();
        drive.create_initial_state_structure(None, version).unwrap();
        let contract = json_document_to_contract(FAMILY_HISTORY_CONTRACT, false, version).unwrap();
        drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
            .expect("expected to apply the contract");
        let document_type = document_type_of(&contract);
        let mut document =
            json_document_to_document(PERSON, Some(owner.into()), document_type, version).unwrap();
        let flags = Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(owner))));
        for revision in 1..=chunk {
            document.set_revision(Some(revision));
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentRefInfo((
                                &document,
                                flags.clone(),
                            )),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    revision > 1,
                    BlockInfo::default_with_time(1_000 + revision),
                    true,
                    None,
                    version,
                    None,
                )
                .expect("expected to write a revision");
        }
        let id = document.id();
        delete(&drive, &contract, id, Identifier::new(owner), 5_000);
        fees.push(erase(&drive, &contract, id, 6_000).processing_fee);
        roots.push(
            drive
                .grove
                .root_hash(None, &version.drive.grove_version)
                .value
                .unwrap(),
        );
        assert!(matches!(
            lifecycle_of(&drive, &contract, id),
            DocumentLifecycleState::Absent
        ));
    }
    assert_eq!(
        roots[0], roots[1],
        "consistency checking must not change the state a lifecycle batch commits"
    );
    assert_eq!(fees[0], fees[1], "nor what it costs");
}

/// Two deletes of different documents of one type, combined into one batch
/// before either is applied and before the type has a lifecycle container, each
/// emit the insert of the same shared key. Every document operation is
/// converted on its own and cannot see its siblings, so neither can tell that
/// the other is already creating it.
///
/// GroveDB refuses the batch rather than committing one of the two, so nothing
/// is written and the caller is told. That fail-closed behaviour is what this
/// pins. Consensus cannot reach it: a batch state transition carries exactly
/// one document transition at every protocol version, so two document
/// operations never share a batch by that route, and no other caller combines
/// keep-history deletes.
#[test]
fn should_refuse_rather_than_half_apply_two_deletes_sharing_a_new_container() {
    assert_eq!(
        latest().system_limits.max_transitions_in_documents_batch,
        1,
        "the cap is what keeps consensus away from this shape"
    );
    use crate::util::batch::{DocumentOperationType, DriveOperation};
    use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};

    let owner = [45u8; 32];
    let version = latest();
    let (drive, contract, first_id) = setup_history(1, owner);
    let document_type = document_type_of(&contract);
    let mut second =
        json_document_to_document(PERSON, Some(owner.into()), document_type, version).unwrap();
    second.set_id(Identifier::new([46u8; 32]));
    second.set_revision(Some(1));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((
                        &second,
                        Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(owner)))),
                    )),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default_with_time(2_000),
            true,
            None,
            version,
            None,
        )
        .expect("expected a second document");

    let delete_operation = |id: Identifier| {
        DriveOperation::DocumentOperation(DocumentOperationType::DeleteDocument {
            document_id: id,
            deleter_id: Some(Identifier::new(owner)),
            contract_info: DataContractInfo::BorrowedDataContract(&contract),
            document_type_info: DocumentTypeInfo::DocumentTypeName("person".to_string()),
        })
    };
    drive
        .apply_drive_operations(
            vec![delete_operation(first_id), delete_operation(second.id())],
            true,
            &BlockInfo::default_with_time(5_000),
            None,
            version,
            None,
        )
        .expect_err("the duplicated container insert must be refused, not half applied");

    for id in [first_id, second.id()] {
        assert!(
            matches!(
                lifecycle_of(&drive, &contract, id),
                DocumentLifecycleState::Active(_)
            ),
            "a refused batch must leave both documents exactly as they were"
        );
    }

    // One at a time is the supported shape, and the second finds the container
    // the first created.
    for id in [first_id, second.id()] {
        drive
            .apply_drive_operations(
                vec![delete_operation(id)],
                true,
                &BlockInfo::default_with_time(6_000),
                None,
                version,
                None,
            )
            .expect("expected one delete at a time to succeed");
    }
    for id in [first_id, second.id()] {
        assert!(matches!(
            lifecycle_of(&drive, &contract, id),
            DocumentLifecycleState::Deleted(_)
        ));
    }
}

/// Every state the lifecycle can be in has to survive the proof round trip, and
/// each of the four claimed times has to be the one the proof authenticates: a
/// node that reports a different deletion time, a different erasure start, or a
/// different remaining count must fail verification rather than be believed.
#[test]
fn should_prove_the_erasing_state_and_reject_a_tampered_claim() {
    let chunk = chunk_size();
    let owner = [47u8; 32];
    let (drive, contract, id) = setup_history(chunk + 2, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);
    erase(&drive, &contract, id, 6_000);

    let query = |selector| DocumentHistoryQueryV1 {
        contract_id: contract.id().to_buffer(),
        document_type_name: "person".into(),
        document_id: id.to_buffer(),
        // A single-revision read is capped at one entry by the selector's own
        // rule; the page selectors take the full page.
        limit: Some(match selector {
            DocumentHistorySelector::Revision(_) => 1,
            _ => 10,
        }),
        selector,
    };

    // A populated page, and a page whose lower bound is past the end of what
    // survives. Both must authenticate the same erasing metadata.
    for selector in [
        DocumentHistorySelector::StartAtTime(0),
        DocumentHistorySelector::StartAtTime(u64::MAX / 2),
        DocumentHistorySelector::Revision(1),
    ] {
        let query = query(selector);
        let (page, proof) = drive
            .prove_document_history_v1(&query, document_type_of(&contract), None, latest())
            .expect("expected to prove the page");
        let (_, verified) = Drive::verify_document_history_v1(
            &query,
            &proof,
            document_type_of(&contract),
            latest(),
        )
        .expect("expected the proof to verify");
        assert_eq!(verified, page);
        assert_eq!(verified.lifecycle.state, DocumentHistoryState::Erasing);
        assert_eq!(verified.lifecycle.remaining_revisions, 2);
        assert_eq!(verified.lifecycle.times.deleted_at_ms, 5_000);
        assert_eq!(verified.lifecycle.times.erasing_started_at_ms, 6_000);
        assert_eq!(verified.lifecycle.times.erasing_from_revision, chunk + 2);

        // A claim that differs from the proof in any one field is not the
        // proof's claim, and the verifier rebuilds it rather than trusting it.
        let mut tampered = verified.lifecycle.clone();
        for corrupt in [
            &mut tampered.times.deleted_at_ms,
            &mut tampered.times.erasing_started_at_ms,
            &mut tampered.times.erasing_from_time_ms,
            &mut tampered.times.erasing_from_revision,
        ] {
            *corrupt += 1;
        }
        tampered.remaining_revisions += 1;
        tampered.state = DocumentHistoryState::Deleted;
        assert_ne!(
            tampered, verified.lifecycle,
            "the verifier's answer is derived, so a different claim cannot match it"
        );
    }

    // And the terminal state after the erasure finishes.
    erase(&drive, &contract, id, 7_000);
    let query = query(DocumentHistorySelector::StartAtTime(0));
    let (page, proof) = drive
        .prove_document_history_v1(&query, document_type_of(&contract), None, latest())
        .expect("expected to prove the absent id");
    let (_, verified) =
        Drive::verify_document_history_v1(&query, &proof, document_type_of(&contract), latest())
            .expect("expected the absence proof to verify");
    assert_eq!(verified, page);
    assert_eq!(verified.lifecycle.state, DocumentHistoryState::Absent);
    assert_eq!(verified.lifecycle.remaining_revisions, 0);
    assert_eq!(verified.lifecycle.times, Default::default());
    assert!(
        proof.entries_proof.is_none(),
        "there is no history tree left to page over"
    );
}

/// After a partial erasure the surviving revisions are the oldest ones and
/// still number from one, so a by-revision read lands on the revision it asks
/// for rather than on whatever is now at that offset.
#[test]
fn should_read_revisions_by_position_after_a_partial_erasure() {
    let chunk = chunk_size();
    let owner = [48u8; 32];
    let (drive, contract, id) = setup_history(chunk + 3, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);
    erase(&drive, &contract, id, 6_000);

    for revision in 1..=3u64 {
        let query = DocumentHistoryQueryV1 {
            contract_id: contract.id().to_buffer(),
            document_type_name: "person".into(),
            document_id: id.to_buffer(),
            selector: DocumentHistorySelector::Revision(revision),
            limit: Some(1),
        };
        let page = drive
            .fetch_document_history_v1(&query, document_type_of(&contract), None, latest())
            .expect("expected to read a surviving revision");
        assert_eq!(
            page.entries.len(),
            1,
            "revision {revision} survives a partial erasure"
        );
        assert_eq!(page.entries[0].revision, revision);
    }

    // The revisions the chunk removed are gone, not silently answered with a
    // neighbour.
    let query = DocumentHistoryQueryV1 {
        contract_id: contract.id().to_buffer(),
        document_type_name: "person".into(),
        document_id: id.to_buffer(),
        selector: DocumentHistorySelector::Revision(4),
        limit: Some(1),
    };
    let page = drive
        .fetch_document_history_v1(&query, document_type_of(&contract), None, latest())
        .expect("expected the read to run");
    assert!(
        page.entries.is_empty(),
        "a removed revision must not be answered with the one that took its place"
    );
}
