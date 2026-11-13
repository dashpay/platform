//! The storage side of the keep-history document lifecycle: what a delete
//! leaves behind, what an erase removes, and what a chunked erasure looks like
//! from one transition to the next.

use super::fetch::DocumentLifecycleState;
use crate::drive::document::paths::{document_history_path, document_lifecycle_path};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::document_history_drive_query::{
    DocumentHistoryDriveQuery, DocumentHistoryFilter, DocumentHistoryLifecycle,
    DocumentHistoryState,
};
use crate::query::{
    DriveDocumentQuery, SingleDocumentDriveQuery, SingleDocumentDriveQueryContestedStatus,
};
use crate::util::estimated_layers::EstimatedLayers;
use crate::util::grove_operations::BatchDeleteApplyType::StatefulBatchDelete;
use crate::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::block::epoch::Epoch;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::DataContract;
use dpp::document::lifecycle::DocumentLifecycleRecord;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::identifier::Identifier;
use dpp::serialization::PlatformDeserializableUntrusted;
use dpp::tests::json_document::{json_document_to_contract, json_document_to_document};
use dpp::version::PlatformVersion;
use grovedb::{MaybeTree, TransactionArg, TreeType};
use std::borrow::Cow;

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
        .expect("protocol 15 bounds the erase chunk") as u64
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
        .map(|bytes| {
            DocumentLifecycleRecord::deserialize_from_bytes_untrusted(&bytes)
                .expect("expected a valid record")
        })
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
) -> DocumentHistoryLifecycle {
    let query = DocumentHistoryDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "person".into(),
        document_id: id.to_buffer(),
        filter: DocumentHistoryFilter::StartAtTime(0),
        limit: Some(10),
    };
    let page = drive
        .fetch_document_history(&query, document_type_of(contract), None, latest())
        .expect("expected to fetch history");
    let proof = drive
        .prove_document_history(&query, document_type_of(contract), None, latest())
        .expect("expected to prove history");
    let (_, verified) =
        Drive::verify_document_history(&query, &proof, document_type_of(contract), latest())
            .expect("expected the proof to verify");
    assert_eq!(
        verified, page,
        "the proved history must match the read history"
    );
    verified
        .lifecycle
        .expect("the latest protocol authenticates lifecycle metadata")
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
    drive
        .calculate_fee(None, Some(operations), &block_info.epoch, latest())
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
    drive
        .calculate_fee(None, Some(operations), &block_info.epoch, latest())
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
    let mut layers = Some(EstimatedLayers::new());
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
    drive
        .calculate_fee(None, Some(operations), &block_info.epoch, latest())
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
        )
        .expect("an erased id is reusable");
    assert_eq!(visible_document_count(&drive, &contract), 1);
}

/// `override_document` tells the writer the caller has already checked the
/// primary-key entry, so it may skip that check on the current pointer. A
/// deleted document has no current pointer but still retains its revisions,
/// and nothing the caller checked says so; the retained-history refusal must
/// therefore hold whatever the caller claims about the pointer.
#[test]
fn should_refuse_to_create_over_retained_revisions_even_when_the_caller_overrides() {
    let owner = [14u8; 32];
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
            true,
            BlockInfo::default_with_time(6_000),
            true,
            None,
            version,
        )
        .expect_err("an override must not merge a new document into a deleted one's history");
    assert!(matches!(
        error,
        Error::Drive(DriveError::CorruptedDocumentAlreadyExists(_))
    ));
    assert_eq!(
        history_metadata(&drive, &contract, id).remaining_revisions,
        2,
        "the refused write must leave the retained history untouched"
    );
    assert!(matches!(
        lifecycle_of(&drive, &contract, id),
        DocumentLifecycleState::Deleted(_)
    ));
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
    let deleter = [80u8; 32];
    let (drive, contract, id) = setup_history(3, owner);
    delete(&drive, &contract, id, Identifier::new(deleter), 5_000);
    let record_refunds = isolated_record_removal_refunds(&drive, &contract, id);

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
    assert_eq!(
        fee.fee_refunds.0.get(&deleter),
        record_refunds.0.get(&deleter)
    );
    assert!(record_refunds.0.contains_key(&deleter));
}

