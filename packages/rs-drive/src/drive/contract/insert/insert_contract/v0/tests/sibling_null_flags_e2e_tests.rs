//! End-to-end coverage for the **null flags of the index-level walkers**
//! (protocol version 14): whether a unique index's entry is the bare
//! reference at `[0]` or a `[0]` tree holding the reference by id, and
//! whether a `nullSearchable: false` index skips a document, depend on that
//! index's own values only, never on a sibling sub-level's.
//!
//! Fixture `tests/supporting_files/contract/sibling-nulls/sibling-nulls-contract.json`:
//!
//! - `listing`: the optional `category` of `byOwnerCategory` sorts before the
//!   `slug` of the unique `byOwnerSlug` under the owner's value tree;
//! - `item`: the unique `byOwnerKey` beside `byOwnerUpdated` on an
//!   `$updatedAt` the type does not require, so it is always missing;
//! - `profile`: `byCityZip` (`nullSearchable: false`) beside `byCityName`;
//! - `offer`: the unique `byOwnerSkuSize` ends two levels below the branch
//!   where `category` is missing;
//! - `badge`: the unique `byOwnerCodeVariant` with an optional `variant`.
//!
//! Earlier protocol versions wrote entries in other layouts: an insert
//! carried a missing value's flag from one sibling sub-level into the next,
//! and a replace (update v0) wrote a unique index's entry as the bare
//! reference unless all of its values were missing. The tests written under
//! protocol version 13 check that a delete and a replace at 14 still find
//! each of those entries where it is stored.

use super::index_only_e2e_tests::{
    assert_grovedb_is_consistent, platform_version, read_grove_element,
};
use super::skip_if_absent_e2e_tests::{
    delete_document, doctype_path, insert_document, keys_under, replace_document, setup, snapshot,
    stored_document_with, with_key, OWNER_1,
};
use crate::drive::Drive;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use grovedb::batch::GroveOp;

const SIBLINGS: &str = "tests/supporting_files/contract/sibling-nulls/sibling-nulls-contract.json";

fn earlier() -> &'static PlatformVersion {
    PlatformVersion::get(13).expect("protocol version 13 exists")
}

fn text(value: &str) -> Value {
    Value::Text(value.to_string())
}

fn listing(
    contract: &DataContract,
    id: [u8; 32],
    category: Option<&str>,
    slug: &str,
    price: u64,
) -> Document {
    let mut properties = vec![("slug", text(slug)), ("price", Value::U64(price))];
    if let Some(category) = category {
        properties.push(("category", text(category)));
    }
    stored_document_with(contract, "listing", id, properties, None)
}

/// `<doctype>/$ownerId/<OWNER_1>` followed by each `(property, value)`.
fn owner_value_path(
    contract: &DataContract,
    doctype: &str,
    levels: &[(&str, &[u8])],
) -> Vec<Vec<u8>> {
    let owner = with_key(&doctype_path(contract, doctype), b"$ownerId");
    levels
        .iter()
        .fold(with_key(&owner, &OWNER_1), |path, (property, value)| {
            with_key(&with_key(&path, property.as_bytes()), value)
        })
}

/// Whether `operations` delete the element at `key` under `path`.
fn deletes(operations: &[LowLevelDriveOperation], path: &[Vec<u8>], key: &[u8]) -> bool {
    let batch = LowLevelDriveOperation::grovedb_operations_batch(operations);
    matches!(
        batch.contains(path.iter().map(Vec::as_slice), key),
        Some(GroveOp::Delete | GroveOp::DeleteDontCheckForBackwardsReferences)
    )
}

fn delete_operations(
    drive: &Drive,
    contract: &DataContract,
    doctype: &str,
    id: [u8; 32],
) -> Vec<LowLevelDriveOperation> {
    drive
        .delete_document_for_contract_operations(
            Identifier::from(id),
            contract,
            contract
                .document_type_for_name(doctype)
                .expect("doctype exists"),
            None,
            &mut None,
            0,
            None,
            platform_version(),
        )
        .expect("expected the delete operations")
}

fn replace_operations(
    drive: &Drive,
    contract: &DataContract,
    doctype: &str,
    document: &Document,
) -> Vec<LowLevelDriveOperation> {
    drive
        .update_document_for_contract_operations(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(document.owner_id().to_buffer()),
                },
                contract,
                document_type: contract
                    .document_type_for_name(doctype)
                    .expect("doctype exists"),
            },
            &BlockInfo::default(),
            &mut None,
            &mut None,
            None,
            platform_version(),
        )
        .expect("expected the replace operations")
}

