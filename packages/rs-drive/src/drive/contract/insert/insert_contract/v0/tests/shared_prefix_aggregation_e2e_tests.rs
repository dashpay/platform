//! End-to-end coverage for shared-prefix aggregate index layouts.
//!
//! A shorter index can terminate at a prefix while another index
//! continues below that same prefix. When the prefix index uses count
//! + sum aggregation, child continuation trees under the prefix value
//! tree must either be supported by Drive's wrapper logic or the
//! contract must be rejected before publish.

use crate::drive::Drive;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::DataContractFactory;
use dpp::document::DocumentV0Setters;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::DataContract;
use dpp::tests::utils::generate_random_identifier_struct;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

const PROTOCOL_VERSION_V12: u32 = 12;

fn build_review_contract() -> DataContract {
    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");

    let document_schema = platform_value!({
        "type": "object",
        "documentsMutable": true,
        "documentsKeepHistory": true,
        "canBeDeleted": false,
        "properties": {
            "resourceId": {
                "type": "string",
                "minLength": 1,
                "maxLength": 63,
                "position": 0,
            },
            "rating": {
                "type": "integer",
                "minimum": 1,
                "maximum": 5,
                "position": 1,
            },
            "reviewText": {
                "type": "string",
                "maxLength": 1000,
                "position": 2,
            },
        },
        "required": ["$createdAt", "$updatedAt", "resourceId", "rating"],
        "additionalProperties": false,
        "indices": [
            {
                "name": "ownerAndResource",
                "unique": true,
                "properties": [{"$ownerId": "asc"}, {"resourceId": "asc"}],
            },
            {
                "name": "ownerReviews",
                "properties": [{"$ownerId": "asc"}, {"$updatedAt": "asc"}],
            },
            {
                "name": "resourceRatingAggregate",
                "properties": [{"resourceId": "asc"}],
                "countable": "countable",
                "summable": "rating",
            },
            {
                "name": "resourceRatingDistribution",
                "properties": [{"resourceId": "asc"}, {"rating": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
            },
        ],
    });

    let schemas = platform_value!({ "review": document_schema });
    let owner_id = generate_random_identifier_struct();

    factory
        .create_with_value_config(owner_id, 0, schemas, None, None)
        .expect("expected to create data contract")
        .data_contract_owned()
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

#[test]
fn insert_document_with_shared_prefix_count_sum_and_range_countable_indexes_succeeds() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = build_review_contract();
    apply_contract(&drive, &contract);

    let document_type = contract
        .document_type_for_name("review")
        .expect("review document type exists");
    let mut document = document_type
        .random_document(Some(1), pv)
        .expect("random review document");
    let mut properties = BTreeMap::new();
    properties.insert(
        "resourceId".to_string(),
        Value::Text("resource-1".to_string()),
    );
    properties.insert("rating".to_string(), Value::U8(5));
    properties.insert(
        "reviewText".to_string(),
        Value::Text("works as expected".to_string()),
    );
    document.set_properties(properties);

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&document, None)),
                    owner_id: Some(generate_random_identifier_struct().into()),
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect(
            "accepted shared-prefix aggregate indexes must not fail document insertion due to \
             unsupported NotCountedOrSummed wrapping",
        );
}
