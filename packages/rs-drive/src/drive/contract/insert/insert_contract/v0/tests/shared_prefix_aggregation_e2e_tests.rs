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

#[derive(Clone, Copy)]
struct IndexFlags {
    countable: bool,
    range_countable: bool,
    summable: bool,
    range_summable: bool,
}

impl IndexFlags {
    const PLAIN: Self = Self {
        countable: false,
        range_countable: false,
        summable: false,
        range_summable: false,
    };
    const COUNT: Self = Self {
        countable: true,
        range_countable: false,
        summable: false,
        range_summable: false,
    };
    const SUM: Self = Self {
        countable: false,
        range_countable: false,
        summable: true,
        range_summable: false,
    };
    const COUNT_SUM: Self = Self {
        countable: true,
        range_countable: false,
        summable: true,
        range_summable: false,
    };
    const RANGE_COUNT: Self = Self {
        countable: true,
        range_countable: true,
        summable: false,
        range_summable: false,
    };
    const RANGE_SUM: Self = Self {
        countable: false,
        range_countable: false,
        summable: true,
        range_summable: true,
    };
    const RANGE_COUNT_SUM: Self = Self {
        countable: true,
        range_countable: true,
        summable: true,
        range_summable: true,
    };
}

struct SharedPrefixCase {
    name: &'static str,
    prefix_flags: IndexFlags,
    child_flags: IndexFlags,
    must_insert: bool,
}

enum SharedPrefixOutcome {
    Inserted,
    ContractRejected(String),
    ApplyFailed(String),
    InsertFailed(String),
}

fn review_index(name: &str, properties: Vec<Value>, flags: IndexFlags) -> Value {
    let mut index = vec![
        (
            Value::Text("name".to_string()),
            Value::Text(name.to_string()),
        ),
        (
            Value::Text("properties".to_string()),
            Value::Array(properties),
        ),
    ];

    if flags.countable {
        index.push((
            Value::Text("countable".to_string()),
            Value::Text("countable".to_string()),
        ));
    }
    if flags.range_countable {
        index.push((Value::Text("rangeCountable".to_string()), Value::Bool(true)));
    }
    if flags.summable {
        index.push((
            Value::Text("summable".to_string()),
            Value::Text("rating".to_string()),
        ));
    }
    if flags.range_summable {
        index.push((Value::Text("rangeSummable".to_string()), Value::Bool(true)));
    }

    Value::Map(index)
}

fn build_review_contract(
    prefix_flags: IndexFlags,
    child_flags: IndexFlags,
) -> Result<DataContract, String> {
    let factory =
        DataContractFactory::new(PROTOCOL_VERSION_V12).expect("expected to create factory");

    let indices = vec![
        platform_value!({
            "name": "ownerAndResource",
            "unique": true,
            "properties": [{"$ownerId": "asc"}, {"resourceId": "asc"}],
        }),
        platform_value!({
            "name": "ownerReviews",
            "properties": [{"$ownerId": "asc"}, {"$updatedAt": "asc"}],
        }),
        review_index(
            "resourceRatingAggregate",
            vec![platform_value!({"resourceId": "asc"})],
            prefix_flags,
        ),
        review_index(
            "resourceRatingDistribution",
            vec![
                platform_value!({"resourceId": "asc"}),
                platform_value!({"rating": "asc"}),
            ],
            child_flags,
        ),
    ];

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
        "indices": Value::Array(indices),
    });

    let schemas = platform_value!({ "review": document_schema });
    let owner_id = generate_random_identifier_struct();

    factory
        .create_with_value_config(owner_id, 0, schemas, None, None)
        .map(|contract| contract.data_contract_owned())
        .map_err(|error| format!("{error:?}"))
}