#[test]
fn should_write_a_bare_unique_reference_when_only_a_sibling_value_is_missing() {
    let (drive, contract) = setup(SIBLINGS);
    insert_document(
        &drive,
        &contract,
        "listing",
        &listing(&contract, [1; 32], None, "my-bike", 5),
        true,
        platform_version(),
    )
    .expect("insert a listing without a category");

    let slug = read_grove_element(
        &drive,
        &owner_value_path(&contract, "listing", &[("slug", b"my-bike")]),
        &[0],
    )
    .expect("the slug entry");
    assert!(
        !slug.is_any_tree(),
        "byOwnerSlug's own values are all present: its entry is the bare reference at [0], got {slug:?}"
    );
    let category = read_grove_element(
        &drive,
        &owner_value_path(&contract, "listing", &[("category", &[])]),
        &[0],
    )
    .expect("the category entry");
    assert!(
        category.is_any_tree(),
        "byOwnerCategory is not unique: its entries sit in the [0] tree"
    );
    assert_grovedb_is_consistent(&drive);
}

#[test]
fn should_write_a_bare_unique_reference_beside_an_unrequired_timestamp_index() {
    let (drive, contract) = setup(SIBLINGS);
    let key = [7u8; 32];
    insert_document(
        &drive,
        &contract,
        "item",
        &stored_document_with(
            &contract,
            "item",
            [2; 32],
            vec![("key", Value::Bytes(key.to_vec()))],
            None,
        ),
        true,
        platform_version(),
    )
    .expect("insert an item");

    let entry = read_grove_element(
        &drive,
        &owner_value_path(&contract, "item", &[("key", &key)]),
        &[0],
    )
    .expect("the key entry");
    assert!(
        !entry.is_any_tree(),
        "byOwnerKey's own values are all present, though the $updatedAt sibling is missing: got {entry:?}"
    );
    assert_grovedb_is_consistent(&drive);
}

/// A replace and a delete of a listing without a category find its unique
/// entry where the insert put it.
#[test]
fn should_replace_and_delete_a_listing_missing_its_sibling_value() {
    let (drive, contract) = setup(SIBLINGS);
    let path = doctype_path(&contract, "listing");
    let before = snapshot(&drive, path.clone(), false);

    insert_document(
        &drive,
        &contract,
        "listing",
        &listing(&contract, [3; 32], None, "my-bike", 5),
        true,
        platform_version(),
    )
    .expect("insert");
    let mut repriced = listing(&contract, [3; 32], None, "my-bike", 9);
    repriced.set_revision(Some(2));
    replace_document(
        &drive,
        &contract,
        "listing",
        &repriced,
        true,
        platform_version(),
    )
    .expect("the replace finds the unique entry where the insert put it");

    let (fresh_drive, fresh_contract) = setup(SIBLINGS);
    insert_document(
        &fresh_drive,
        &fresh_contract,
        "listing",
        &repriced,
        true,
        platform_version(),
    )
    .expect("insert the new version");
    assert_eq!(
        snapshot(&drive, path.clone(), true),
        snapshot(&fresh_drive, doctype_path(&fresh_contract, "listing"), true),
        "the replace left other index trees than a fresh insert"
    );

    delete_document(&drive, &contract, "listing", [3; 32]);
    assert_eq!(snapshot(&drive, path, false), before);
    assert_grovedb_is_consistent(&drive);
}

#[test]
fn should_skip_a_not_null_searchable_entry_whose_own_values_are_all_missing() {
    let (drive, contract) = setup(SIBLINGS);
    let path = doctype_path(&contract, "profile");

    // No city and no zip, but a name: byCityName's name sorts before
    // byCityZip's zip under the city's (null) value tree.
    insert_document(
        &drive,
        &contract,
        "profile",
        &stored_document_with(
            &contract,
            "profile",
            [4; 32],
            vec![("name", text("ada"))],
            None,
        ),
        true,
        platform_version(),
    )
    .expect("insert a profile with only a name");
    let zip = with_key(
        &with_key(&with_key(&with_key(&path, b"city"), &[]), b"zip"),
        &[],
    );
    assert!(
        keys_under(&drive, &zip).is_empty(),
        "byCityZip's values are all missing and it is not null-searchable: no entry"
    );

    // The value trees above a skipped `nullSearchable: false` entry are
    // written and, the entry being absent, not pruned by the delete (the
    // layout's `notNullSearchable` note); no entry of the profile is left.
    delete_document(&drive, &contract, "profile", [4; 32]);
    assert!(
        snapshot(&drive, path, false)
            .iter()
            .all(|(_, _, kind, _)| kind != "reference"),
        "the delete left an entry of the profile"
    );
    assert_grovedb_is_consistent(&drive);
}

