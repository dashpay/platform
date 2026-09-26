//! Documents whose type declares a `ttl`: their expirations tree entries, their flagless
//! storage, their pricing and the cleanup that deletes them.

use crate::drive::document::expiration::paths::{
    documents_expirations_at_time_path_vec, documents_expirations_path_vec, encode_expiration_time,
};
use crate::drive::document::expiration::pricing::document_expiration_cleanup_fee;
use crate::drive::document::expiration::{DocumentExpirationEntry, RemovedExpiredDocuments};
use crate::drive::document::paths::{
    contract_document_type_path_vec, contract_documents_primary_key_path,
};
use crate::drive::Drive;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContractFactory;
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use grovedb::Element;
use std::borrow::Cow;
use std::collections::BTreeMap;

const OWNER: [u8; 32] = [42; 32];
const START_MS: u64 = 1_700_000_000_000;
const TWO_WEEKS_S: u32 = 1_209_600;

/// A contract with a `note` type expiring after `ttl_seconds` and a `memo` type identical
/// but for the `ttl`, both with two indexes.
fn contract_with_ttl(ttl_seconds: u32) -> DataContract {
    let note = |ttl: Option<u32>| {
        let mut schema = platform_value!({
            "type": "object",
            "documentsMutable": true,
            "properties": {
                "text": { "type": "string", "maxLength": 50, "position": 0 },
            },
            "indices": [
                { "name": "byOwner", "properties": [{ "$ownerId": "asc" }] },
                { "name": "byText", "properties": [{ "text": "asc" }] },
            ],
            "required": ["$createdAt", "text"],
            "additionalProperties": false,
        });
        if let Some(ttl) = ttl {
            schema
                .insert("ttl".to_string(), Value::U32(ttl))
                .expect("expected to set the ttl");
        }
        schema
    };
    DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(
            Identifier::from([9; 32]),
            0,
            platform_value!({ "note": note(Some(ttl_seconds)), "memo": note(None) }),
            None,
            None,
        )
        .expect("contract")
        .data_contract_owned()
}

fn setup(ttl_seconds: u32) -> (Drive, DataContract) {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = contract_with_ttl(ttl_seconds);
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("contract applies");
    (drive, contract)
}

fn note(marker: u8, created_at: u64, text: &str) -> Document {
    let mut id = [marker; 32];
    id[1..9].copy_from_slice(&created_at.to_be_bytes());
    Document::V0(DocumentV0 {
        id: Identifier::from(id),
        owner_id: Identifier::from(OWNER),
        properties: BTreeMap::from([("text".to_string(), Value::Text(text.to_string()))]),
        revision: Some(1),
        created_at: Some(created_at),
        ..Default::default()
    })
}

/// The storage flags a create or replace transition writes a document with: the owner's,
/// in the current epoch.
fn owner_flags() -> Option<Cow<'static, StorageFlags>> {
    Some(Cow::Owned(StorageFlags::new_single_epoch(0, Some(OWNER))))
}

fn block_at(time_ms: u64) -> BlockInfo {
    BlockInfo {
        time_ms,
        ..Default::default()
    }
}

/// Inserts `document` into `document_type_name` at its creation time, the way a create
/// transition does: with the owner's storage flags, which a type with a `ttl` must drop.
fn insert(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    document: &Document,
    apply: bool,
) -> FeeResult {
    insert_at_version(
        drive,
        contract,
        document_type_name,
        document,
        apply,
        PlatformVersion::latest(),
    )
}

/// [`insert`] under a given platform version, a changed fee schedule for one.
fn insert_at_version(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    document: &Document,
    apply: bool,
    platform_version: &PlatformVersion,
) -> FeeResult {
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((document, owner_flags())),
                    owner_id: Some(OWNER),
                },
                contract,
                document_type: contract
                    .document_type_for_name(document_type_name)
                    .expect("document type"),
            },
            false,
            block_at(document.created_at().expect("created at")),
            apply,
            None,
            platform_version,
            None,
        )
        .expect("document inserts")
}