fn isolated_record_removal_refunds(
    drive: &Drive,
    contract: &DataContract,
    id: Identifier,
) -> dpp::fee::fee_result::refunds::FeeRefunds {
    let transaction = drive.grove.start_transaction();
    let mut batch = vec![];
    drive
        .batch_delete(
            document_lifecycle_path(contract.id_ref().as_bytes(), "person")
                .as_slice()
                .into(),
            id.as_slice(),
            StatefulBatchDelete {
                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
            },
            Some(&transaction),
            &mut batch,
            &latest().drive,
        )
        .unwrap();
    let mut operations = vec![];
    drive
        .apply_batch_low_level_drive_operations(
            None,
            Some(&transaction),
            batch,
            &mut operations,
            &latest().drive,
        )
        .unwrap();
    drive
        .calculate_fee(
            None,
            Some(operations),
            &BlockInfo::default_with_time(6_000).epoch,
            latest(),
        )
        .unwrap()
        .fee_refunds
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
            estimated.storage_fee >= actual.storage_fee,
            "the storage estimate under-charged a {revisions}-revision erase: {} < {}",
            estimated.storage_fee,
            actual.storage_fee
        );
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

    let mut selector = DocumentHistoryFilter::StartAtTime(0);
    let mut seen = vec![];
    loop {
        let query = DocumentHistoryDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: "person".into(),
            document_id: id.to_buffer(),
            filter: selector,
            limit: Some(10),
        };
        let page = drive
            .fetch_document_history(&query, document_type_of(&contract), None, latest())
            .expect("expected to fetch a page");
        let proof = drive
            .prove_document_history(&query, document_type_of(&contract), None, latest())
            .expect("expected to prove a page");
        let (_, verified) =
            Drive::verify_document_history(&query, &proof, document_type_of(&contract), latest())
                .expect("expected the page proof to verify");
        let lifecycle = verified
            .lifecycle
            .expect("the latest protocol authenticates lifecycle metadata");
        assert_eq!(lifecycle.state, DocumentHistoryState::Deleted);
        assert_eq!(lifecycle.remaining_revisions, 12);
        assert_eq!(lifecycle.times.deleted_at_ms, 5_000);
        seen.extend(page.entries.iter().map(|entry| entry.revision));
        let Some(last) = page.entries.last() else {
            break;
        };
        selector = DocumentHistoryFilter::StartAfter {
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
    // The terminal chunk removed the record along with the revisions, so the
    // id reads as never deleted and the refusal is the not-deleted one.
    assert!(matches!(error, Error::Drive(DriveError::InvalidInput(_))));
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
    assert_ne!(before, after, "the erase start rewrote the record");
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
    let deleter = [81u8; 32];
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
        )
        .expect("expected a second writer's revision");

    delete(&drive, &contract, id, Identifier::new(deleter), 5_000);
    let record_refunds = isolated_record_removal_refunds(&drive, &contract, id);
    let fee = erase(&drive, &contract, id, 6_000);

    assert!(
        fee.fee_refunds.0.contains_key(&first) && fee.fee_refunds.0.contains_key(&second),
        "each writer must be credited for the revision they paid for; got {:?}",
        fee.fee_refunds.0.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        fee.fee_refunds.0.get(&deleter),
        record_refunds.0.get(&deleter)
    );
    assert!(record_refunds.0.contains_key(&deleter));
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
    use crate::util::estimated_layers::EstimatedLayers;

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
        Some(0),
        "protocol 15 selects the only implementation there is"
    );
    version
        .drive
        .methods
        .document
        .delete
        .add_estimation_costs_for_erase_document = Some(1);

    let error = Drive::add_estimation_costs_for_erase_document(
        id,
        &contract,
        document_type_of(&contract),
        &mut EstimatedLayers::new(),
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

    let mut no_revisions = version.clone();
    no_revisions
        .system_limits
        .max_document_revisions_erased_per_transition = Some(0);
    let structural = drive
        .erase_refund_recipient_cost(&epoch, &no_revisions)
        .expect("expected the structural-recipient price");
    let mut one_revision = no_revisions.clone();
    one_revision
        .system_limits
        .max_document_revisions_erased_per_transition = Some(1);
    let with_one_revision = drive
        .erase_refund_recipient_cost(&epoch, &one_revision)
        .expect("expected the one-revision price");
    let one_recipient = with_one_revision
        .processing_fee
        .checked_sub(structural.processing_fee)
        .expect("one additional recipient increases the price");
    assert_eq!(
        structural.processing_fee,
        one_recipient * 2,
        "terminal erase refunds only the lifecycle record and history subtree"
    );
    assert_eq!(
        priced.processing_fee,
        one_recipient * (chunk_size() + 2),
        "the full estimate prices every revision plus two structural recipients"
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
    drive
        .calculate_fee(None, Some(operations), &block_info.epoch, latest())
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

/// Sibling deletes share the per-type lifecycle container even before it is committed.
#[test]
fn should_apply_two_deletes_sharing_a_new_container() {
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
        )
        .expect("sibling deletes create their shared lifecycle container once");

    for id in [first_id, second.id()] {
        assert!(matches!(
            lifecycle_of(&drive, &contract, id),
            DocumentLifecycleState::Deleted(_)
        ));
    }
}

#[test]
fn should_keep_sibling_lifecycle_containers_separate_and_reuse_existing_containers() {
    use crate::util::batch::{DocumentOperationType, DriveOperation};
    use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::schema::DataContractSchemaMethodsV0;

    let version = latest();
    let owner = Identifier::from([48; 32]);
    for existing in [false, true] {
        let drive = setup_drive_with_initial_state_structure(None);
        let mut contract =
            json_document_to_contract(FAMILY_HISTORY_CONTRACT, false, version).unwrap();
        let schema = contract
            .document_type_for_name("person")
            .unwrap()
            .schema()
            .clone();
        contract
            .set_document_schema("otherPerson", schema, false, &mut vec![], version)
            .unwrap();
        drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
            .unwrap();
        let insert = |name: &str, id: Identifier| {
            let document_type = contract.document_type_for_name(name).unwrap();
            let mut document =
                json_document_to_document(PERSON, Some(owner), document_type, version).unwrap();
            document.set_id(id);
            document.set_revision(Some(1));
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
                    BlockInfo::default_with_time(1000),
                    true,
                    None,
                    version,
                )
                .unwrap();
        };
        let documents: [(&str, Identifier); 2] = [
            ("person", [49; 32].into()),
            ("otherPerson", [50; 32].into()),
        ];
        for (name, id) in documents {
            if existing {
                let seed_id = Identifier::from([id.as_slice()[0] + 10; 32]);
                insert(name, seed_id);
                drive
                    .delete_document_for_contract(
                        seed_id,
                        &contract,
                        name,
                        BlockInfo::default_with_time(2000),
                        true,
                        None,
                        version,
                    )
                    .unwrap();
            }
            insert(name, id);
        }
        let operations = || {
            documents
                .iter()
                .map(|(name, id)| {
                    DriveOperation::DocumentOperation(DocumentOperationType::DeleteDocument {
                        document_id: *id,
                        deleter_id: Some(owner),
                        contract_info: DataContractInfo::BorrowedDataContract(&contract),
                        document_type_info: DocumentTypeInfo::DocumentTypeName((*name).to_string()),
                    })
                })
                .collect()
        };
        for apply in [false, true] {
            drive
                .apply_drive_operations(
                    operations(),
                    apply,
                    &BlockInfo::default_with_time(3000),
                    None,
                    version,
                )
                .expect("each type has its own lifecycle container in estimation and application");
        }
        for (name, id) in documents {
            assert!(matches!(
                drive
                    .fetch_document_lifecycle(
                        &contract,
                        contract.document_type_for_name(name).unwrap(),
                        id,
                        None,
                        None,
                        version,
                    )
                    .unwrap()
                    .0,
                DocumentLifecycleState::Deleted(_)
            ));
        }
    }
}

/// Every state the lifecycle can be in has to survive the proof round trip, and
/// corrupting the lifecycle proof must fail verification rather than return
/// unauthenticated metadata.
#[test]
fn should_prove_the_erasing_state_and_reject_a_tampered_proof() {
    let chunk = chunk_size();
    let owner = [47u8; 32];
    let (drive, contract, id) = setup_history(chunk + 2, owner);
    delete(&drive, &contract, id, Identifier::new(owner), 5_000);
    erase(&drive, &contract, id, 6_000);

    let query = |filter| DocumentHistoryDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "person".into(),
        document_id: id.to_buffer(),
        // A single-revision read is capped at one entry by the selector's own
        // rule; the page selectors take the full page.
        limit: Some(match filter {
            DocumentHistoryFilter::Revision(_) => 1,
            _ => 10,
        }),
        filter,
    };

    // A populated page, and a page whose lower bound is past the end of what
    // survives. Both must authenticate the same erasing metadata.
    for selector in [
        DocumentHistoryFilter::StartAtTime(0),
        DocumentHistoryFilter::StartAtTime(u64::MAX / 2),
        DocumentHistoryFilter::Revision(1),
    ] {
        let query = query(selector);
        let page = drive
            .fetch_document_history(&query, document_type_of(&contract), None, latest())
            .expect("expected to fetch the page");
        let proof = drive
            .prove_document_history(&query, document_type_of(&contract), None, latest())
            .expect("expected to prove the page");
        let (_, verified) =
            Drive::verify_document_history(&query, &proof, document_type_of(&contract), latest())
                .expect("expected the proof to verify");
        assert_eq!(verified, page);
        let lifecycle = verified
            .lifecycle
            .expect("the latest protocol authenticates lifecycle metadata");
        assert_eq!(lifecycle.state, DocumentHistoryState::Erasing);
        assert_eq!(lifecycle.remaining_revisions, 2);
        assert_eq!(lifecycle.times.deleted_at_ms, 5_000);
        assert_eq!(lifecycle.times.erasing_started_at_ms, 6_000);
        assert_eq!(lifecycle.times.erasing_from_revision, chunk + 2);

        let mut tampered = proof.clone();
        tampered.push(0xff);
        Drive::verify_document_history(&query, &tampered, document_type_of(&contract), latest())
            .expect_err("tampering with the lifecycle proof must fail verification");
    }

    // And the terminal state after the erasure finishes.
    erase(&drive, &contract, id, 7_000);
    let query = query(DocumentHistoryFilter::StartAtTime(0));
    let page = drive
        .fetch_document_history(&query, document_type_of(&contract), None, latest())
        .expect("expected to fetch the absent id");
    let proof = drive
        .prove_document_history(&query, document_type_of(&contract), None, latest())
        .expect("expected to prove the absent id");
    let (_, verified) =
        Drive::verify_document_history(&query, &proof, document_type_of(&contract), latest())
            .expect("expected the absence proof to verify");
    assert_eq!(verified, page);
    let lifecycle = verified
        .lifecycle
        .expect("the latest protocol authenticates lifecycle metadata");
    assert_eq!(lifecycle.state, DocumentHistoryState::Absent);
    assert_eq!(lifecycle.remaining_revisions, 0);
    assert_eq!(lifecycle.times, Default::default());
    assert!(
        verified.entries.is_empty(),
        "no revisions survive the terminal erase"
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
        let query = DocumentHistoryDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: "person".into(),
            document_id: id.to_buffer(),
            filter: DocumentHistoryFilter::Revision(revision),
            limit: Some(1),
        };
        let page = drive
            .fetch_document_history(&query, document_type_of(&contract), None, latest())
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
    let query = DocumentHistoryDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "person".into(),
        document_id: id.to_buffer(),
        filter: DocumentHistoryFilter::Revision(4),
        limit: Some(1),
    };
    let page = drive
        .fetch_document_history(&query, document_type_of(&contract), None, latest())
        .expect("expected the read to run");
    assert!(
        page.entries.is_empty(),
        "a removed revision must not be answered with the one that took its place"
    );
}

