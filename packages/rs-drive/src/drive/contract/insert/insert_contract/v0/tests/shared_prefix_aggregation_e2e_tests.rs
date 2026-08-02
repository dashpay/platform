//! End-to-end coverage for shared-prefix aggregate index layouts.
//!
//! A shorter index can terminate at a prefix while another index
//! continues below that same prefix. When the prefix index uses count
//! + sum aggregation, child continuation trees under the prefix value
//! tree must contribute zero to every axis the value tree aggregates.
//!
//! Before platform v14, only the diagonal of the parent×child matrix
//! could be inserted: count-only parents accepted non-sum children and
//! sum-bearing parents accepted sum-bearing children. Everything else
//! registered fine as a contract but rejected every document insert.
//! The v14 index walkers (v2) complete the matrix — via
//! `Element::NonCounted` for non-sum children of count-sum parents,
//! unwrapped inserts for non-sum children of sum-only parents, and the
//! demotion of provable count-bearing value trees to `CountSumTree`
//! when continuations exist (grovedb's stated design rejects
//! count-suppressed children under provable count parents; pre-v14
//! those shapes only worked at all through an unenforced in-batch
//! creation path).
//!
//! Two suites:
//! - `..._insert_and_delete_at_latest` proves the entire matrix
//!   inserts at v14+, that the value trees carry exactly the `[0]`
//!   ref-bucket's (count, sum) — never the structural overhead of
//!   continuations — through insert AND delete, and that the
//!   continuation trees land with the exact expected wrapper.
//! - `..._frozen_at_v13` pins the pre-v14 behavior (which combos
//!   insert, which error) so the consensus-locked v1 walkers cannot
//!   drift.

use crate::drive::Drive;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::DataContractFactory;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::DataContract;
use dpp::tests::utils::generate_random_identifier_struct;
use dpp::version::PlatformVersion;
use grovedb::{Element, TreeType};
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
    const COUNT_SUM_RANGE_COUNT: Self = Self {
        countable: true,
        range_countable: true,
        summable: true,
        range_summable: false,
    };
    const COUNT_SUM_RANGE_SUM: Self = Self {
        countable: true,
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

    /// The property-name (continuation) tree type this sub-level's
    /// range flags produce — the inner tree hung under the prefix
    /// value tree when these are the CHILD index's flags.
    fn continuation_tree_type(&self) -> TreeType {
        match (self.range_countable, self.range_summable) {
            (true, true) => TreeType::ProvableCountProvableSumTree,
            (true, false) => TreeType::ProvableCountTree,
            (false, true) => TreeType::ProvableSumTree,
            (false, false) => TreeType::NormalTree,
        }
    }

    fn is_sum_bearing_continuation(&self) -> bool {
        self.range_summable
    }
}

/// Which axes the prefix index aggregates. With a compound sibling
/// present, this alone determines the value-tree type at v14+: the
/// provable count-bearing variants demote to `CountSumTree`, so the
/// range flags stop mattering for the value tree (they still upgrade
/// the property-name tree one level up).
#[derive(Clone, Copy, PartialEq)]
enum ParentAxes {
    Count,
    Sum,
    CountSum,
}

impl ParentAxes {
    fn from_flags(flags: &IndexFlags) -> Self {
        match (flags.countable, flags.summable) {
            (true, false) => ParentAxes::Count,
            (false, true) => ParentAxes::Sum,
            (true, true) => ParentAxes::CountSum,
            (false, false) => panic!("prefix index must aggregate at least one axis"),
        }
    }
}

struct SharedPrefixCase {
    name: String,
    prefix_flags: IndexFlags,
    child_flags: IndexFlags,
    /// Whether the combination could be inserted by the v1 walkers
    /// (protocol v12/v13). Pins today's consensus-frozen behavior.
    works_at_v13: bool,
}