fn entry_element(drive: &Drive, expires_at_ms: u64, document_id: Identifier) -> Option<Element> {
    drive
        .grove_get_raw_optional(
            documents_expirations_at_time_path_vec(expires_at_ms)
                .as_slice()
                .into(),
            document_id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .unwrap_or(None)
}

fn stored_document(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    id: Identifier,
) -> Option<Element> {
    let path =
        contract_documents_primary_key_path(contract.id_ref().as_bytes(), document_type_name);
    drive
        .grove_get_raw_optional(
            (&path).into(),
            id.as_slice(),
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("read")
}

fn expiry_tree_exists(drive: &Drive, expires_at_ms: u64) -> bool {
    drive
        .grove_get_raw_optional(
            documents_expirations_path_vec().as_slice().into(),
            &encode_expiration_time(expires_at_ms),
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("read")
        .is_some()
}

fn remove_expired(drive: &Drive, time_ms: u64, limit: u16) -> RemovedExpiredDocuments {
    let transaction = drive.grove.start_transaction();
    let removed = drive
        .remove_expired_documents(
            &block_at(time_ms),
            limit,
            Some(&transaction),
            PlatformVersion::latest(),
        )
        .expect("cleanup runs");
    drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("commits");
    removed
}

#[test]
fn should_index_a_document_with_a_time_to_live_by_its_expiry() {
    let (drive, contract) = setup(TWO_WEEKS_S);
    let document = note(1, START_MS, "hello");
    insert(&drive, &contract, "note", &document, true);
    let expires_at = START_MS + u64::from(TWO_WEEKS_S) * 1000;

    let entry = entry_element(&drive, expires_at, document.id()).expect("the entry exists");
    let Element::Item(bytes, flags) = entry else {
        panic!("the entry must be an item");
    };
    assert_eq!(flags, None, "the entry carries no storage flags");
    assert_eq!(
        DocumentExpirationEntry::from_bytes(&bytes).expect("decodes"),
        DocumentExpirationEntry {
            contract_id: contract.id(),
            document_type_name: "note".to_string(),
        }
    );

    let platform_version = PlatformVersion::latest();
    let early = drive
        .fetch_expired_documents(expires_at - 1, 128, None, &mut vec![], platform_version)
        .expect("fetch");
    assert!(early.is_empty(), "nothing has expired a ms early");
    let due = drive
        .fetch_expired_documents(expires_at, 128, None, &mut vec![], platform_version)
        .expect("fetch");
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].document_id, document.id());
    assert_eq!(due[0].expires_at_ms, expires_at);

    // A document of the type without a `ttl` gets no entry.
    let memo = note(2, START_MS, "memo");
    insert(&drive, &contract, "memo", &memo, true);
    let due = drive
        .fetch_expired_documents(u64::MAX, 128, None, &mut vec![], platform_version)
        .expect("fetch");
    assert_eq!(due.len(), 1);
}

#[test]
fn should_store_a_document_with_a_time_to_live_without_storage_flags() {
    let (drive, contract) = setup(TWO_WEEKS_S);
    let document = note(1, START_MS, "hello");
    insert(&drive, &contract, "note", &document, true);
    let memo = note(2, START_MS, "hello");
    insert(&drive, &contract, "memo", &memo, true);

    let Some(Element::Item(_, flags)) = stored_document(&drive, &contract, "note", document.id())
    else {
        panic!("the document is stored as an item");
    };
    assert_eq!(flags, None, "the document carries no storage flags");
    let Some(Element::Item(_, memo_flags)) = stored_document(&drive, &contract, "memo", memo.id())
    else {
        panic!("the memo is stored as an item");
    };
    assert!(
        memo_flags.is_some(),
        "a document without a time to live keeps its owner's flags"
    );

    // Its index entries carry none either: the `byText` value tree it created and the
    // reference under it.
    let text_value_path: Vec<Vec<u8>> = {
        let mut path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "note");
        path.push(b"text".to_vec());
        path
    };
    let value_tree = drive
        .grove_get_raw_optional(
            text_value_path.as_slice().into(),
            b"hello",
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &PlatformVersion::latest().drive,
        )
        .expect("read")
        .expect("the value tree exists");
    assert_eq!(value_tree.get_flags(), &None);
}