/// A history that the legacy layout left gapped keeps refusing by-revision
/// reads after the document is deleted: a revision no longer maps onto a
/// position, and an empty page for a revision that exists would otherwise
/// verify as an authenticated absence.
#[test]
fn should_keep_refusing_by_revision_reads_of_a_gapped_history_after_deletion() {
    let old = PlatformVersion::get(14).unwrap();
    let mut version = latest().clone();
    version
        .system_limits
        .max_document_revisions_erased_per_transition = Some(1);
    let new = &version;
    let drive = setup_drive_with_initial_state_structure(None);
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
    let owner = [9u8; 32];
    let mut document = json_document_to_document(
        "tests/supporting_files/contract/dashpay/profile0.json",
        Some(owner.into()),
        document_type,
        old,
    )
    .unwrap();
    // Two writes in one block under the legacy layout overwrite each other's
    // revision, so revision 2 is lost and the retained history is [1, 3].
    for (revision, time) in [(1, 1_000), (2, 2_000), (3, 2_000)] {
        document.set_revision(Some(revision));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((
                            &document,
                            Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(owner)))),
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
    let transaction = drive.grove.start_transaction();
    drive
        .migrate_document_history_storage(&transaction, new)
        .expect("expected the migration to run");
    drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("expected the migration to commit");
    let id = document.id();

    let query = |filter| DocumentHistoryDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "profile".into(),
        document_id: id.to_buffer(),
        // A single-revision read is capped at one entry by the selector's
        // own rule; the page selectors take the full page.
        limit: Some(match filter {
            DocumentHistoryFilter::Revision(_) => 1,
            _ => 10,
        }),
        filter,
    };
    let by_revision = [
        DocumentHistoryFilter::Revision(3),
        DocumentHistoryFilter::StartAtRevision(3),
        DocumentHistoryFilter::Revision(2),
    ];
    let is_gap_refusal = |error: Error| {
        let message = error.to_string();
        assert!(
            message.contains("revision gap"),
            "expected the gap refusal, got {message}"
        );
    };

    // While the document is current the gap already refuses both selectors.
    for selector in by_revision.clone() {
        is_gap_refusal(
            drive
                .fetch_document_history(&query(selector), document_type, None, new)
                .expect_err("a gapped history refuses by-revision reads"),
        );
    }

    let apply = |batch: Vec<crate::fees::op::LowLevelDriveOperation>| {
        drive
            .apply_batch_low_level_drive_operations(None, None, batch, &mut vec![], &new.drive)
            .expect("expected to apply the operations");
    };
    apply(
        drive
            .delete_document_for_contract_operations(
                id,
                &contract,
                document_type,
                &BlockInfo::default_with_time(5_000),
                Some(Identifier::new(owner)),
                None,
                &mut None,
                None,
                new,
            )
            .expect("expected to build the delete"),
    );
    let page = drive
        .fetch_document_history(
            &query(DocumentHistoryFilter::StartAtTime(0)),
            document_type,
            None,
            new,
        )
        .expect("time pagination still reads a deleted gapped history");
    let lifecycle = page
        .lifecycle
        .expect("protocol 15 authenticates lifecycle metadata");
    assert_eq!(lifecycle.state, DocumentHistoryState::Deleted);
    assert_eq!(lifecycle.remaining_revisions, 2);
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.revision)
            .collect::<Vec<_>>(),
        vec![1, 3]
    );
    for selector in by_revision.clone() {
        let query = query(selector.clone());
        is_gap_refusal(
            drive
                .fetch_document_history(&query, document_type, None, new)
                .expect_err("a deleted gapped history refuses by-revision reads"),
        );
        is_gap_refusal(
            drive
                .prove_document_history(&query, document_type, None, new)
                .expect_err("a deleted gapped history cannot be proved by revision"),
        );
    }

    // A one-revision chunk leaves the oldest revision of the migrated history.
    apply(
        drive
            .erase_document_for_contract_operations(
                id,
                &contract,
                document_type,
                &BlockInfo::default_with_time(6_000),
                &mut None,
                None,
                new,
            )
            .expect("expected to build the erase"),
    );
    let history_query = query(DocumentHistoryFilter::StartAtTime(0));
    let page = drive
        .fetch_document_history(&history_query, document_type, None, new)
        .unwrap();
    let proof = drive
        .prove_document_history(&history_query, document_type, None, new)
        .unwrap();
    let (_, verified) =
        Drive::verify_document_history(&history_query, &proof, document_type, new).unwrap();
    assert_eq!(verified, page);
    let lifecycle = page.lifecycle.unwrap();
    assert_eq!(lifecycle.state, DocumentHistoryState::Erasing);
    assert_eq!(lifecycle.remaining_revisions, 1);
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.revision)
            .collect::<Vec<_>>(),
        vec![1]
    );
    for selector in by_revision.clone() {
        is_gap_refusal(
            drive
                .fetch_document_history(&query(selector.clone()), document_type, None, new)
                .unwrap_err(),
        );
        is_gap_refusal(
            drive
                .prove_document_history(&query(selector), document_type, None, new)
                .unwrap_err(),
        );
    }
    for time in [0, 999, 1_000, 2_000, 4_999, 5_000, 6_000, u64::MAX] {
        let timed = SingleDocumentDriveQuery {
            contract_id: contract.id().to_buffer(),
            document_type_name: "profile".into(),
            document_type_keeps_history: true,
            document_id: id.to_buffer(),
            block_time_ms: Some(time),
            contested_status: SingleDocumentDriveQueryContestedStatus::NotContested,
        };
        let path_query = timed.construct_path_query(new).unwrap();
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &new.drive)
            .unwrap();
        for subset in [false, true] {
            let (_, returned) = timed
                .verify_proof(subset, &proof, document_type, new)
                .unwrap();
            assert!(returned.is_none(), "erasing is absent at time {time}");
        }
    }
    apply(
        drive
            .erase_document_for_contract_operations(
                id,
                &contract,
                document_type,
                &BlockInfo::default_with_time(7_000),
                &mut None,
                None,
                new,
            )
            .unwrap(),
    );
    for selector in by_revision {
        let page = drive
            .fetch_document_history(&query(selector), document_type, None, new)
            .expect("an unused id reads as an authenticated absence");
        assert_eq!(
            page.lifecycle
                .expect("protocol 15 authenticates lifecycle metadata")
                .state,
            DocumentHistoryState::Absent
        );
        assert!(page.entries.is_empty());
    }
}

