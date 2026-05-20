//! End-to-end coverage for the top-level *index* `rangeSummable` /
//! `rangeCountable` 4-way dispatcher in
//! [`Drive::insert_contract_operations_v0`].
//!
//! Mirrors `range_countable_index_e2e_tests` but pins the property-
//! name tree variant at `[contract_doc, doctype, "<prop>"]` for each
//! of the four `(range_countable, range_summable)` corners:
//!
//!   - `( true,  true ) → Element::ProvableCountProvableSumTree`
//!   - `( true, false ) → Element::ProvableCountTree`   (regression)
//!   - `(false,  true ) → Element::ProvableSumTree`     (NEW — was
//!     silently `NormalTree` pre-fix, which broke any
//!     `AggregateSumOnRange` query against a top-level
//!     `rangeSummable` index — Q7 in the sum bench).
//!   - `(false, false ) → Element::Tree` (NormalTree, the existing
//!     unflagged-index baseline).
//!
//! The compound-index walker in
//! `add_indices_for_index_level_for_contract_operations/v0/mod.rs`
//! already gets this right for *nested* levels (see the
//! `property_name_tree_type` 4-way match there); these tests pin
//! that the contract creation dispatcher now matches.
//!
//! These tests target only the contract-setup-time tree shape (does
//! the property-name tree have the right `TreeType`?). End-to-end
//! `AggregateSumOnRange` query coverage lives in the sum bench
//! (`benches/document_sum_worst_case.rs`).
use crate::drive::Drive;
use crate::util::grove_operations::DirectQueryType;
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::DataContractFactory;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::DataContract;
use dpp::tests::utils::generate_random_identifier_struct;
use dpp::version::PlatformVersion;
use grovedb::Element;

const PROTOCOL_VERSION_V12: u32 = 12;