/// Every aggregating prefix combination × every child combination.
/// `works_at_v13` marks the pre-v14 diagonal, empirically pinned:
/// count-only parents accepted non-sum children (`NonCounted`'s v0
/// inner set), and every sum-bearing parent — *including* the provable
/// count-bearing variants — accepted sum-bearing children
/// (`NotSummed` / `NotCountedOrSummed`'s inner set). The provable
/// parents accept them only because grovedb's wrapper-vs-provable
/// batch guard fires solely when the parent merk pre-exists, and the
/// walker always creates parent and wrapped child in one batch.
fn all_cases() -> Vec<SharedPrefixCase> {
    let prefixes: [(&'static str, IndexFlags); 8] = [
        ("count", IndexFlags::COUNT),
        ("sum", IndexFlags::SUM),
        ("count_sum", IndexFlags::COUNT_SUM),
        ("range_count", IndexFlags::RANGE_COUNT),
        ("range_sum", IndexFlags::RANGE_SUM),
        ("count_sum_range_count", IndexFlags::COUNT_SUM_RANGE_COUNT),
        ("count_sum_range_sum", IndexFlags::COUNT_SUM_RANGE_SUM),
        ("range_count_sum", IndexFlags::RANGE_COUNT_SUM),
    ];
    let children: [(&'static str, IndexFlags); 4] = [
        ("plain", IndexFlags::PLAIN),
        ("range_count", IndexFlags::RANGE_COUNT),
        ("range_sum", IndexFlags::RANGE_SUM),
        ("range_count_sum", IndexFlags::RANGE_COUNT_SUM),
    ];

    let mut cases = Vec::new();
    for (prefix_name, prefix_flags) in prefixes {
        for (child_name, child_flags) in children {
            let works_at_v13 = match ParentAxes::from_flags(&prefix_flags) {
                // v1's NonCounted helper accepted only count-ish inners.
                ParentAxes::Count => !child_flags.is_sum_bearing_continuation(),
                // v1's NotSummed / NotCountedOrSummed helpers accepted
                // only sum-bearing inners (and the provable parents let
                // them through — see the fn doc).
                ParentAxes::Sum | ParentAxes::CountSum => child_flags.is_sum_bearing_continuation(),
            };
            cases.push(SharedPrefixCase {
                name: format!("{prefix_name}_parent_{child_name}_child"),
                prefix_flags,
                child_flags,
                works_at_v13,
            });
        }
    }
    cases
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
        "documentsKeepHistory": false,
        "canBeDeleted": true,
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

fn apply_contract(
    drive: &Drive,
    contract: &DataContract,
    platform_version: &PlatformVersion,
) -> Result<(), String> {
    drive
        .apply_contract(
            contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

fn insert_review_document(
    drive: &Drive,
    contract: &DataContract,
    seed: u64,
    rating: u8,
    platform_version: &PlatformVersion,
) -> Result<Document, String> {
    let document_type = contract
        .document_type_for_name("review")
        .expect("review document type exists");
    let mut document = document_type
        .random_document(Some(seed), platform_version)
        .expect("random review document");
    let mut properties = BTreeMap::new();
    properties.insert(
        "resourceId".to_string(),
        Value::Text("resource-1".to_string()),
    );
    properties.insert("rating".to_string(), Value::U8(rating));
    properties.insert(
        "reviewText".to_string(),
        Value::Text("works as expected".to_string()),
    );
    document.set_properties(properties);

    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    // Use the document's own random owner — the delete
                    // path re-reads `$ownerId` from the stored document,
                    // so an override here would strand the owner-index
                    // entries.
                    document_info: DocumentRefInfo((&document, None)),
                    owner_id: None,
                },
                contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .map(|_| document)
        .map_err(|error| format!("{error:?}"))
}

/// Reads the raw element at `@/contract/1/review/resourceId` +
/// `"resource-1"` — the prefix index's value tree — or None if absent.
fn probe_value_tree(drive: &Drive, contract: &DataContract) -> Option<Element> {
    probe(drive, contract, &["resourceId"], b"resource-1")
}

/// Reads the raw element at
/// `@/contract/1/review/resourceId/resource-1` + `"rating"` — the
/// compound index's continuation property-name tree.
fn probe_continuation_tree(drive: &Drive, contract: &DataContract) -> Option<Element> {
    probe(drive, contract, &["resourceId", "resource-1"], b"rating")
}

fn probe(drive: &Drive, contract: &DataContract, sub_path: &[&str], key: &[u8]) -> Option<Element> {
    use crate::drive::RootTree;
    use crate::util::grove_operations::DirectQueryType;
    use grovedb_path::SubtreePath;

    let platform_version = PlatformVersion::latest();
    let contract_id = contract.id().to_buffer();
    let mut path: Vec<Vec<u8>> = vec![
        vec![RootTree::DataContractDocuments as u8],
        contract_id.to_vec(),
        vec![1u8],
        b"review".to_vec(),
    ];
    path.extend(sub_path.iter().map(|segment| segment.as_bytes().to_vec()));
    let path_slices: Vec<&[u8]> = path.iter().map(|segment| segment.as_slice()).collect();
    drive
        .grove_get_raw_optional(
            SubtreePath::from(path_slices.as_slice()),
            key,
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &platform_version.drive,
        )
        .expect("probe must succeed")
}

/// Asserts the value tree carries exactly the `[0]` bucket's
/// (count, sum) on the axes the prefix aggregates — with the
/// continuation demotion, always the non-provable variant.
fn assert_value_tree_aggregates(
    case_name: &str,
    element: &Element,
    axes: ParentAxes,
    expected_count: u64,
    expected_sum: i64,
) {
    match (axes, element) {
        (ParentAxes::Count, Element::CountTree(_, count, _)) => {
            assert_eq!(
                *count, expected_count,
                "{case_name}: value tree count must equal the ref-bucket contribution"
            );
        }
        (ParentAxes::Sum, Element::SumTree(_, sum, _)) => {
            assert_eq!(
                *sum, expected_sum,
                "{case_name}: value tree sum must equal the ref-bucket contribution"
            );
        }
        (ParentAxes::CountSum, Element::CountSumTree(_, count, sum, _)) => {
            assert_eq!(
                (*count, *sum),
                (expected_count, expected_sum),
                "{case_name}: value tree (count, sum) must equal the ref-bucket contribution"
            );
        }
        (_, other) => panic!(
            "{case_name}: unexpected value tree element (demotion or axis mismatch): {other:?}"
        ),
    }
}

/// Asserts the continuation property-name tree landed with the exact
/// wrapper the zero-contribution matrix specifies for this
/// parent-axes / child-tree-type combination.
fn assert_continuation_shape(
    case_name: &str,
    element: &Element,
    axes: ParentAxes,
    child_flags: &IndexFlags,
) {
    let expected_inner = child_flags.continuation_tree_type();
    let child_is_sum_bearing = child_flags.is_sum_bearing_continuation();

    let inner: &Element = match (axes, child_is_sum_bearing, element) {
        // Count-only parents wrap every child NonCounted.
        (ParentAxes::Count, _, Element::NonCounted(inner)) => inner,
        // Count-sum parents: sum-bearing children get NotCountedOrSummed,
        // non-sum children get NonCounted.
        (ParentAxes::CountSum, true, Element::NotCountedOrSummed(inner)) => inner,
        (ParentAxes::CountSum, false, Element::NonCounted(inner)) => inner,
        // Sum-only parents: sum-bearing children get NotSummed, non-sum
        // children are stored unwrapped (they contribute 0 naturally).
        (ParentAxes::Sum, true, Element::NotSummed(inner)) => inner,
        (ParentAxes::Sum, false, plain) => plain,
        (_, _, other) => panic!("{case_name}: unexpected continuation wrapper: {other:?}"),
    };

    let inner_tree_type = match inner {
        Element::Tree(..) => TreeType::NormalTree,
        Element::ProvableCountTree(..) => TreeType::ProvableCountTree,
        Element::ProvableSumTree(..) => TreeType::ProvableSumTree,
        Element::ProvableCountProvableSumTree(..) => TreeType::ProvableCountProvableSumTree,
        other => panic!("{case_name}: unexpected continuation inner element: {other:?}"),
    };
    assert_eq!(
        inner_tree_type, expected_inner,
        "{case_name}: continuation inner tree type mismatch"
    );
}

/// The whole matrix must insert at v14+, keep value-tree aggregates
/// equal to the `[0]` bucket contribution through insert and delete,
/// and clean the (possibly wrapped) trees up once the last document
/// is gone.
#[test]
fn shared_prefix_aggregate_index_combinations_insert_and_delete_at_latest() {
    let platform_version = PlatformVersion::latest();
    assert!(
        platform_version.protocol_version >= 14,
        "this suite exercises the v2 index walkers"
    );

    for case in all_cases() {
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = build_review_contract(case.prefix_flags, case.child_flags)
            .unwrap_or_else(|error| panic!("{}: contract must build: {error}", case.name));
        apply_contract(&drive, &contract, platform_version)
            .unwrap_or_else(|error| panic!("{}: contract must apply: {error}", case.name));

        let first_document = insert_review_document(&drive, &contract, 1, 5, platform_version)
            .unwrap_or_else(|error| panic!("{}: first insert must succeed: {error}", case.name));
        let second_document = insert_review_document(&drive, &contract, 2, 3, platform_version)
            .unwrap_or_else(|error| panic!("{}: second insert must succeed: {error}", case.name));

        let axes = ParentAxes::from_flags(&case.prefix_flags);
        let value_tree = probe_value_tree(&drive, &contract)
            .unwrap_or_else(|| panic!("{}: value tree must exist after inserts", case.name));
        assert_value_tree_aggregates(&case.name, &value_tree, axes, 2, 8);

        let continuation = probe_continuation_tree(&drive, &contract)
            .unwrap_or_else(|| panic!("{}: continuation tree must exist after inserts", case.name));
        assert_continuation_shape(&case.name, &continuation, axes, &case.child_flags);

        // Deleting one document must subtract exactly its contribution.
        drive
            .delete_document_for_contract(
                second_document.id(),
                &contract,
                "review",
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "{}: delete of second document must succeed: {error:?}",
                    case.name
                )
            });
        let value_tree = probe_value_tree(&drive, &contract)
            .unwrap_or_else(|| panic!("{}: value tree must survive first delete", case.name));
        assert_value_tree_aggregates(&case.name, &value_tree, axes, 1, 5);

        // Deleting the last document must clean the value tree (and the
        // wrapped continuation trees inside it) away entirely.
        drive
            .delete_document_for_contract(
                first_document.id(),
                &contract,
                "review",
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "{}: delete of last document must succeed: {error:?}",
                    case.name
                )
            });
        assert!(
            probe_value_tree(&drive, &contract).is_none(),
            "{}: value tree must be cleaned up once empty",
            case.name
        );
    }
}