/// A lifecycle delete given only the contract's id has to fetch the contract,
/// and that fetch is billed: its operation must come back with the delete's
/// operations, as an erase also preserves its contract fetch.
#[test]
fn should_keep_the_contract_fetch_cost_of_a_lifecycle_delete_by_contract_id() {
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
    use crate::util::batch::DocumentOperationType;
    use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};

    let owner = [48u8; 32];
    let version = latest();
    let (drive, contract, id) = setup_history(1, owner);
    let block_info = BlockInfo::default_with_time(5_000);

    let delete_with = |contract_info: DataContractInfo| {
        DocumentOperationType::DeleteDocument {
            document_id: id,
            deleter_id: Some(Identifier::new(owner)),
            contract_info,
            document_type_info: DocumentTypeInfo::DocumentTypeName("person".to_string()),
        }
        .into_low_level_drive_operations(&drive, &mut None, &block_info, None, version)
        .expect("expected the delete to convert")
    };

    // The delete bills its own reads too, so the comparison is against the
    // borrowed-contract path, which performs no fetch.
    let borrowed = delete_with(DataContractInfo::BorrowedDataContract(&contract));
    assert!(!borrowed.is_empty(), "the delete itself emits operations");

    let by_id = delete_with(DataContractInfo::DataContractId(contract.id()));
    assert!(
        matches!(
            by_id.first(),
            Some(LowLevelDriveOperation::PreCalculatedFeeResult(_))
        ),
        "the contract fetch is billed first, got {:?}",
        by_id.first()
    );
    assert_eq!(
        by_id[1..],
        borrowed[..],
        "after the fetch cost, the by-id path emits exactly the delete's operations"
    );
}

#[test]
fn should_preserve_released_non_history_delete_operations_by_contract_id() {
    use crate::fees::op::LowLevelDriveOperation;
    use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
    use crate::util::batch::DocumentOperationType;
    use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};

    for protocol in [12, 13, 14, 15] {
        let version = PlatformVersion::get(protocol).unwrap();
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = json_document_to_contract(
            "tests/supporting_files/contract/family/family-contract.json",
            false,
            version,
        )
        .unwrap();
        drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
            .unwrap();
        let document_type = document_type_of(&contract);
        let document = json_document_to_document(
            PERSON,
            Some(Identifier::new([82; 32])),
            document_type,
            version,
        )
        .unwrap();
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
                BlockInfo::default(),
                true,
                None,
                version,
            )
            .unwrap();
        let block = BlockInfo::default_with_time(5_000);
        let lower = |contract_info| {
            DocumentOperationType::DeleteDocument {
                document_id: document.id(),
                deleter_id: None,
                contract_info,
                document_type_info: DocumentTypeInfo::DocumentTypeName("person".into()),
            }
            .into_low_level_drive_operations(&drive, &mut None, &block, None, version)
            .unwrap()
        };
        let by_id = lower(DataContractInfo::DataContractId(contract.id()));
        let borrowed = lower(DataContractInfo::BorrowedDataContract(&contract));
        if protocol == 15 {
            assert!(matches!(
                by_id.first(),
                Some(LowLevelDriveOperation::PreCalculatedFeeResult(_))
            ));
            assert_eq!(&by_id[1..], borrowed);
        } else {
            assert_eq!(
                by_id, borrowed,
                "protocol {protocol} retains its original delete operations"
            );
        }
    }
}

#[test]
fn should_keep_non_history_delete_independent_of_pending_lifecycle_operations() {
    use crate::util::batch::DocumentOperationType;
    use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
    let version = latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/family/family-contract-only-first-name-index.json",
        false,
        version,
    )
    .unwrap();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let document_type = document_type_of(&contract);
    let first = json_document_to_document(
        PERSON,
        Some(Identifier::new([82; 32])),
        document_type,
        version,
    )
    .unwrap();
    let mut second = first.clone();
    second.set_id(Identifier::new([84; 32]));
    for document in [&first, &second] {
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                version,
            )
            .unwrap();
    }
    let block = BlockInfo::default_with_time(5_000);
    let build = |id, previous| {
        DocumentOperationType::DeleteDocument {
            document_id: id,
            deleter_id: None,
            contract_info: DataContractInfo::BorrowedDataContract(&contract),
            document_type_info: DocumentTypeInfo::DocumentTypeRef(document_type),
        }
        .into_low_level_drive_operations_after_ttl_drain(
            &drive, &mut None, previous, &block, None, version,
        )
        .unwrap()
    };
    let mut previous = build(first.id(), None);
    assert_eq!(
        build(second.id(), Some(&mut previous)),
        build(second.id(), None)
    );
}

fn install_lifecycle_record(drive: &Drive, contract: &DataContract, id: Identifier) {
    let mut operations = vec![];
    drive
        .add_lifecycle_record_operations(
            id,
            contract,
            document_type_of(contract),
            DocumentLifecycleRecord::deleted_at(5_000, 2, 2),
            None,
            &mut None,
            None,
            None,
            &mut operations,
            latest(),
        )
        .unwrap();
    drive
        .apply_batch_low_level_drive_operations(
            None,
            None,
            operations,
            &mut vec![],
            &latest().drive,
        )
        .unwrap();
}

#[test]
fn should_refuse_a_current_pointer_and_lifecycle_record_when_fetching_lifecycle() {
    let (drive, contract, id) = setup_history(2, [74; 32]);
    install_lifecycle_record(&drive, &contract, id);
    assert!(matches!(
        drive.fetch_document_lifecycle(
            &contract,
            document_type_of(&contract),
            id,
            None,
            None,
            latest()
        ),
        Err(Error::Drive(DriveError::CorruptedDriveState(_)))
    ));
}

#[test]
fn should_refuse_a_current_pointer_and_lifecycle_record_when_erasing() {
    let (drive, contract, id) = setup_history(2, [75; 32]);
    install_lifecycle_record(&drive, &contract, id);
    assert!(matches!(
        drive.erase_document_for_contract_operations(
            id,
            &contract,
            document_type_of(&contract),
            &BlockInfo::default_with_time(6_000),
            &mut None,
            None,
            latest(),
        ),
        Err(Error::Drive(DriveError::CorruptedDriveState(_)))
    ));
}