/// Build a v12 contract whose `tip` doctype declares a single-property
/// index over `sentAt` (an integer property) with the requested
/// `(range_countable, range_summable)` corner. Mirrors the
/// production sum-bench `bySentAt` shape but per-test so each corner
/// gets its own minimal contract.
fn build_tip_with_sent_at_index(range_countable: bool, range_summable: bool) -> DataContract {
    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");

    // Build the index map — only attach the flags that this corner
    // is testing. `rangeSummable: true` requires `summable: "amount"`
    // (enforced by the DPP validator); `rangeCountable: true`
    // requires `countable: "countable"` likewise.
    let mut index_map = vec![
        (
            Value::Text("name".to_string()),
            Value::Text("bySentAt".to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(vec![platform_value!({"sentAt": "asc"})]),
        ),
    ];
    if range_countable {
        index_map.push((
            Value::Text("countable".to_string()),
            Value::Text("countable".to_string()),
        ));
        index_map.push((Value::Text("rangeCountable".to_string()), Value::Bool(true)));
    }
    if range_summable {
        index_map.push((
            Value::Text("summable".to_string()),
            Value::Text("amount".to_string()),
        ));
        index_map.push((Value::Text("rangeSummable".to_string()), Value::Bool(true)));
    }

    let document_schema = platform_value!({
        "type": "object",
        "properties": {
            "sentAt": {"type": "integer", "minimum": 0, "position": 0},
            // `maximum` bounds the property to u32::MAX so DPP infers
            // `U32` (an accepted summable type) rather than the default
            // `U64` (rejected — would overflow grovedb's i64
            // aggregator). Test values stay well under 2^32.
            "amount": {"type": "integer", "minimum": 1, "maximum": 4294967295i64, "position": 1},
        },
        "required": ["sentAt", "amount"],
        "indices": Value::Array(vec![Value::Map(index_map)]),
        "additionalProperties": false,
    });
    let schemas = platform_value!({ "tip": document_schema });
    let owner_id = generate_random_identifier_struct();

    factory
        .create_with_value_config(owner_id, 0, schemas, None, None)
        .expect("expected to create data contract")
        .data_contract_owned()
}

/// Returns the parent path and key needed to fetch the property-name
/// tree element at `[contract_doc, doctype, "<prop>"]` from grove —
/// i.e. `parent = [..., doctype]`, `key = "<prop>"`. The element at
/// that path is the property-name tree under test.
fn property_name_tree_parent_and_key(
    contract: &DataContract,
    document_type_name: &str,
    property_name: &str,
) -> (Vec<Vec<u8>>, Vec<u8>) {
    (
        vec![
            vec![crate::drive::RootTree::DataContractDocuments as u8],
            contract.id().as_bytes().to_vec(),
            vec![1],
            document_type_name.as_bytes().to_vec(),
        ],
        property_name.as_bytes().to_vec(),
    )
}

fn read_grove_element(drive: &Drive, path: &[Vec<u8>], key: &[u8]) -> Element {
    let pv = PlatformVersion::latest();
    let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
    drive
        .grove_get_raw(
            path_refs.as_slice().into(),
            key,
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &pv.drive,
        )
        .expect("grove_get_raw should succeed")
        .expect("property-name tree element must exist")
}

fn apply_contract(drive: &Drive, contract: &DataContract) {
    drive
        .apply_contract(
            contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            PlatformVersion::latest(),
        )
        .expect("expected to apply contract");
}

/// `rangeSummable: true` (no rangeCountable) → property-name tree is
/// `Element::ProvableSumTree`. This is the regression that was
/// triggering "AggregateSumOnRange is only valid against
/// ProvableSumTree or ProvableCountProvableSumTree, got NormalTree"
/// before the dispatcher fix.
#[test]
fn property_name_tree_for_range_summable_index_is_provable_sum_tree() {
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = build_tip_with_sent_at_index(false, true);
    apply_contract(&drive, &contract);

    let (parent, key) = property_name_tree_parent_and_key(&contract, "tip", "sentAt");
    let elem = read_grove_element(&drive, &parent, &key);
    match elem {
        Element::ProvableSumTree(_, sum, _) => {
            assert_eq!(
                sum, 0,
                "freshly created property-name ProvableSumTree should have aggregate sum 0"
            );
        }
        other => panic!(
            "rangeSummable-only top-level index property-name tree should be \
             ProvableSumTree, got {:?}",
            other
        ),
    }
}

/// `rangeCountable: true` AND `rangeSummable: true` → property-name
/// tree is `Element::ProvableCountProvableSumTree`. The combined PCPS
/// surface (grovedb PR 670) carries both per-node counts and per-node
/// sums in one tree.
#[test]
fn property_name_tree_for_range_countable_and_summable_index_is_pcps() {
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = build_tip_with_sent_at_index(true, true);
    apply_contract(&drive, &contract);

    let (parent, key) = property_name_tree_parent_and_key(&contract, "tip", "sentAt");
    let elem = read_grove_element(&drive, &parent, &key);
    match elem {
        Element::ProvableCountProvableSumTree(_, count, sum, _) => {
            assert_eq!(
                count, 0,
                "freshly created PCPS should have aggregate count 0"
            );
            assert_eq!(sum, 0, "freshly created PCPS should have aggregate sum 0");
        }
        other => panic!(
            "(rangeCountable + rangeSummable) top-level index property-name tree should \
             be ProvableCountProvableSumTree, got {:?}",
            other
        ),
    }
}

/// Neither flag → property-name tree is a plain `Element::Tree`
/// (NormalTree). Pins the default unflagged-index path; without
/// this the 4-way match could regress to a flag default and the
/// existing non-aggregating indexes would suddenly emit
/// CountTree/SumTree under the wrong corners.
#[test]
fn property_name_tree_for_unflagged_index_is_normal_tree() {
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = build_tip_with_sent_at_index(false, false);
    apply_contract(&drive, &contract);

    let (parent, key) = property_name_tree_parent_and_key(&contract, "tip", "sentAt");
    let elem = read_grove_element(&drive, &parent, &key);
    match elem {
        Element::Tree(..) => {}
        other => panic!(
            "unflagged top-level index property-name tree should be a plain Tree \
             (NormalTree), got {:?}",
            other
        ),
    }
}

/// `rangeCountable: true` only → property-name tree is
/// `Element::ProvableCountTree`. Regression guard for the existing
/// path the dispatcher already handled correctly pre-fix; this
/// ensures the new 4-way match didn't lose the count-only corner.
#[test]
fn property_name_tree_for_range_countable_only_index_is_provable_count_tree() {
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = build_tip_with_sent_at_index(true, false);
    apply_contract(&drive, &contract);

    let (parent, key) = property_name_tree_parent_and_key(&contract, "tip", "sentAt");
    let elem = read_grove_element(&drive, &parent, &key);
    match elem {
        Element::ProvableCountTree(_, count, _) => {
            assert_eq!(
                count, 0,
                "freshly created property-name ProvableCountTree should have aggregate 0"
            );
        }
        other => panic!(
            "rangeCountable-only top-level index property-name tree should be \
             ProvableCountTree, got {:?}",
            other
        ),
    }
}
