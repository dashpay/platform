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
//! - `profile`: `byCityZip` (`nullSearchable: false`) beside `byCityName`.
//!
//! Before protocol version 14 a walker carried a missing value's flag from
//! one sibling sub-level into the next; a delete still removes an entry
//! written that way.

use super::index_only_e2e_tests::{assert_grovedb_is_consistent, platform_version};
use super::skip_if_absent_e2e_tests::{
    doctype_path, keys_under, replace_document, setup, snapshot, with_key,
};
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use grovedb::batch::GroveOp;
use grovedb::Element;
use std::collections::BTreeMap;

const SIBLINGS: &str = "tests/supporting_files/contract/sibling-nulls/sibling-nulls-contract.json";

const OWNER: [u8; 32] = [0x11; 32];

/// A document of `doctype` carrying `properties`, owned by `OWNER`.
fn document(
    contract: &DataContract,
    doctype: &str,
    id: [u8; 32],
    properties: Vec<(&str, Value)>,
) -> Document {
    let mut document = contract
        .document_type_for_name(doctype)
        .expect("doctype exists")
        .random_document(Some(id[0] as u64), platform_version())
        .expect("random document");
    document.set_properties(
        properties
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect::<BTreeMap<_, _>>(),
    );
    document.set_id(Identifier::from(id));
    document.set_owner_id(Identifier::from(OWNER));
    document.set_created_at(None);
    document.set_updated_at(None);
    document.set_revision(Some(1));
    document
}

fn listing(contract: &DataContract, id: [u8; 32], category: Option<&str>, price: u64) -> Document {
    let mut properties = vec![
        ("slug", Value::Text("my-bike".to_string())),
        ("price", Value::U64(price)),
    ];
    if let Some(category) = category {
        properties.push(("category", Value::Text(category.to_string())));
    }
    document(contract, "listing", id, properties)
}

fn insert_at(
    drive: &Drive,
    contract: &DataContract,
    doctype: &str,
    document: &Document,
    platform_version: &PlatformVersion,
) -> Result<FeeResult, Error> {
    drive.add_document_for_contract(
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((document, StorageFlags::optional_default_as_cow())),
                owner_id: None,
            },
            contract,
            document_type: contract
                .document_type_for_name(doctype)
                .expect("doctype exists"),
        },
        false,
        BlockInfo::default(),
        true,
        None,
        platform_version,
        None,
    )
}

fn delete(drive: &Drive, contract: &DataContract, doctype: &str, id: [u8; 32]) {
    drive
        .delete_document_for_contract(
            Identifier::from(id),
            contract,
            doctype,
            BlockInfo::default(),
            true,
            None,
            platform_version(),
            None,
        )
        .expect("expected to delete the document");
}

/// The element at `key` under `path`.
fn element(drive: &Drive, path: &[Vec<u8>], key: &[u8]) -> Element {
    drive
        .grove
        .get_raw(
            path.iter()
                .map(Vec::as_slice)
                .collect::<Vec<_>>()
                .as_slice()
                .into(),
            key,
            None,
            &platform_version().drive.grove_version,
        )
        .unwrap()
        .expect("expected the element")
}

/// `<doctype>/$ownerId/<OWNER>/<property>/<value>`.
fn owner_value_path(
    contract: &DataContract,
    doctype: &str,
    property: &str,
    value: &[u8],
) -> Vec<Vec<u8>> {
    let owner = with_key(&doctype_path(contract, doctype), b"$ownerId");
    with_key(
        &with_key(&with_key(&owner, &OWNER), property.as_bytes()),
        value,
    )
}