#[test]
fn should_refuse_to_replace_an_existing_deletion_record() {
    let (drive, contract, id) = setup_history(2, [76; 32]);
    install_lifecycle_record(&drive, &contract, id);
    let batch = drive.delete_document_for_contract_operations(
        id,
        &contract,
        document_type_of(&contract),
        &BlockInfo::default_with_time(6_000),
        None,
        None,
        &mut None,
        None,
        latest(),
    );
    let result = batch.and_then(|batch| {
        drive.apply_batch_low_level_drive_operations(
            None,
            None,
            batch,
            &mut vec![],
            &latest().drive,
        )
    });
    assert!(
        result.is_err(),
        "a deletion must not overwrite an existing lifecycle record"
    );
}

#[test]
fn should_refuse_to_write_an_incoherent_deletion_record() {
    let (drive, contract, id) = setup_history(2, [77; 32]);
    let result = drive.add_lifecycle_record_operations(
        id,
        &contract,
        document_type_of(&contract),
        DocumentLifecycleRecord::deleted_at(5_000, 0, 2),
        None,
        &mut None,
        None,
        None,
        &mut vec![],
        latest(),
    );
    assert!(matches!(
        result,
        Err(Error::Drive(DriveError::CorruptedDriveState(_)))
    ));
}

#[test]
fn should_refuse_to_start_erasing_before_the_deletion_time() {
    let (drive, contract, id) = setup_history(2, [78; 32]);
    delete(&drive, &contract, id, Identifier::new([79; 32]), 5_000);
    let mut version = latest().clone();
    version
        .system_limits
        .max_document_revisions_erased_per_transition = Some(1);
    let result = drive.erase_document_for_contract_operations(
        id,
        &contract,
        document_type_of(&contract),
        &BlockInfo::default_with_time(4_000),
        &mut None,
        None,
        &version,
    );
    assert!(matches!(
        result,
        Err(Error::Drive(DriveError::CorruptedDriveState(_)))
    ));
}

/// The erase operation is a public Drive method, reachable without the
/// transition validation that checks the lifecycle. It has to refuse a
/// document that is not deleted on its own: a document that was never deleted,
/// and one whose id was reused after an earlier erasure finished, both keep
/// every revision.
#[test]
fn should_refuse_to_erase_a_document_that_is_not_deleted() {
    let owner = [49u8; 32];
    let version = latest();
    let (drive, contract, id) = setup_history(2, owner);
    let document_type = document_type_of(&contract);
    let block_info = BlockInfo::default_with_time(5_000);
    let attempt = |drive: &Drive| {
        drive.erase_document_for_contract_operations(
            id,
            &contract,
            document_type,
            &block_info,
            &mut None,
            None,
            version,
        )
    };
    let revisions_retained = |drive: &Drive| {
        drive
            .fetch_document_history(
                &DocumentHistoryDriveQuery {
                    contract_id: contract.id().to_buffer(),
                    document_type_name: "person".into(),
                    document_id: id.to_buffer(),
                    filter: DocumentHistoryFilter::StartAtTime(0),
                    limit: Some(10),
                },
                document_type,
                None,
                version,
            )
            .expect("expected to read the history")
            .entries
            .len()
    };

    // Never deleted.
    let error = attempt(&drive).expect_err("an active document must not be erasable");
    assert!(
        matches!(error, Error::Drive(DriveError::InvalidInput(_))),
        "got {error:?}"
    );
    assert!(matches!(
        lifecycle_of(&drive, &contract, id),
        DocumentLifecycleState::Active(_)
    ));
    assert_eq!(revisions_retained(&drive), 2);

    // Deleted, fully erased, then the id reused by a fresh document: the
    // earlier erasure left the per-type lifecycle container behind, and that
    // must not stand in for a record of this document.
    delete(&drive, &contract, id, Identifier::new(owner), 6_000);
    erase(&drive, &contract, id, 7_000);
    assert!(matches!(
        lifecycle_of(&drive, &contract, id),
        DocumentLifecycleState::Absent
    ));
    let mut recreated =
        json_document_to_document(PERSON, Some(owner.into()), document_type, version)
            .expect("expected a person document");
    recreated.set_revision(Some(1));
    assert_eq!(recreated.id(), id, "the fixture reproduces the same id");
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((
                        &recreated,
                        Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(owner)))),
                    )),
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
        )
        .expect("expected the id to be free again");

    let error = attempt(&drive).expect_err("a recreated document must not be erasable");
    assert!(
        matches!(error, Error::Drive(DriveError::InvalidInput(_))),
        "got {error:?}"
    );
    assert!(matches!(
        lifecycle_of(&drive, &contract, id),
        DocumentLifecycleState::Active(_)
    ));
    assert_eq!(revisions_retained(&drive), 1);
}

#[test]
fn should_refuse_an_erase_limit_that_cannot_probe_a_following_revision() {
    let owner = [7; 32];
    let (drive, contract, id) = setup_history(1, owner);
    delete(&drive, &contract, id, owner.into(), 9000);
    for limit in [0, u16::MAX] {
        let mut version = latest().clone();
        version
            .system_limits
            .max_document_revisions_erased_per_transition = Some(limit);
        assert!(
            drive
                .erase_document_for_contract_operations(
                    id,
                    &contract,
                    document_type_of(&contract),
                    &BlockInfo::default_with_time(9001),
                    &mut None,
                    None,
                    &version
                )
                .is_err(),
            "limit {limit} cannot bound a chunk plus its probe"
        );
    }
}

#[test]
fn should_select_history_pointer_removal_by_the_storage_layout() {
    use crate::drive::document::paths::contract_documents_primary_key_path;

    let (drive, contract, id) = setup_history(1, [7; 32]);
    let document_type = document_type_of(&contract);
    let path = contract_documents_primary_key_path(contract.id_ref().as_bytes(), "person");
    for layout in [0, 2] {
        let mut version = latest().clone();
        version
            .drive
            .methods
            .document
            .document_history_storage_layout = layout;
        let result = drive.remove_document_from_primary_storage(
            id,
            document_type,
            path,
            &mut None,
            None,
            &mut vec![],
            &version,
        );
        assert!(
            result.is_err(),
            "layout {layout} cannot remove a history pointer"
        );
    }

    let mut version = latest().clone();
    version
        .drive
        .methods
        .document
        .insert
        .add_document_to_primary_storage = 0;
    drive
        .remove_document_from_primary_storage(
            id,
            document_type,
            path,
            &mut None,
            None,
            &mut vec![],
            &version,
        )
        .expect(
            "the layout slot, independently of the writer selector, describes the stored pointer",
        );
}

#[test]
fn should_refuse_erasure_from_an_incoherent_lifecycle_record() {
    use dpp::serialization::PlatformSerializable;

    let (drive, contract, id) = setup_history(1, [7; 32]);
    delete(&drive, &contract, id, [7; 32].into(), 9000);
    let path = document_lifecycle_path(contract.id_ref().as_bytes(), "person");
    drive
        .grove
        .insert(
            path.as_slice(),
            id.as_slice(),
            grovedb::Element::new_item(
                DocumentLifecycleRecord::deleted_at(9000, 1, 0)
                    .serialize_to_bytes()
                    .unwrap(),
            ),
            None,
            None,
            &latest().drive.grove_version,
        )
        .value
        .unwrap();
    assert!(
        drive
            .erase_document_for_contract_operations(
                id,
                &contract,
                document_type_of(&contract),
                &BlockInfo::default_with_time(9001),
                &mut None,
                None,
                latest(),
            )
            .is_err(),
        "only a coherent deletion may authorize removing retained revisions"
    );
}

