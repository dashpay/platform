//! The `apply: false` dry run of a document insert must upper-bound the
//! applied fees for a stored (not indexOnly) type whose index counts or sums.
//!
//! The first document under an index value creates the value tree and its
//! `0` bucket, both sum, count or count-sum trees, in the same GroveDB batch
//! as the reference inside them. With small fixed-width properties nothing
//! else in the estimate leaves room, so the dry run covers those two tree
//! elements only when GroveDB's average-case estimate sizes each as the
//! aggregating tree it is rather than as a plain tree (grove version 4,
//! protocol version 14).

use crate::drive::Drive;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::DataContractFactory;
use dpp::document::{Document, DocumentV0};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

const OWNER: [u8; 32] = [9; 32];

/// A `post` type with a `u8` `tier` and an `i8` `weight`, under `index`.
fn build_post_contract(index: Value) -> DataContract {
    let platform_version = PlatformVersion::latest();
    let schemas = platform_value!({
        "post": {
            "type": "object",
            "properties": {
                "tier": {"type": "integer", "minimum": 0, "maximum": 255, "position": 0},
                "weight": {"type": "integer", "minimum": -100, "maximum": 100, "position": 1}
            },
            "required": ["tier", "weight"],
            "indices": [index],
            "additionalProperties": false
        }
    });
    let config = DataContractConfig::default_for_version(platform_version).expect("config");
    DataContractFactory::new(platform_version.protocol_version)
        .expect("factory")
        .create(
            Identifier::from([203u8; 32]),
            0,
            schemas,
            Some(config),
            None,
        )
        .expect("create contract")
        .data_contract_owned()
}

fn post(marker: u8, tier: u8, weight: i8) -> Document {
    Document::V0(DocumentV0 {
        id: Identifier::from([marker; 32]),
        owner_id: Identifier::from(OWNER),
        properties: BTreeMap::from([
            ("tier".to_string(), Value::U8(tier)),
            ("weight".to_string(), Value::I8(weight)),
        ]),
        revision: Some(1),
        ..Default::default()
    })
}

fn insert_post(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    apply: bool,
) -> FeeResult {
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(OWNER),
                },
                contract,
                document_type: contract.document_type_for_name("post").expect("post"),
            },
            false,
            BlockInfo::default(),
            apply,
            None,
            PlatformVersion::latest(),
            None,
        )
        .expect("add document")
}

/// Estimates then applies a first and a second post (same tier, then a new
/// one), asserting the dry run upper-bounds each applied storage fee.
fn assert_estimate_upper_bounds_inserts(index: Value) {
    let platform_version = PlatformVersion::latest();
    let contract = build_post_contract(index);
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
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

    for document in [post(1, 5, -3), post(2, 5, 7), post(3, 9, 100)] {
        let estimated = insert_post(&drive, &contract, &document, false);
        let actual = insert_post(&drive, &contract, &document, true);
        assert!(
            estimated.storage_fee >= actual.storage_fee,
            "estimated storage fee {} must upper-bound actual {}",
            estimated.storage_fee,
            actual.storage_fee
        );
        assert!(
            estimated.processing_fee >= actual.processing_fee,
            "estimated processing fee {} must upper-bound actual {}",
            estimated.processing_fee,
            actual.processing_fee
        );
    }
}

#[test]
fn should_upper_bound_a_summable_index_insert() {
    assert_estimate_upper_bounds_inserts(platform_value!({
        "name": "byTier",
        "properties": [{"tier": "asc"}],
        "summable": "weight"
    }));
}

#[test]
fn should_upper_bound_a_range_summable_index_insert() {
    assert_estimate_upper_bounds_inserts(platform_value!({
        "name": "byTier",
        "properties": [{"tier": "asc"}],
        "summable": "weight",
        "rangeSummable": true
    }));
}

#[test]
fn should_upper_bound_a_countable_index_insert() {
    assert_estimate_upper_bounds_inserts(platform_value!({
        "name": "byTier",
        "properties": [{"tier": "asc"}],
        "countable": "countable"
    }));
}

#[test]
fn should_upper_bound_a_countable_and_summable_index_insert() {
    assert_estimate_upper_bounds_inserts(platform_value!({
        "name": "byTier",
        "properties": [{"tier": "asc"}],
        "countable": "countable",
        "summable": "weight"
    }));
}