#[test]
fn should_delete_expired_documents_oldest_first_and_drop_their_trees() {
    let (drive, contract) = setup(TWO_WEEKS_S);
    let ttl_ms = u64::from(TWO_WEEKS_S) * 1000;
    let documents: Vec<Document> = (0..3u64)
        .map(|i| note(1 + i as u8, START_MS + i * 1000, &format!("note {i}")))
        .collect();
    for document in &documents {
        insert(&drive, &contract, "note", document, true);
    }

    // At the second document's expiry the first two go; the third stays.
    let removed = remove_expired(&drive, START_MS + 1000 + ttl_ms, 128);
    assert_eq!(
        removed,
        RemovedExpiredDocuments {
            deleted_documents: 2,
            orphaned_entries: 0,
        }
    );
    assert!(stored_document(&drive, &contract, "note", documents[0].id()).is_none());
    assert!(stored_document(&drive, &contract, "note", documents[1].id()).is_none());
    assert!(stored_document(&drive, &contract, "note", documents[2].id()).is_some());
    assert!(!expiry_tree_exists(&drive, START_MS + ttl_ms));
    assert!(!expiry_tree_exists(&drive, START_MS + 1000 + ttl_ms));
    assert!(expiry_tree_exists(&drive, START_MS + 2000 + ttl_ms));

    // Their index entries went with them: the text values of the deleted notes are gone.
    let mut text_path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "note");
    text_path.push(b"text".to_vec());
    let value = |text: &str| {
        drive
            .grove_get_raw_optional(
                text_path.as_slice().into(),
                text.as_bytes(),
                DirectQueryType::StatefulDirectQuery,
                None,
                &mut vec![],
                &PlatformVersion::latest().drive,
            )
            .expect("read")
    };
    assert!(value("note 0").is_none());
    assert!(value("note 1").is_none());
    assert!(value("note 2").is_some());

    let removed = remove_expired(&drive, START_MS + 2000 + ttl_ms, 128);
    assert_eq!(removed.deleted_documents, 1);
    assert!(!expiry_tree_exists(&drive, START_MS + 2000 + ttl_ms));
}

#[test]
fn should_delete_at_most_the_limit_per_run_and_keep_a_partly_drained_tree() {
    let (drive, contract) = setup(TWO_WEEKS_S);
    let expires_at = START_MS + u64::from(TWO_WEEKS_S) * 1000;
    // Five documents created in one block expire together.
    for i in 0..5u8 {
        insert(
            &drive,
            &contract,
            "note",
            &note(10 + i, START_MS, &format!("same block {i}")),
            true,
        );
    }

    let first = remove_expired(&drive, expires_at, 2);
    assert_eq!(first.deleted_documents, 2);
    assert!(expiry_tree_exists(&drive, expires_at));

    let second = remove_expired(&drive, expires_at, 2);
    assert_eq!(second.deleted_documents, 2);
    assert!(expiry_tree_exists(&drive, expires_at));

    let third = remove_expired(&drive, expires_at, 2);
    assert_eq!(third.deleted_documents, 1);
    assert!(!expiry_tree_exists(&drive, expires_at));

    assert_eq!(
        remove_expired(&drive, expires_at, 2),
        RemovedExpiredDocuments::default()
    );
}

#[test]
fn should_remove_the_entry_when_the_owner_deletes_the_document() {
    let (drive, contract) = setup(TWO_WEEKS_S);
    let expires_at = START_MS + u64::from(TWO_WEEKS_S) * 1000;
    let document = note(1, START_MS, "hello");
    insert(&drive, &contract, "note", &document, true);

    let fee = drive
        .delete_document_for_contract(
            document.id(),
            &contract,
            "note",
            block_at(START_MS + 60_000),
            true,
            None,
            PlatformVersion::latest(),
            None,
        )
        .expect("the owner deletes");
    assert!(
        fee.fee_refunds.0.is_empty(),
        "a document with a time to live refunds nothing"
    );
    assert!(entry_element(&drive, expires_at, document.id()).is_none());
    // Its entry was the last of its expiry time, so the tree of that time went with it.
    assert!(!expiry_tree_exists(&drive, expires_at));
    assert_eq!(
        remove_expired(&drive, expires_at, 128),
        RemovedExpiredDocuments::default()
    );
}