#[test]
fn should_cover_distinct_storing_epoch_reads_in_a_minimal_document_erase_estimate() {
    use crate::drive::credit_pools::epochs::operations_factory::EpochOperations;
    use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
    use crate::util::batch::GroveDbOpBatch;
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_factory::DocumentFactory;
    use dpp::platform_value::platform_value;
    let mut version = latest().clone();
    version
        .system_limits
        .max_document_revisions_erased_per_transition = Some(4);
    let drive = setup_drive_with_initial_state_structure(Some(&version));
    let contract = DataContractFactory::new(version.protocol_version).unwrap()
        .create_with_value_config([7; 32].into(), 0, platform_value!({"person": {
            "type": "object", "documentsKeepHistory": true, "canBeDeleted": true, "canBeErased": true,
            "properties": {"a": {"type": "boolean", "position": 0}}, "additionalProperties": false
        }}), None, None).unwrap().data_contract_owned();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, &version)
        .unwrap();
    let mut epoch_records = GroveDbOpBatch::new();
    for epoch in 2..=6 {
        epoch_records.push(
            Epoch::new(epoch)
                .unwrap()
                .update_protocol_version_operation(version.protocol_version),
        );
    }
    drive
        .grove_apply_batch(epoch_records, false, None, &version.drive)
        .unwrap();
    let document_type = document_type_of(&contract);
    let mut document = DocumentFactory::new(version.protocol_version)
        .unwrap()
        .create_document(
            &contract,
            [7; 32].into(),
            "person".into(),
            platform_value!({}),
        )
        .unwrap();
    for revision in 1..=4 {
        document.set_revision(Some(revision));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((
                            &document,
                            Some(Cow::Owned(StorageFlags::new_single_epoch(
                                revision as u16 + 1,
                                Some([7; 32]),
                            ))),
                        )),
                        owner_id: Some([7; 32]),
                    },
                    contract: &contract,
                    document_type,
                },
                revision > 1,
                BlockInfo {
                    epoch: Epoch::new(revision as u16 + 1).unwrap(),
                    ..BlockInfo::default_with_time(revision * 1000)
                },
                true,
                None,
                &version,
            )
            .unwrap();
    }
    let block = BlockInfo {
        epoch: Epoch::new(6).unwrap(),
        ..BlockInfo::default_with_time(5000)
    };
    let deletes = drive
        .delete_document_for_contract_operations(
            document.id(),
            &contract,
            document_type,
            &block,
            Some([7; 32].into()),
            None,
            &mut None,
            None,
            &version,
        )
        .unwrap();
    drive
        .apply_batch_low_level_drive_operations(None, None, deletes, &mut vec![], &version.drive)
        .unwrap();
    let block = BlockInfo {
        epoch: Epoch::new(7).unwrap(),
        ..BlockInfo::default_with_time(6000)
    };
    let mut layers = Some(EstimatedLayers::new());
    let estimate = drive
        .erase_document_for_contract_operations(
            document.id(),
            &contract,
            document_type,
            &block,
            &mut layers,
            None,
            &version,
        )
        .unwrap();
    let mut costs = vec![];
    drive
        .apply_batch_low_level_drive_operations(layers, None, estimate, &mut costs, &version.drive)
        .unwrap();
    let estimated = drive
        .calculate_fee(None, Some(costs), &block.epoch, &version)
        .unwrap();
    let actual = drive
        .erase_document_for_contract_operations(
            document.id(),
            &contract,
            document_type,
            &block,
            &mut None,
            None,
            &version,
        )
        .unwrap();
    let mut costs = vec![];
    drive
        .apply_batch_low_level_drive_operations(None, None, actual, &mut costs, &version.drive)
        .unwrap();
    let actual = drive
        .calculate_fee(None, Some(costs), &block.epoch, &version)
        .unwrap();
    assert!(
        actual.processing_fee <= estimated.processing_fee,
        "distinct storing epochs must fit the erase estimate: actual {}, estimated {}",
        actual.processing_fee,
        estimated.processing_fee
    );
    assert!(actual.storage_fee <= estimated.storage_fee);
}

#[test]
fn should_retry_and_resume_the_lifecycle_after_rollback_and_reopening() {
    let version = latest();
    let owner = [74; 32];
    let (mut drive, contract, id) = setup_history(chunk_size() + 1, owner);
    let directory = drive
        .temp_dir
        .take()
        .expect("the database must outlive Drive");
    let document_type = document_type_of(&contract);
    let query = DocumentHistoryDriveQuery {
        contract_id: contract.id().to_buffer(),
        document_type_name: "person".into(),
        document_id: id.to_buffer(),
        filter: DocumentHistoryFilter::StartAtTime(0),
        limit: Some(10),
    };

    for (step, (state, remaining)) in [
        (DocumentHistoryState::Deleted, chunk_size() + 1),
        (DocumentHistoryState::Erasing, 1),
        (DocumentHistoryState::Absent, 0),
    ]
    .into_iter()
    .enumerate()
    {
        let before = history_metadata(&drive, &contract, id);
        let before_root = drive
            .grove
            .root_hash(None, &version.drive.grove_version)
            .unwrap()
            .unwrap();
        let block = BlockInfo::default_with_time(5_000 + step as u64 * 1_000);
        let mut staged_root = None;
        for commit in [false, true] {
            let transaction = drive.grove.start_transaction();
            let batch = if step == 0 {
                drive
                    .delete_document_for_contract_operations(
                        id,
                        &contract,
                        document_type,
                        &block,
                        Some(owner.into()),
                        None,
                        &mut None,
                        Some(&transaction),
                        version,
                    )
                    .unwrap()
            } else {
                drive
                    .erase_document_for_contract_operations(
                        id,
                        &contract,
                        document_type,
                        &block,
                        &mut None,
                        Some(&transaction),
                        version,
                    )
                    .unwrap()
            };
            drive
                .apply_batch_low_level_drive_operations(
                    None,
                    Some(&transaction),
                    batch,
                    &mut vec![],
                    &version.drive,
                )
                .unwrap();
            let page = drive
                .fetch_document_history(&query, document_type, Some(&transaction), version)
                .unwrap();
            let lifecycle = page.lifecycle.unwrap();
            assert_eq!(lifecycle.state, state);
            assert_eq!(lifecycle.remaining_revisions, remaining);
            assert_eq!(history_metadata(&drive, &contract, id), before);
            let root = drive
                .grove
                .root_hash(Some(&transaction), &version.drive.grove_version)
                .unwrap()
                .unwrap();
            if commit {
                assert_eq!(Some(root), staged_root, "retry must produce the same state");
                drive
                    .grove
                    .commit_transaction(transaction)
                    .unwrap()
                    .unwrap();
            } else {
                staged_root = Some(root);
                drive.grove.rollback_transaction(&transaction).unwrap();
                drop(transaction);
                assert_eq!(
                    drive
                        .grove
                        .root_hash(None, &version.drive.grove_version)
                        .unwrap()
                        .unwrap(),
                    before_root
                );
                assert_eq!(history_metadata(&drive, &contract, id), before);
            }
        }
        let config = drive.config.clone();
        drop(drive);
        (drive, _) = Drive::open(directory.path(), Some(config)).unwrap();
        assert_eq!(
            drive
                .grove
                .root_hash(None, &version.drive.grove_version)
                .unwrap()
                .unwrap(),
            staged_root.unwrap()
        );
        let persisted = history_metadata(&drive, &contract, id);
        assert_eq!(persisted.state, state);
        assert_eq!(persisted.remaining_revisions, remaining);
    }
    assert!(record_of(&drive, &contract, id).is_none());
    drop(drive);
}