#[test]
fn should_write_a_bare_unique_reference_when_only_a_sibling_value_is_missing() {
    let (drive, contract) = setup(SIBLINGS);
    insert_at(
        &drive,
        &contract,
        "listing",
        &listing(&contract, [1; 32], None, 5),
        platform_version(),
    )
    .expect("insert a listing without a category");

    let slug = element(
        &drive,
        &owner_value_path(&contract, "listing", "slug", b"my-bike"),
        &[0],
    );
    assert!(
        !slug.is_any_tree(),
        "byOwnerSlug's own values are all present: its entry is the bare reference at [0], got {slug:?}"
    );
    let category = element(
        &drive,
        &owner_value_path(&contract, "listing", "category", &[]),
        &[0],
    );
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
    insert_at(
        &drive,
        &contract,
        "item",
        &document(
            &contract,
            "item",
            [2; 32],
            vec![("key", Value::Bytes(key.to_vec()))],
        ),
        platform_version(),
    )
    .expect("insert an item");

    let entry = element(
        &drive,
        &owner_value_path(&contract, "item", "key", &key),
        &[0],
    );
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

    insert_at(
        &drive,
        &contract,
        "listing",
        &listing(&contract, [3; 32], None, 5),
        platform_version(),
    )
    .expect("insert");
    let mut repriced = listing(&contract, [3; 32], None, 9);
    repriced.set_revision(Some(2));
    replace_document(&drive, &contract, "listing", &repriced, true)
        .expect("the replace finds the unique entry where the insert put it");

    let (fresh_drive, fresh_contract) = setup(SIBLINGS);
    insert_at(
        &fresh_drive,
        &fresh_contract,
        "listing",
        &repriced,
        platform_version(),
    )
    .expect("insert the new version");
    assert_eq!(
        snapshot(&drive, path.clone(), true),
        snapshot(&fresh_drive, doctype_path(&fresh_contract, "listing"), true),
        "the replace left other index trees than a fresh insert"
    );

    delete(&drive, &contract, "listing", [3; 32]);
    assert_eq!(snapshot(&drive, path, false), before);
    assert_grovedb_is_consistent(&drive);
}

#[test]
fn should_skip_a_not_null_searchable_entry_whose_own_values_are_all_missing() {
    let (drive, contract) = setup(SIBLINGS);
    let path = doctype_path(&contract, "profile");

    // No city and no zip, but a name: byCityName's name sorts before
    // byCityZip's zip under the city's (null) value tree.
    insert_at(
        &drive,
        &contract,
        "profile",
        &document(
            &contract,
            "profile",
            [4; 32],
            vec![("name", Value::Text("ada".to_string()))],
        ),
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
    delete(&drive, &contract, "profile", [4; 32]);
    assert!(
        snapshot(&drive, path, false)
            .iter()
            .all(|(_, _, kind, _)| kind != "reference"),
        "the delete left an entry of the profile"
    );
    assert_grovedb_is_consistent(&drive);
}

/// An entry written before protocol version 14, when a sibling's missing
/// value put byOwnerSlug's entry in the `[0]` tree, still deletes.
#[test]
fn should_delete_a_listing_written_under_the_earlier_sibling_rule() {
    let (drive, contract) = setup(SIBLINGS);
    let path = doctype_path(&contract, "listing");
    let before = snapshot(&drive, path.clone(), false);

    let earlier = PlatformVersion::get(13).expect("protocol version 13 exists");
    insert_at(
        &drive,
        &contract,
        "listing",
        &listing(&contract, [5; 32], None, 5),
        earlier,
    )
    .expect("insert under protocol version 13");
    assert!(
        element(
            &drive,
            &owner_value_path(&contract, "listing", "slug", b"my-bike"),
            &[0]
        )
        .is_any_tree(),
        "protocol version 13 wrote the [0] tree layout"
    );

    // The delete removes the reference inside the `[0]` tree. Removing `[0]`
    // as a lone element instead would leave the reference in storage,
    // unreachable, which no walk of the tree can see.
    let entry = with_key(
        &owner_value_path(&contract, "listing", "slug", b"my-bike"),
        &[0],
    );
    let deletes: Vec<(Vec<Vec<u8>>, Vec<u8>)> = drive
        .delete_document_for_contract_operations(
            Identifier::from([5; 32]),
            &contract,
            contract
                .document_type_for_name("listing")
                .expect("listing doctype exists"),
            None,
            &mut None,
            0,
            None,
            platform_version(),
        )
        .expect("expected the delete operations")
        .into_iter()
        .filter_map(|operation| match operation {
            LowLevelDriveOperation::GroveOperation(operation)
                if matches!(
                    operation.op,
                    GroveOp::Delete
                        | GroveOp::DeleteDontCheckForBackwardsReferences
                        | GroveOp::DeleteTree(..)
                        | GroveOp::DeleteTreeDontCheckForBackwardsReferences(..)
                ) =>
            {
                Some((operation.path.to_path(), operation.key?.get_key_clone()))
            }
            _ => None,
        })
        .collect();
    assert!(
        deletes.contains(&(entry, vec![5; 32])),
        "the delete must remove the reference inside the [0] tree: {deletes:?}"
    );

    delete(&drive, &contract, "listing", [5; 32]);
    assert_eq!(snapshot(&drive, path, false), before);
    assert_grovedb_is_consistent(&drive);
}