#[test]
fn should_keep_the_tree_of_an_expiry_time_until_its_last_entry_goes() {
    let (drive, contract) = setup(TWO_WEEKS_S);
    let expires_at = START_MS + u64::from(TWO_WEEKS_S) * 1000;
    let first = note(1, START_MS, "first");
    let second = note(2, START_MS, "second");
    insert(&drive, &contract, "note", &first, true);
    insert(&drive, &contract, "note", &second, true);
    let delete = |document: &Document| {
        drive
            .delete_document_for_contract(
                document.id(),
                &contract,
                "note",
                block_at(START_MS + 60_000),
                true,
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("the owner deletes");
    };

    delete(&first);
    assert!(expiry_tree_exists(&drive, expires_at));
    assert!(entry_element(&drive, expires_at, second.id()).is_some());
    delete(&second);
    assert!(!expiry_tree_exists(&drive, expires_at));
}

#[test]
fn should_not_let_documents_deleted_early_delay_the_ones_that_expire() {
    // Each owner deletion takes its expiry time's tree with it, so the documents deleted
    // early leave nothing the cleanup would read ahead of a document that is due.
    let (drive, contract) = setup(TWO_WEEKS_S);
    let ttl_ms = u64::from(TWO_WEEKS_S) * 1000;
    let documents: Vec<Document> = (0..4u64)
        .map(|i| note(1 + i as u8, START_MS + i * 1000, &format!("note {i}")))
        .collect();
    for document in &documents {
        insert(&drive, &contract, "note", document, true);
    }
    for document in &documents[..3] {
        drive
            .delete_document_for_contract(
                document.id(),
                &contract,
                "note",
                block_at(START_MS + 60_000),
                true,
                None,
                PlatformVersion::latest(),
                None,
            )
            .expect("the owner deletes");
    }

    let removed = remove_expired(&drive, START_MS + 3000 + ttl_ms, 1);
    assert_eq!(removed.deleted_documents, 1);
    assert!(stored_document(&drive, &contract, "note", documents[3].id()).is_none());
}

#[test]
fn should_keep_the_expiry_and_the_flagless_storage_when_a_document_is_replaced() {
    let (drive, contract) = setup(TWO_WEEKS_S);
    let expires_at = START_MS + u64::from(TWO_WEEKS_S) * 1000;
    let document = note(1, START_MS, "hello");
    insert(&drive, &contract, "note", &document, true);

    let mut replaced = document.clone();
    replaced.set("text", Value::Text("a longer text than before".to_string()));
    replaced.bump_revision();
    let platform_version = PlatformVersion::latest();
    drive
        .update_document_for_contract(
            &replaced,
            &contract,
            contract.document_type_for_name("note").expect("type"),
            Some(OWNER),
            block_at(START_MS + 3_600_000),
            true,
            owner_flags(),
            None,
            platform_version,
            None,
        )
        .expect("the document is replaced");

    let Some(Element::Item(_, flags)) = stored_document(&drive, &contract, "note", document.id())
    else {
        panic!("the document is stored as an item");
    };
    assert_eq!(flags, None, "a replace does not add storage flags");
    assert!(entry_element(&drive, expires_at, document.id()).is_some());

    let removed = remove_expired(&drive, expires_at, 128);
    assert_eq!(removed.deleted_documents, 1);
    assert!(stored_document(&drive, &contract, "note", document.id()).is_none());
}

#[test]
fn should_delete_an_expired_document_its_owner_may_not_delete() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    let contract = DataContractFactory::new(platform_version.protocol_version)
        .expect("factory")
        .create_with_value_config(
            Identifier::from([9; 32]),
            0,
            platform_value!({ "note": {
                "type": "object",
                "canBeDeleted": false,
                "documentsMutable": false,
                "ttl": 3600,
                "properties": { "text": { "type": "string", "maxLength": 50, "position": 0 } },
                "required": ["$createdAt", "text"],
                "additionalProperties": false,
            }}),
            None,
            None,
        )
        .expect("contract")
        .data_contract_owned();
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("contract applies");
    let mut document = note(1, START_MS, "permanent to its owner");
    document.set_revision(None);
    insert(&drive, &contract, "note", &document, true);

    let removed = remove_expired(&drive, START_MS + 3_600_000, 128);
    assert_eq!(removed.deleted_documents, 1);
    assert!(stored_document(&drive, &contract, "note", document.id()).is_none());
}