#[test]
fn should_preserve_pending_operations_when_force_deleting_a_non_history_document() {
    let version = latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/family/family-contract-only-first-name-index.json",
        false,
        version,
    )
    .unwrap();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let document_type = document_type_of(&contract);
    let first = json_document_to_document(
        PERSON,
        Some(Identifier::new([82; 32])),
        document_type,
        version,
    )
    .unwrap();
    let mut second = first.clone();
    second.set_id(Identifier::new([84; 32]));
    for document in [&first, &second] {
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                version,
            )
            .unwrap();
    }
    let block = BlockInfo::default_with_time(5_000);
    let mut parent_version = version.clone();
    parent_version
        .drive
        .methods
        .document
        .delete
        .delete_document_for_contract_operations = 0;
    let build = |id, previous, version| {
        drive
            .force_delete_document_for_contract_operations(
                id,
                &contract,
                document_type,
                &block,
                None,
                previous,
                &mut None,
                None,
                version,
            )
            .unwrap()
    };
    let mut pending = build(first.id(), None, &parent_version);
    let expected = build(second.id(), Some(&mut pending), &parent_version);
    let mut pending = build(first.id(), None, &parent_version);
    assert_ne!(
        expected,
        build(second.id(), None, &parent_version),
        "the pending deletion changes the shared index subtree"
    );
    assert_eq!(build(second.id(), Some(&mut pending), version), expected);

    let mut unknown = version.clone();
    unknown
        .drive
        .methods
        .document
        .delete
        .delete_document_for_contract_operations = 3;
    let error = drive
        .force_delete_read_document_for_contract_operations(
            first,
            &contract,
            document_type,
            None,
            &mut None,
            block.time_ms,
            None,
            &unknown,
        )
        .unwrap_err();
    assert!(
        matches!(error, Error::Drive(DriveError::UnknownVersionMismatch {
        known_versions, received: 3, ..
    }) if known_versions == vec![0, 1])
    );
}

#[test]
fn should_cover_applied_lifecycle_delete_fees_in_the_admission_estimate() {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use crate::state_transition_action::batch::record_gas_sponsor_as_storage_owner;
    use crate::util::batch::drive_op_batch::DriveLowLevelOperationConverter;
    use crate::util::batch::{DocumentOperationType, DriveOperation};
    use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
    use dpp::data_contract::DataContractFactory;
    use dpp::document::document_factory::DocumentFactory;
    use dpp::platform_value::platform_value;
    use grovedb::Element;

    let version = latest();
    // Processing has wide headroom; storage is the tight side of this fee matrix.
    let mut underestimates = vec![];
    for case in ["first-container", "summable-countable", "sponsored"] {
        let drive = setup_drive_with_initial_state_structure(None);
        let schema = if case == "summable-countable" {
            platform_value!({
                "type": "object", "documentsKeepHistory": true, "canBeDeleted": true,
                "documentsMutable": false, "documentsCountable": true, "documentsSummable": "amount",
                "properties": {"amount": {"type": "integer", "minimum": 5, "maximum": 5, "position": 0}},
                "required": ["amount"], "additionalProperties": false,
                "indices": [{"name": "amount", "properties": [{"amount": "asc"}]}]
            })
        } else {
            platform_value!({
                "type": "object", "documentsKeepHistory": true, "canBeDeleted": true,
                "documentsMutable": false,
                "properties": {"amount": {"type": "integer", "minimum": 5, "maximum": 5, "position": 0}},
                "required": ["amount"], "additionalProperties": false
            })
        };
        let contract = DataContractFactory::new(version.protocol_version)
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
        drive
            .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
            .unwrap();
        let document_type = document_type_of(&contract);
        let properties = platform_value!({"amount": 5});
        let document = DocumentFactory::new(version.protocol_version)
            .unwrap()
            .create_document(&contract, [7; 32].into(), "person".into(), properties)
            .unwrap();
        let flags = Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some([7; 32]))));
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((&document, flags)),
                        owner_id: Some([7; 32]),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default_with_time(1000),
                true,
                None,
                version,
            )
            .unwrap();
        let type_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "person");
        assert!(drive
            .grove
            .get_raw_optional(
                type_path.as_slice().into(),
                &[1],
                None,
                &version.drive.grove_version
            )
            .value
            .unwrap()
            .is_none());
        let block = BlockInfo::default_with_time(5000);
        let price = |estimate: bool| {
            let mut high_level = vec![DriveOperation::DocumentOperation(
                DocumentOperationType::DeleteDocument {
                    document_id: document.id(),
                    deleter_id: Some([8; 32].into()),
                    contract_info: DataContractInfo::BorrowedDataContract(&contract),
                    document_type_info: DocumentTypeInfo::DocumentTypeRef(document_type),
                },
            )];
            if case == "sponsored" {
                record_gas_sponsor_as_storage_owner(&mut high_level, [9; 32].into());
            }
            let mut layers = estimate.then(EstimatedLayers::new);
            let operations = high_level
                .pop()
                .unwrap()
                .into_low_level_drive_operations(&drive, &mut layers, &block, None, version)
                .unwrap();
            let mut costs = vec![];
            drive
                .apply_batch_low_level_drive_operations(
                    layers,
                    None,
                    operations,
                    &mut costs,
                    &version.drive,
                )
                .unwrap();
            drive
                .calculate_fee(None, Some(costs), &block.epoch, version)
                .unwrap()
        };
        let estimated = price(true);
        let actual = price(false);
        println!(
            "delete {case}: estimated storage={} processing={}, actual storage={} processing={}",
            estimated.storage_fee,
            estimated.processing_fee,
            actual.storage_fee,
            actual.processing_fee
        );
        if estimated.storage_fee < actual.storage_fee
            || estimated.processing_fee < actual.processing_fee
        {
            underestimates.push(case);
        }
        let record = drive
            .grove
            .get_raw(
                document_lifecycle_path(contract.id_ref().as_bytes(), "person")
                    .as_slice()
                    .into(),
                document.id().as_bytes(),
                None,
                &version.drive.grove_version,
            )
            .value
            .unwrap();
        assert!(matches!(record, Element::Item(..)));
        let flags = StorageFlags::map_cow_some_element_flags_ref(record.get_flags())
            .unwrap()
            .unwrap();
        assert_eq!(
            flags.owner_id(),
            Some(&if case == "sponsored" {
                [9; 32]
            } else {
                [8; 32]
            })
        );
    }
    assert!(
        underestimates.is_empty(),
        "lifecycle delete estimates must cover applied fees: {underestimates:?}"
    );
}

