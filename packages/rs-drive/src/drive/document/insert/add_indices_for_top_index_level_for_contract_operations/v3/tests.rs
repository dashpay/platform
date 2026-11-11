//! An insert of a document whose time range buckets have expired: the
//! document the winner of a contest is, inserted when the contest ends with
//! the `$createdAt` it was submitted with.

use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentPropertyType;
use dpp::data_contract::DataContractFactory;
use dpp::document::{Document, DocumentV0, DocumentV0Getters};
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use grovedb::PendingPrefixDropsReport;
use grovedb_path::SubtreePath;
use std::collections::BTreeMap;

const MINUTE_MS: u64 = 60_000;
const HOUR_MS: u64 = 60 * MINUTE_MS;
/// The start of the bucket the documents of these tests are created in.
const BUCKET_START_MS: u64 = 1_000 * HOUR_MS;
const OWNER: [u8; 32] = [7; 32];

/// A `post` type with a `($createdAt, hashtag)` time range index of two hour
/// buckets, every `step_hours`, that expire four hours after their start.
fn post_contract(step_hours: u64, platform_version: &PlatformVersion) -> DataContract {
    DataContractFactory::new(platform_version.protocol_version)
        .expect("factory")
        .create_with_value_config(
            Identifier::from([252; 32]),
            0,
            platform_value!({"post": {
                "type": "object",
                "properties": {"hashtag": {"type": "string", "maxLength": 61, "position": 0}},
                "required": ["hashtag", "$createdAt"],
                "indices": [{"name": "recent",
                    "properties": [{"$createdAt": "asc"}, {"hashtag": "asc"}],
                    "timeRange": {"on": "$createdAt", "range": 7200, "step": step_hours * 3600,
                        "ttl": 14400}}],
                "additionalProperties": false
            }}),
            None,
            None,
        )
        .expect("contract")
        .data_contract_owned()
}

fn setup(step_hours: u64, platform_version: &PlatformVersion) -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = post_contract(step_hours, platform_version);
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    (drive, contract)
}

fn post(marker: u8, created_at: u64, hashtag: &str) -> Document {
    let mut id = [0u8; 32];
    id[0] = marker;
    id[1..9].copy_from_slice(&created_at.to_be_bytes());
    for (byte, tag_byte) in id[9..].iter_mut().zip(hashtag.bytes()) {
        *byte = tag_byte;
    }
    Document::V0(DocumentV0 {
        id: Identifier::from(id),
        owner_id: Identifier::from(OWNER),
        properties: BTreeMap::from([("hashtag".to_string(), Value::Text(hashtag.to_string()))]),
        created_at: Some(created_at),
        revision: Some(1),
        ..Default::default()
    })
}

/// Inserts `document` in a block of its own at `block_time_ms`, then
/// reclaims the storage of what the block dropped, as the end of a block
/// does.
fn insert_in_a_block(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    block_time_ms: u64,
    platform_version: &PlatformVersion,
) -> Result<PendingPrefixDropsReport, Error> {
    let transaction = drive.grove.start_transaction();
    drive.add_document_for_contract(
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((document, StorageFlags::optional_default_as_cow())),
                owner_id: Some(OWNER),
            },
            contract,
            document_type: contract.document_type_for_name("post").expect("post"),
        },
        false,
        BlockInfo {
            time_ms: block_time_ms,
            ..Default::default()
        },
        true,
        Some(&transaction),
        platform_version,
    )?;
    drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("commit");
    Ok(drive
        .grove
        .flush_pending_prefix_drops(&platform_version.drive.grove_version)
        .expect("flush"))
}

fn delete_in_a_block(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    block_time_ms: u64,
    platform_version: &PlatformVersion,
) -> Result<(), Error> {
    drive
        .delete_document_for_contract(
            document.id(),
            contract,
            "post",
            BlockInfo {
                time_ms: block_time_ms,
                ..Default::default()
            },
            true,
            None,
            platform_version,
        )
        .map(|_| ())
}

/// The path of the bucket starting at `bucket_start_ms`, then `below` it.
fn bucket_path(contract: &DataContract, bucket_start_ms: u64, below: &[&[u8]]) -> Vec<Vec<u8>> {
    let document_type = contract.document_type_for_name("post").expect("post");
    let transform = document_type
        .indexes()
        .get("recent")
        .expect("recent")
        .time_range
        .clone()
        .expect("time range");
    let mut path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "post");
    path.push(transform.storage_key("$createdAt").into_bytes());
    path.push(DocumentPropertyType::encode_date_timestamp(bucket_start_ms));
    path.extend(below.iter().map(|segment| segment.to_vec()));
    path
}

/// Whether the element at `path` is in the state the root hash commits to.
fn exists(drive: &Drive, path: &[Vec<u8>], platform_version: &PlatformVersion) -> bool {
    let (key, parent) = path.split_last().expect("a path");
    let parent: Vec<&[u8]> = parent.iter().map(|segment| segment.as_slice()).collect();
    let mut operations: Vec<LowLevelDriveOperation> = vec![];
    drive
        .grove_get_raw_optional(
            SubtreePath::from(parent.as_slice()),
            key,
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut operations,
            &platform_version.drive,
        )
        .expect("read")
        .is_some()
}