/// Inserts `document` under protocol version 13, checks where its unique
/// entry went (`entry_value_path`, a `[0]` tree by the carried flag), then
/// deletes it at 14: the delete removes the reference inside that tree (not
/// `[0]` as a lone element, which would leave the reference in storage,
/// unreachable, where no walk of the tree can see it) and the prior tree is
/// restored.
fn assert_earlier_tree_entry_deletes(doctype: &str, document: Document, levels: &[(&str, &[u8])]) {
    let (drive, contract) = setup(SIBLINGS);
    let path = doctype_path(&contract, doctype);
    let before = snapshot(&drive, path.clone(), false);
    let id = document.id().to_buffer();

    insert_document(&drive, &contract, doctype, &document, true, earlier())
        .expect("insert under protocol version 13");
    let entry_value_path = owner_value_path(&contract, doctype, levels);
    assert!(
        read_grove_element(&drive, &entry_value_path, &[0])
            .expect("the entry")
            .is_any_tree(),
        "{doctype}: protocol version 13 wrote the [0] tree layout"
    );

    assert!(
        deletes(
            &delete_operations(&drive, &contract, doctype, id),
            &with_key(&entry_value_path, &[0]),
            &id
        ),
        "{doctype}: the delete must remove the reference inside the [0] tree"
    );
    delete_document(&drive, &contract, doctype, id);
    assert_eq!(snapshot(&drive, path, false), before, "{doctype}");
    assert_grovedb_is_consistent(&drive);
}

#[test]
fn should_delete_a_listing_written_under_the_earlier_sibling_rule() {
    let (_, contract) = setup(SIBLINGS);
    assert_earlier_tree_entry_deletes(
        "listing",
        listing(&contract, [5; 32], None, "my-bike", 5),
        &[("slug", b"my-bike")],
    );
}

#[test]
fn should_delete_an_item_written_under_the_earlier_sibling_rule() {
    let (_, contract) = setup(SIBLINGS);
    let key = [8u8; 32];
    assert_earlier_tree_entry_deletes(
        "item",
        stored_document_with(
            &contract,
            "item",
            [6; 32],
            vec![("key", Value::Bytes(key.to_vec()))],
            None,
        ),
        &[("key", &key)],
    );
}

/// The carried flag crosses a level: `category` is missing under the
/// owner's value tree, byOwnerSkuSize ends two levels further down.
#[test]
fn should_delete_an_offer_whose_earlier_flag_crossed_a_level() {
    let (_, contract) = setup(SIBLINGS);
    assert_earlier_tree_entry_deletes(
        "offer",
        stored_document_with(
            &contract,
            "offer",
            [7; 32],
            vec![("sku", text("s-1")), ("size", text("m"))],
            None,
        ),
        &[("sku", b"s-1"), ("size", b"m")],
    );
}

/// Before protocol version 14 a present `name` carried "not all null" into
/// byCityZip's branch, which then got an entry for a profile missing both of
/// its values; the delete at 14 still removes it and prunes the branch.
#[test]
fn should_delete_a_profile_entry_written_under_the_earlier_sibling_rule() {
    let (drive, contract) = setup(SIBLINGS);
    let path = doctype_path(&contract, "profile");
    let before = snapshot(&drive, path.clone(), false);

    insert_document(
        &drive,
        &contract,
        "profile",
        &stored_document_with(
            &contract,
            "profile",
            [9; 32],
            vec![("name", text("ada"))],
            None,
        ),
        true,
        earlier(),
    )
    .expect("insert under protocol version 13");
    let zip = with_key(
        &with_key(&with_key(&with_key(&path, b"city"), &[]), b"zip"),
        &[],
    );
    assert!(
        read_grove_element(&drive, &with_key(&zip, &[0]), &[9; 32]).is_some(),
        "protocol version 13 wrote byCityZip's entry"
    );

    delete_document(&drive, &contract, "profile", [9; 32]);
    assert_eq!(snapshot(&drive, path, false), before);
    assert_grovedb_is_consistent(&drive);
}

/// A replace at 14 that moves a listing written the earlier way to another
/// slug removes the old reference inside its `[0]` tree and leaves the tree
/// a fresh insert of the new version would.
#[test]
fn should_replace_a_listing_written_under_the_earlier_sibling_rule() {
    let (drive, contract) = setup(SIBLINGS);
    insert_document(
        &drive,
        &contract,
        "listing",
        &listing(&contract, [10; 32], None, "my-bike", 5),
        true,
        earlier(),
    )
    .expect("insert under protocol version 13");

    let mut moved = listing(&contract, [10; 32], None, "my-car", 5);
    moved.set_revision(Some(2));
    assert!(
        deletes(
            &replace_operations(&drive, &contract, "listing", &moved),
            &with_key(
                &owner_value_path(&contract, "listing", &[("slug", b"my-bike")]),
                &[0]
            ),
            &[10; 32]
        ),
        "the replace must remove the old reference inside the [0] tree"
    );
    replace_document(
        &drive,
        &contract,
        "listing",
        &moved,
        true,
        platform_version(),
    )
    .expect("replace at protocol version 14");

    let (fresh_drive, fresh_contract) = setup(SIBLINGS);
    insert_document(
        &fresh_drive,
        &fresh_contract,
        "listing",
        &moved,
        true,
        platform_version(),
    )
    .expect("insert the new version");
    assert_eq!(
        snapshot(&drive, doctype_path(&contract, "listing"), true),
        snapshot(&fresh_drive, doctype_path(&fresh_contract, "listing"), true),
    );
    assert_grovedb_is_consistent(&drive);
}