fn assert_delete_estimate_covers_applied(
    drive: &Drive,
    contract: &DataContract,
    id: Identifier,
    block: BlockInfo,
    fixture: &str,
) {
    let price = |apply| {
        drive
            .delete_document_for_contract(id, contract, "person", block, apply, None, latest())
            .unwrap()
    };
    let estimated = price(false);
    let applied = price(true);
    println!(
        "delete {fixture}: estimated storage={} processing={}, applied storage={} processing={}",
        estimated.storage_fee,
        estimated.processing_fee,
        applied.storage_fee,
        applied.processing_fee
    );
    // Processing has wide headroom; storage is the tight side of this estimate.
    assert!(estimated.storage_fee >= applied.storage_fee);
    assert!(estimated.processing_fee >= applied.processing_fee);
}

#[test]
fn should_cover_applied_delete_fees_for_a_history_tree_node_with_two_children() {
    use crate::drive::document::paths::contract_document_type_path_vec;
    use grovedb::Element;

    let version = latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = json_document_to_contract(FAMILY_HISTORY_CONTRACT, false, version).unwrap();
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .unwrap();
    let document_type = document_type_of(&contract);
    let owner = [7; 32];
    let mut document =
        json_document_to_document(PERSON, Some(owner.into()), document_type, version).unwrap();
    for id in [1, 2, 3] {
        document.set_id([id; 32].into());
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentInfo::DocumentRefInfo((
                            &document,
                            Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(owner)))),
                        )),
                        owner_id: Some(owner),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default_with_time(1000),
                true,
                None,
                version,
            )
            .unwrap();
    }
    let type_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "person");
    let history_root = drive
        .grove
        .get_raw(
            type_path.as_slice().into(),
            &[2],
            None,
            &version.drive.grove_version,
        )
        .value
        .unwrap();
    // With exactly three document ids, the middle root has both a left and a right child.
    assert!(matches!(history_root, Element::Tree(Some(key), _) if key == vec![2; 32]));
    assert_delete_estimate_covers_applied(
        &drive,
        &contract,
        [2; 32].into(),
        BlockInfo::default_with_time(5000),
        "two-children-history-node",
    );
}

#[test]
fn should_cover_applied_delete_fees_for_migrated_history_with_multi_epoch_contract_flags() {
    use dpp::block::epoch::Epoch;
    use dpp::data_contract::conversion::json::DataContractJsonConversionMethodsV0;
    use dpp::tests::json_document::json_document_to_json_value;

    let old = PlatformVersion::get(14).unwrap();
    let version = latest();
    let drive = setup_drive_with_initial_state_structure(None);
    let owner = [7; 32];
    let mut schema = json_document_to_json_value(FAMILY_HISTORY_CONTRACT).unwrap();
    // A deletable contract, so the legacy writer flags the document's history
    // tree with the document's own flags.
    schema["config"] = serde_json::json!({"$formatVersion": "0", "canBeDeleted": true});
    let contract = DataContract::from_json(schema.clone(), false, old).unwrap();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(owner)))),
            None,
            old,
        )
        .unwrap();
    let document_type = document_type_of(&contract);
    let document =
        json_document_to_document(PERSON, Some(owner.into()), document_type, old).unwrap();
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentInfo::DocumentRefInfo((
                        &document,
                        Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(owner)))),
                    )),
                    owner_id: Some(owner),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default_with_time(1000),
            true,
            None,
            old,
        )
        .unwrap();

    // Growing the stored contract in a later epoch gives it multiple storing epochs.
    schema["version"] = 2.into();
    schema["documentSchemas"]["person"]["properties"]["extra"] = serde_json::json!({
        "type": "string", "maxLength": 64, "position": 5
    });
    let contract = DataContract::from_json(schema, false, old).unwrap();
    drive
        .apply_contract(
            &contract,
            BlockInfo {
                epoch: Epoch::new(1).unwrap(),
                ..BlockInfo::default_with_time(2000)
            },
            true,
            Some(Cow::Owned(StorageFlags::new_single_epoch(1, Some(owner)))),
            None,
            old,
        )
        .unwrap();
    let transaction = drive.grove.start_transaction();
    drive
        .migrate_document_history_storage(&transaction, version)
        .unwrap();
    drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .unwrap();
    let mut history_type_path = document_history_path(
        contract.id_ref().as_bytes(),
        "person",
        document.id().as_bytes(),
    );
    history_type_path.pop();
    let tree = drive
        .grove
        .get_raw(
            history_type_path.as_slice().into(),
            document.id().as_bytes(),
            None,
            &version.drive.grove_version,
        )
        .value
        .unwrap();
    assert_eq!(
        tree.get_flags(),
        &StorageFlags::map_to_some_element_flags(Some(&StorageFlags::new_single_epoch(
            0,
            Some(owner)
        ))),
        "the migrated history tree keeps its legacy tree's flags, not the contract's"
    );
    assert_delete_estimate_covers_applied(
        &drive,
        &contract,
        document.id(),
        BlockInfo {
            epoch: Epoch::new(2).unwrap(),
            ..BlockInfo::default_with_time(5000)
        },
        "migrated-multi-epoch-contract-flags",
    );
}

#[cfg(test)]
mod raw_conversion_tests;

#[test]
fn should_refuse_erase_refund_cost_before_lifecycle_activation_even_with_a_chunk_limit() {
    let drive = setup_drive_with_initial_state_structure(None);
    let epoch = Epoch::new(0).unwrap();
    let mut released = PlatformVersion::get(14).unwrap().clone();
    released
        .system_limits
        .max_document_revisions_erased_per_transition = Some(100);
    let result = drive.erase_refund_recipient_cost(&epoch, &released);
    assert!(
        matches!(result, Err(Error::Drive(DriveError::VersionNotActive { method, .. }))
            if method == "erase_refund_recipient_cost"),
        "an erase limit cannot activate an inactive fee method"
    );
}

#[test]
fn should_refuse_inactive_and_unknown_erase_refund_cost_versions() {
    let drive = setup_drive_with_initial_state_structure(None);
    let epoch = Epoch::new(0).unwrap();
    let mut version = latest().clone();
    version
        .drive
        .methods
        .document
        .delete
        .erase_refund_recipient_cost = None;
    assert!(matches!(
        drive.erase_refund_recipient_cost(&epoch, &version),
        Err(Error::Drive(DriveError::VersionNotActive { method, known_versions }))
            if method == "erase_refund_recipient_cost" && known_versions == vec![0]
    ));
    version
        .drive
        .methods
        .document
        .delete
        .erase_refund_recipient_cost = Some(1);
    assert!(matches!(
        drive.erase_refund_recipient_cost(&epoch, &version),
        Err(Error::Drive(DriveError::UnknownVersionMismatch { method, known_versions, received: 1 }))
            if method == "erase_refund_recipient_cost" && known_versions == vec![0]
    ));
    version
        .drive
        .methods
        .document
        .delete
        .erase_refund_recipient_cost = Some(0);
    assert_eq!(
        drive.erase_refund_recipient_cost(&epoch, &version).unwrap(),
        drive.erase_refund_recipient_cost(&epoch, latest()).unwrap()
    );
}