/// Pins the consensus-frozen v1 walker behavior: at protocol v13 the
/// pre-v14 diagonal still inserts and everything else still errors.
/// If this test ever changes outcome for any case, v13 consensus has
/// drifted.
#[test]
fn shared_prefix_aggregate_index_combinations_frozen_at_v13() {
    let platform_version_v13 = PlatformVersion::get(13).expect("platform version 13 must be known");

    let mut mismatches = Vec::new();
    for case in all_cases() {
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = build_review_contract(case.prefix_flags, case.child_flags)
            .unwrap_or_else(|error| panic!("{}: contract must build: {error}", case.name));
        apply_contract(&drive, &contract, platform_version_v13)
            .unwrap_or_else(|error| panic!("{}: contract must apply at v13: {error}", case.name));

        let result = insert_review_document(&drive, &contract, 1, 5, platform_version_v13);
        match (case.works_at_v13, result) {
            (true, Err(error)) => mismatches.push(format!(
                "{}: expected insert to succeed at v13, got: {error}",
                case.name
            )),
            (false, Ok(_)) => mismatches.push(format!(
                "{}: expected insert to fail at v13, but it succeeded",
                case.name
            )),
            _ => {}
        }
    }

    assert!(
        mismatches.is_empty(),
        "v13 consensus freeze drifted:\n{}",
        mismatches.join("\n")
    );
}