#[test]
fn should_remove_an_entry_whose_document_is_gone() {
    let (drive, contract) = setup(TWO_WEEKS_S);
    let expires_at = START_MS + 5_000;
    // An entry no document stands behind, as a bug could leave: the cleanup must not fail
    // the block, only drop the entry and its tree.
    let transaction = drive.grove.start_transaction();
    let platform_version = PlatformVersion::latest();
    let mut operations = vec![];
    drive
        .add_document_expiration_operations(
            Some([77; 32]),
            &DocumentExpirationEntry {
                contract_id: contract.id(),
                document_type_name: "note".to_string(),
            },
            expires_at,
            &mut None,
            &mut None,
            Some(&transaction),
            &mut operations,
            platform_version,
        )
        .expect("queued");
    drive
        .apply_batch_low_level_drive_operations(
            None,
            Some(&transaction),
            operations,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("applied");
    drive
        .grove
        .commit_transaction(transaction)
        .unwrap()
        .expect("commits");

    assert_eq!(
        remove_expired(&drive, expires_at, 128),
        RemovedExpiredDocuments {
            deleted_documents: 0,
            orphaned_entries: 1,
        }
    );
    assert!(!expiry_tree_exists(&drive, expires_at));
}

#[test]
fn should_price_a_short_lived_document_into_processing_and_prepay_its_deletion() {
    let platform_version = PlatformVersion::latest();
    let (drive, contract) = setup(3_600);
    let note_fee = insert(&drive, &contract, "note", &note(1, START_MS, "hello"), true);
    let memo_fee = insert(&drive, &contract, "memo", &note(2, START_MS, "hello"), true);

    assert_eq!(
        note_fee.storage_fee, 0,
        "a lifetime under two epochs pays nothing into the storage pool"
    );
    assert!(memo_fee.storage_fee > 0);
    let cleanup_fee = document_expiration_cleanup_fee(
        contract.document_type_for_name("note").expect("type"),
        &platform_version.fee_version,
    )
    .expect("fee");
    assert_eq!(
        cleanup_fee,
        platform_version
            .fee_version
            .document_ttl
            .cleanup_base_processing_cost
            + 2 * platform_version
                .fee_version
                .document_ttl
                .cleanup_processing_cost_per_index_level,
        "two single-property indexes are two index levels"
    );
    // The same create under a schedule whose deletion costs nothing: the difference is the
    // prepaid deletion, exactly.
    let mut free_deletion = platform_version.clone();
    free_deletion
        .fee_version
        .document_ttl
        .cleanup_base_processing_cost = 0;
    free_deletion
        .fee_version
        .document_ttl
        .cleanup_processing_cost_per_index_level = 0;
    let (other_drive, other_contract) = setup(3_600);
    let without_prepay = insert_at_version(
        &other_drive,
        &other_contract,
        "note",
        &note(1, START_MS, "hello"),
        true,
        &free_deletion,
    );
    assert_eq!(
        note_fee.processing_fee - without_prepay.processing_fee,
        cleanup_fee,
        "the processing fee carries the prepaid deletion"
    );
    assert!(
        note_fee.total_base_fee() < memo_fee.total_base_fee(),
        "an hour of storage costs less than perpetual storage"
    );
}

#[test]
fn should_price_a_long_lived_document_into_the_storage_pool() {
    let (drive, contract) = setup(31_536_000);
    let note_fee = insert(&drive, &contract, "note", &note(1, START_MS, "hello"), true);
    let memo_fee = insert(&drive, &contract, "memo", &note(2, START_MS, "hello"), true);
    let per_period = PlatformVersion::latest()
        .fee_version
        .document_ttl
        .credit_per_byte_per_period;
    // A year of 365 days is exactly 40 pricing periods of 9.125 days.
    assert!(note_fee.storage_fee > 0);
    assert_eq!(note_fee.storage_fee % (40 * per_period), 0);
    assert!(note_fee.storage_fee < memo_fee.storage_fee);
}

#[test]
fn should_estimate_at_least_what_a_document_with_a_time_to_live_costs() {
    for ttl in [3_600, TWO_WEEKS_S, 31_536_000] {
        let (drive, contract) = setup(ttl);
        let document = note(1, START_MS, "hello");
        let estimated = insert(&drive, &contract, "note", &document, false);
        let actual = insert(&drive, &contract, "note", &document, true);
        assert!(
            estimated.storage_fee >= actual.storage_fee,
            "ttl {ttl}: estimated storage {} below actual {}",
            estimated.storage_fee,
            actual.storage_fee
        );
        assert!(
            estimated.total_base_fee() >= actual.total_base_fee(),
            "ttl {ttl}: estimated {} below actual {}",
            estimated.total_base_fee(),
            actual.total_base_fee()
        );
    }
}

#[test]
fn should_estimate_at_least_what_replacing_a_document_with_a_time_to_live_costs() {
    let platform_version = PlatformVersion::latest();
    for ttl in [3_600, TWO_WEEKS_S, 31_536_000] {
        let (drive, contract) = setup(ttl);
        let document = note(1, START_MS, "hello");
        insert(&drive, &contract, "note", &document, true);
        let mut replaced = document.clone();
        replaced.set("text", Value::Text("a longer text than before".to_string()));
        replaced.bump_revision();
        let replace = |apply: bool| {
            drive
                .update_document_for_contract(
                    &replaced,
                    &contract,
                    contract.document_type_for_name("note").expect("type"),
                    Some(OWNER),
                    block_at(START_MS + 60_000),
                    apply,
                    owner_flags(),
                    None,
                    platform_version,
                    None,
                )
                .expect("the replace runs")
        };
        let estimated = replace(false);
        let actual = replace(true);
        assert!(
            estimated.total_base_fee() >= actual.total_base_fee(),
            "ttl {ttl}: estimated {} below actual {}",
            estimated.total_base_fee(),
            actual.total_base_fee()
        );
    }
}