fn apply_contract(drive: &Drive, contract: &DataContract) -> Result<(), String> {
    drive
        .apply_contract(
            contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            PlatformVersion::latest(),
        )
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

fn insert_review_document_for_case(case: &SharedPrefixCase) -> SharedPrefixOutcome {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = PlatformVersion::latest();
    let contract = match build_review_contract(case.prefix_flags, case.child_flags) {
        Ok(contract) => contract,
        Err(error) => return SharedPrefixOutcome::ContractRejected(error),
    };
    if let Err(error) = apply_contract(&drive, &contract) {
        return SharedPrefixOutcome::ApplyFailed(error);
    }

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
        .map_or_else(
            |error| SharedPrefixOutcome::InsertFailed(format!("{error:?}")),
            |_| SharedPrefixOutcome::Inserted,
        )
}

#[test]
#[ignore = "tracks #3960; unsupported shared-prefix aggregate layouts are accepted but fail insertion today"]
fn shared_prefix_aggregate_index_combinations_reject_or_insert() {
    let cases = [
        SharedPrefixCase {
            name: "count_parent_plain_child",
            prefix_flags: IndexFlags::COUNT,
            child_flags: IndexFlags::PLAIN,
            must_insert: true,
        },
        SharedPrefixCase {
            name: "count_parent_range_count_child",
            prefix_flags: IndexFlags::COUNT,
            child_flags: IndexFlags::RANGE_COUNT,
            must_insert: true,
        },
        SharedPrefixCase {
            name: "count_parent_range_sum_child",
            prefix_flags: IndexFlags::COUNT,
            child_flags: IndexFlags::RANGE_SUM,
            must_insert: false,
        },
        SharedPrefixCase {
            name: "count_parent_range_count_sum_child",
            prefix_flags: IndexFlags::COUNT,
            child_flags: IndexFlags::RANGE_COUNT_SUM,
            must_insert: false,
        },
        SharedPrefixCase {
            name: "sum_parent_plain_child",
            prefix_flags: IndexFlags::SUM,
            child_flags: IndexFlags::PLAIN,
            must_insert: false,
        },
        SharedPrefixCase {
            name: "sum_parent_range_count_child",
            prefix_flags: IndexFlags::SUM,
            child_flags: IndexFlags::RANGE_COUNT,
            must_insert: false,
        },
        SharedPrefixCase {
            name: "sum_parent_range_sum_child",
            prefix_flags: IndexFlags::SUM,
            child_flags: IndexFlags::RANGE_SUM,
            must_insert: true,
        },
        SharedPrefixCase {
            name: "sum_parent_range_count_sum_child",
            prefix_flags: IndexFlags::SUM,
            child_flags: IndexFlags::RANGE_COUNT_SUM,
            must_insert: true,
        },
        SharedPrefixCase {
            name: "count_sum_parent_plain_child",
            prefix_flags: IndexFlags::COUNT_SUM,
            child_flags: IndexFlags::PLAIN,
            must_insert: false,
        },
        SharedPrefixCase {
            name: "count_sum_parent_range_count_child",
            prefix_flags: IndexFlags::COUNT_SUM,
            child_flags: IndexFlags::RANGE_COUNT,
            must_insert: false,
        },
        SharedPrefixCase {
            name: "count_sum_parent_range_sum_child",
            prefix_flags: IndexFlags::COUNT_SUM,
            child_flags: IndexFlags::RANGE_SUM,
            must_insert: true,
        },
        SharedPrefixCase {
            name: "count_sum_parent_range_count_sum_child",
            prefix_flags: IndexFlags::COUNT_SUM,
            child_flags: IndexFlags::RANGE_COUNT_SUM,
            must_insert: true,
        },
    ];

    let failures = cases
        .iter()
        .filter_map(|case| match insert_review_document_for_case(case) {
            SharedPrefixOutcome::Inserted => None,
            SharedPrefixOutcome::ContractRejected(_) if !case.must_insert => None,
            SharedPrefixOutcome::ContractRejected(error) => Some(format!(
                "{}: compatible shared-prefix aggregate indexes were rejected: {}",
                case.name, error
            )),
            SharedPrefixOutcome::ApplyFailed(error) => Some(format!(
                "{}: contract application failed: {}",
                case.name, error
            )),
            SharedPrefixOutcome::InsertFailed(error) => Some(format!("{}: {}", case.name, error)),
        })
        .collect::<Vec<_>>();

    assert!(
        failures.is_empty(),
        "shared-prefix aggregate index contracts must either reject unsupported layouts at \
         contract creation or allow document insertion; incompatible accepted layouts failed for:\n{}",
        failures.join("\n")
    );
}