/// An ordinary document, then one six hours later that was created a minute
/// after the first. Nothing was written in between, so the second insert is
/// the write that drops the first document's expired bucket.
fn insert_into_a_bucket_the_same_insert_drops(
    platform_version: &PlatformVersion,
) -> (
    Drive,
    DataContract,
    Document,
    Result<PendingPrefixDropsReport, Error>,
) {
    let (drive, contract) = setup(2, platform_version);
    let created_at = BUCKET_START_MS + 10 * MINUTE_MS;
    insert_in_a_block(
        &drive,
        &contract,
        &post(1, created_at, "alpha"),
        created_at,
        platform_version,
    )
    .expect("an ordinary create");

    let late = post(2, created_at + MINUTE_MS, "alpha");
    let report = insert_in_a_block(
        &drive,
        &contract,
        &late,
        BUCKET_START_MS + 6 * HOUR_MS,
        platform_version,
    );
    (drive, contract, late, report)
}

#[test]
fn should_write_no_entry_in_a_bucket_that_has_expired() {
    let platform_version = PlatformVersion::latest();
    let (drive, contract, late, report) =
        insert_into_a_bucket_the_same_insert_drops(platform_version);

    // Everything the block dropped is reclaimed: none of it was written again.
    let report = report.expect("the insert");
    assert_eq!(report.skipped_live, 0);
    assert!(report.reclaimed_records > 0);
    assert!(!exists(
        &drive,
        &bucket_path(&contract, BUCKET_START_MS, &[]),
        platform_version
    ));

    // The document itself is stored, and is deleted as any other.
    delete_in_a_block(
        &drive,
        &contract,
        &late,
        BUCKET_START_MS + 7 * HOUR_MS,
        platform_version,
    )
    .expect("the document is stored");
}

#[test]
fn should_write_every_bucket_at_protocol_version_14() {
    let platform_version = PlatformVersion::get(14).expect("protocol version 14");
    let (drive, contract, late, report) =
        insert_into_a_bucket_the_same_insert_drops(platform_version);

    report.expect("the insert");
    assert!(exists(
        &drive,
        &bucket_path(
            &contract,
            BUCKET_START_MS,
            &[b"hashtag", b"alpha", &[0], late.id().as_slice()]
        ),
        platform_version
    ));
}

#[test]
fn should_keep_the_entries_of_the_buckets_still_live() {
    let platform_version = PlatformVersion::latest();
    // Two hour buckets every hour: a document is in two of them.
    let (drive, contract) = setup(1, platform_version);
    let created_at = BUCKET_START_MS + 90 * MINUTE_MS;
    let late = post(1, created_at, "alpha");
    // Four and a half hours after the first bucket started, it has expired
    // and the one that started an hour later has not.
    insert_in_a_block(
        &drive,
        &contract,
        &late,
        BUCKET_START_MS + 4 * HOUR_MS + 30 * MINUTE_MS,
        platform_version,
    )
    .expect("the insert");

    let late_id = late.id();
    let entry: [&[u8]; 4] = [b"hashtag", b"alpha", &[0], late_id.as_slice()];
    assert!(!exists(
        &drive,
        &bucket_path(&contract, BUCKET_START_MS, &[]),
        platform_version
    ));
    assert!(exists(
        &drive,
        &bucket_path(&contract, BUCKET_START_MS + HOUR_MS, &entry),
        platform_version
    ));
}

/// An expired bucket is dropped a bounded number of trees per write, so one
/// with many groups stands for a few writes more. A document inserted late
/// has no entry in it, and its delete finds the bucket standing, with the
/// entry of another document in the group the document's would be in.
#[test]
fn should_delete_a_document_with_no_entry_in_a_standing_expired_bucket() {
    let platform_version = PlatformVersion::latest();
    let (drive, contract) = setup(2, platform_version);
    let created_at = BUCKET_START_MS + 10 * MINUTE_MS;

    // More groups than two writes drop. The drop takes them in key order, so
    // the last one stands the longest.
    let mut hashtags: Vec<String> = (0..48).map(|group| format!("a{group:02}")).collect();
    hashtags.push("zz".to_string());
    let mut early = None;
    for hashtag in &hashtags {
        let document = post(1, created_at, hashtag);
        insert_in_a_block(&drive, &contract, &document, created_at, platform_version)
            .expect("an ordinary create");
        early = Some(document);
    }
    let early = early.expect("a document");

    let late = post(2, created_at + MINUTE_MS, "zz");
    insert_in_a_block(
        &drive,
        &contract,
        &late,
        BUCKET_START_MS + 6 * HOUR_MS,
        platform_version,
    )
    .expect("the insert");

    delete_in_a_block(
        &drive,
        &contract,
        &late,
        BUCKET_START_MS + 6 * HOUR_MS + MINUTE_MS,
        platform_version,
    )
    .expect("the delete");

    // The bucket stood while the delete ran, and the other document's entry
    // is still in it.
    let early_id = early.id();
    assert!(exists(
        &drive,
        &bucket_path(
            &contract,
            BUCKET_START_MS,
            &[b"hashtag", b"zz", &[0], early_id.as_slice()]
        ),
        platform_version
    ));

    // And that document is deleted as any other.
    delete_in_a_block(
        &drive,
        &contract,
        &early,
        BUCKET_START_MS + 6 * HOUR_MS + 2 * MINUTE_MS,
        platform_version,
    )
    .expect("the delete of the other document");
}