/// A replace at 14 that keeps the slug of a listing written the earlier way
/// refreshes the reference inside its `[0]` tree, which stays a tree.
#[test]
fn should_refresh_a_listing_written_under_the_earlier_sibling_rule_in_place() {
    let (drive, contract) = setup(SIBLINGS);
    let path = doctype_path(&contract, "listing");
    let before = snapshot(&drive, path.clone(), false);
    insert_document(
        &drive,
        &contract,
        "listing",
        &listing(&contract, [11; 32], None, "my-bike", 5),
        true,
        earlier(),
    )
    .expect("insert under protocol version 13");

    let mut repriced = listing(&contract, [11; 32], None, "my-bike", 9);
    repriced.set_revision(Some(2));
    replace_document(
        &drive,
        &contract,
        "listing",
        &repriced,
        true,
        platform_version(),
    )
    .expect("replace at protocol version 14");
    let slug = owner_value_path(&contract, "listing", &[("slug", b"my-bike")]);
    assert!(
        read_grove_element(&drive, &with_key(&slug, &[0]), &[11; 32]).is_some(),
        "the reference is still inside the [0] tree"
    );

    delete_document(&drive, &contract, "listing", [11; 32]);
    assert_eq!(snapshot(&drive, path, false), before);
    assert_grovedb_is_consistent(&drive);
}

/// Under protocol version 13 a replace dropping `variant` wrote byOwnerCodeVariant's
/// entry as the bare reference, though a value is missing.
fn badge_replaced_under_update_v0() -> (Drive, DataContract, Vec<Vec<u8>>) {
    let (drive, contract) = setup(SIBLINGS);
    let path = doctype_path(&contract, "badge");
    insert_document(
        &drive,
        &contract,
        "badge",
        &stored_document_with(
            &contract,
            "badge",
            [12; 32],
            vec![("code", text("a")), ("variant", text("x"))],
            None,
        ),
        true,
        earlier(),
    )
    .expect("insert under protocol version 13");
    let mut dropped = stored_document_with(
        &contract,
        "badge",
        [12; 32],
        vec![("code", text("a"))],
        None,
    );
    dropped.set_revision(Some(2));
    replace_document(&drive, &contract, "badge", &dropped, true, earlier())
        .expect("replace under protocol version 13");
    assert!(
        !read_grove_element(
            &drive,
            &owner_value_path(&contract, "badge", &[("code", b"a"), ("variant", &[])]),
            &[0]
        )
        .expect("the entry")
        .is_any_tree(),
        "protocol version 13's replace wrote the bare reference"
    );
    (drive, contract, path)
}

#[test]
fn should_delete_a_badge_whose_earlier_replace_wrote_the_unique_layout() {
    let (drive, contract, path) = badge_replaced_under_update_v0();
    delete_document(&drive, &contract, "badge", [12; 32]);
    let (fresh_drive, fresh_contract) = setup(SIBLINGS);
    assert_eq!(
        snapshot(&drive, path, false),
        snapshot(&fresh_drive, doctype_path(&fresh_contract, "badge"), false),
    );
    assert_grovedb_is_consistent(&drive);
}

#[test]
fn should_replace_a_badge_whose_earlier_replace_wrote_the_unique_layout() {
    let (drive, contract, path) = badge_replaced_under_update_v0();
    let mut recoded = stored_document_with(
        &contract,
        "badge",
        [12; 32],
        vec![("code", text("b"))],
        None,
    );
    recoded.set_revision(Some(3));
    replace_document(
        &drive,
        &contract,
        "badge",
        &recoded,
        true,
        platform_version(),
    )
    .expect("replace at protocol version 14");

    let (fresh_drive, fresh_contract) = setup(SIBLINGS);
    insert_document(
        &fresh_drive,
        &fresh_contract,
        "badge",
        &recoded,
        true,
        platform_version(),
    )
    .expect("insert the new version");
    assert_eq!(
        snapshot(&drive, path, true),
        snapshot(&fresh_drive, doctype_path(&fresh_contract, "badge"), true),
    );
    assert_grovedb_is_consistent(&drive);
}
