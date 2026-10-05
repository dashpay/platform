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
//! creation path). Key-changing updates materialize index branches
//! through their own walker, which is bumped to v1 at v14 with the
//! same demotion + zero-contribution treatment.
//!
//! Suites:
//! - `..._insert_update_delete_at_v14` proves the entire matrix
//!   inserts at v14, that the value trees carry exactly the `[0]`
//!   ref-bucket's (count, sum) — never the structural overhead of
//!   continuations — through insert, key-changing update (which
//!   materializes a fresh branch via the update walker), and delete,
//!   and that the continuation trees land with the exact expected
//!   wrapper on both the insert- and update-materialized branches.
//! - `..._frozen_at_v13` pins the pre-v14 behavior (which combos
//!   insert, which error) so the consensus-locked v1 walkers cannot
//!   drift.
//! - `..._estimated_costs_do_not_write_state` exercises the v2
//!   walkers' stateless-estimation branches (`apply: false`).
//! - `..._v13_and_v14_layouts_coexist` proves a provable value tree
//!   created at v13 keeps working at v14 next to newly-demoted
//!   `CountSumTree` siblings, through inserts and full-cleanup
//!   deletes.

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
    const RANGE_COUNT_RANGE_SUM: Self = Self {
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
        ("range_count_range_sum", IndexFlags::RANGE_COUNT_RANGE_SUM),
    ];
    let children: [(&'static str, IndexFlags); 4] = [
        ("plain", IndexFlags::PLAIN),
        ("range_count", IndexFlags::RANGE_COUNT),
        ("range_sum", IndexFlags::RANGE_SUM),
        ("range_count_range_sum", IndexFlags::RANGE_COUNT_RANGE_SUM),
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

/// Inserts a review document. `apply: false` runs the stateless
/// estimation path instead of writing state.
///
/// Seeds must be distinct across calls: the `ownerAndResource` index
/// is unique on `($ownerId, resourceId)` and all documents share a
/// per-test `resourceId`, so only the seed-derived random owner keeps
/// the inserts from colliding on that index.
fn insert_review_document(
    drive: &Drive,
    contract: &DataContract,
    seed: u64,
    resource: &str,
    rating: u8,
    apply: bool,
    platform_version: &PlatformVersion,
) -> Result<Document, String> {
    let document_type = contract
        .document_type_for_name("review")
        .expect("review document type exists");
    let mut document = document_type
        .random_document(Some(seed), platform_version)
        .expect("random review document");
    let mut properties = BTreeMap::new();
    properties.insert("resourceId".to_string(), Value::Text(resource.to_string()));
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
            apply,
            None,
            platform_version,
            None,
        )
        .map(|_| document)
        .map_err(|error| format!("{error:?}"))
}

/// Moves a stored review document to a new `resourceId` through the
/// document UPDATE path — the update walker materializes the new
/// index branch itself, which is exactly the surface the v14
/// update-walker bump covers.
fn update_review_document_resource(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    new_resource: &str,
    platform_version: &PlatformVersion,
) -> Result<Document, String> {
    let document_type = contract
        .document_type_for_name("review")
        .expect("review document type exists");
    let mut updated = document.clone();
    updated.set("resourceId", Value::Text(new_resource.to_string()));
    updated.set_revision(updated.revision().map(|revision| revision + 1));

    drive
        .update_document_for_contract(
            &updated,
            contract,
            document_type,
            None,
            BlockInfo::default(),
            true,
            None,
            None,
            platform_version,
            None,
        )
        .map(|_| updated)
        .map_err(|error| format!("{error:?}"))
}

fn delete_review_document(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    platform_version: &PlatformVersion,
) -> Result<(), String> {
    drive
        .delete_document_for_contract(
            document.id(),
            contract,
            "review",
            BlockInfo::default(),
            true,
            None,
            platform_version,
            None,
        )
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

/// Reads the raw element at `@/contract/1/review/resourceId` +
/// `resource` — the prefix index's value tree — or None if absent.
fn probe_value_tree(drive: &Drive, contract: &DataContract, resource: &str) -> Option<Element> {
    probe(drive, contract, &["resourceId"], resource.as_bytes())
}

/// Reads the raw element at
/// `@/contract/1/review/resourceId/<resource>` + `"rating"` — the
/// compound index's continuation property-name tree.
fn probe_continuation_tree(
    drive: &Drive,
    contract: &DataContract,
    resource: &str,
) -> Option<Element> {
    probe(drive, contract, &["resourceId", resource], b"rating")
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

/// Checks the value tree carries exactly the `[0]` bucket's
/// (count, sum) on the axes the prefix aggregates — with the
/// continuation demotion, always the non-provable variant.
fn check_value_tree_aggregates(
    element: &Element,
    axes: ParentAxes,
    expected_count: u64,
    expected_sum: i64,
) -> Result<(), String> {
    match (axes, element) {
        (ParentAxes::Count, Element::CountTree(_, count, _)) => {
            if *count != expected_count {
                return Err(format!(
                    "value tree count {count} != expected ref-bucket contribution {expected_count}"
                ));
            }
        }
        (ParentAxes::Sum, Element::SumTree(_, sum, _)) => {
            if *sum != expected_sum {
                return Err(format!(
                    "value tree sum {sum} != expected ref-bucket contribution {expected_sum}"
                ));
            }
        }
        (ParentAxes::CountSum, Element::CountSumTree(_, count, sum, _)) => {
            if (*count, *sum) != (expected_count, expected_sum) {
                return Err(format!(
                    "value tree (count, sum) ({count}, {sum}) != expected ref-bucket \
                     contribution ({expected_count}, {expected_sum})"
                ));
            }
        }
        (_, other) => {
            return Err(format!(
                "unexpected value tree element (demotion or axis mismatch): {other:?}"
            ))
        }
    }
    Ok(())
}

/// Checks the continuation property-name tree landed with the exact
/// wrapper the zero-contribution matrix specifies for this
/// parent-axes / child-tree-type combination.
fn check_continuation_shape(
    element: &Element,
    axes: ParentAxes,
    child_flags: &IndexFlags,
) -> Result<(), String> {
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
        (_, _, other) => return Err(format!("unexpected continuation wrapper: {other:?}")),
    };

    let inner_tree_type = match inner {
        Element::Tree(..) => TreeType::NormalTree,
        Element::ProvableCountTree(..) => TreeType::ProvableCountTree,
        Element::ProvableSumTree(..) => TreeType::ProvableSumTree,
        Element::ProvableCountProvableSumTree(..) => TreeType::ProvableCountProvableSumTree,
        other => return Err(format!("unexpected continuation inner element: {other:?}")),
    };
    if inner_tree_type != expected_inner {
        return Err(format!(
            "continuation inner tree type {inner_tree_type:?} != expected {expected_inner:?}"
        ));
    }
    Ok(())
}

/// Full v14 lifecycle for one matrix case: two inserts, a
/// key-changing update (branch materialized by the UPDATE walker),
/// and deletes down to full cleanup, with the value-tree aggregates
/// checked against the `[0]`-bucket contribution at every step.
fn run_case_at_v14(
    case: &SharedPrefixCase,
    platform_version: &PlatformVersion,
) -> Result<(), String> {
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = build_review_contract(case.prefix_flags, case.child_flags)
        .map_err(|error| format!("contract must build: {error}"))?;
    apply_contract(&drive, &contract, platform_version)
        .map_err(|error| format!("contract must apply: {error}"))?;

    let axes = ParentAxes::from_flags(&case.prefix_flags);

    let first_document = insert_review_document(
        &drive,
        &contract,
        1,
        "resource-1",
        5,
        true,
        platform_version,
    )
    .map_err(|error| format!("first insert must succeed: {error}"))?;
    let second_document = insert_review_document(
        &drive,
        &contract,
        2,
        "resource-1",
        3,
        true,
        platform_version,
    )
    .map_err(|error| format!("second insert must succeed: {error}"))?;

    let value_tree = probe_value_tree(&drive, &contract, "resource-1")
        .ok_or("value tree must exist after inserts")?;
    check_value_tree_aggregates(&value_tree, axes, 2, 8)
        .map_err(|error| format!("after inserts: {error}"))?;
    let continuation = probe_continuation_tree(&drive, &contract, "resource-1")
        .ok_or("continuation tree must exist after inserts")?;
    check_continuation_shape(&continuation, axes, &case.child_flags)
        .map_err(|error| format!("insert-materialized continuation: {error}"))?;

    // Move the second document to a fresh resource through the UPDATE
    // path — the update walker materializes the resource-2 branch.
    let second_document = update_review_document_resource(
        &drive,
        &contract,
        &second_document,
        "resource-2",
        platform_version,
    )
    .map_err(|error| format!("key-changing update must succeed: {error}"))?;

    let value_tree = probe_value_tree(&drive, &contract, "resource-1")
        .ok_or("resource-1 value tree must survive the update")?;
    check_value_tree_aggregates(&value_tree, axes, 1, 5)
        .map_err(|error| format!("after update, old branch: {error}"))?;
    let value_tree = probe_value_tree(&drive, &contract, "resource-2")
        .ok_or("resource-2 value tree must exist after the update")?;
    check_value_tree_aggregates(&value_tree, axes, 1, 3)
        .map_err(|error| format!("after update, new branch: {error}"))?;
    let continuation = probe_continuation_tree(&drive, &contract, "resource-2")
        .ok_or("continuation tree must exist on the update-materialized branch")?;
    check_continuation_shape(&continuation, axes, &case.child_flags)
        .map_err(|error| format!("update-materialized continuation: {error}"))?;

    // Deleting each branch's last document must clean its trees away
    // entirely (through the wrapped continuations).
    delete_review_document(&drive, &contract, &second_document, platform_version)
        .map_err(|error| format!("delete of second document must succeed: {error}"))?;
    if probe_value_tree(&drive, &contract, "resource-2").is_some() {
        return Err("resource-2 value tree must be cleaned up once empty".to_string());
    }
    delete_review_document(&drive, &contract, &first_document, platform_version)
        .map_err(|error| format!("delete of first document must succeed: {error}"))?;
    if probe_value_tree(&drive, &contract, "resource-1").is_some() {
        return Err("resource-1 value tree must be cleaned up once empty".to_string());
    }
    Ok(())
}

/// The whole matrix must insert, update, and delete correctly at
/// protocol v14. Pinned to v14 explicitly (not `latest()`) so the
/// exact v14 → Drive v9 → v2-walker dispatch chain stays covered when
/// later protocol versions appear.
#[test]
fn shared_prefix_aggregate_index_combinations_insert_update_delete_at_v14() {
    let platform_version = PlatformVersion::get(14).expect("platform version 14 must be known");

    let mut failures = Vec::new();
    for case in all_cases() {
        if let Err(error) = run_case_at_v14(&case, platform_version) {
            failures.push(format!("{}: {error}", case.name));
        }
    }

    assert!(
        failures.is_empty(),
        "v14 shared-prefix aggregate matrix failed:\n{}",
        failures.join("\n")
    );
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

        let result = insert_review_document(
            &drive,
            &contract,
            1,
            "resource-1",
            5,
            true,
            platform_version_v13,
        );
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

/// The v2 walkers' stateless-estimation branches (`apply: false`)
/// must produce a fee without writing any state — covering the
/// post-demotion `tree_type` / `EstimatedSumTrees` layer info and the
/// stateless zero-contribution op construction.
#[test]
fn shared_prefix_aggregate_estimated_costs_do_not_write_state() {
    let platform_version = PlatformVersion::get(14).expect("platform version 14 must be known");

    // One demoted layout (provable count-bearing prefix) and one
    // non-demoted aggregating layout (sum-only prefix).
    let representative = [
        (
            "demoted_range_count_range_sum_parent_plain_child",
            IndexFlags::RANGE_COUNT_RANGE_SUM,
            IndexFlags::PLAIN,
        ),
        ("sum_parent_plain_child", IndexFlags::SUM, IndexFlags::PLAIN),
    ];

    for (name, prefix_flags, child_flags) in representative {
        let drive = setup_drive_with_initial_state_structure(None);
        let contract = build_review_contract(prefix_flags, child_flags)
            .unwrap_or_else(|error| panic!("{name}: contract must build: {error}"));
        apply_contract(&drive, &contract, platform_version)
            .unwrap_or_else(|error| panic!("{name}: contract must apply: {error}"));

        insert_review_document(
            &drive,
            &contract,
            1,
            "resource-1",
            5,
            false,
            platform_version,
        )
        .unwrap_or_else(|error| panic!("{name}: estimated insert must succeed: {error}"));

        assert!(
            probe_value_tree(&drive, &contract, "resource-1").is_none(),
            "{name}: estimated insert must not write state"
        );
    }
}

/// A provable count-sum value tree created at v13 (through the
/// unenforced in-batch wrapper path) must keep working at v14 next to
/// newly-demoted `CountSumTree` siblings: inserts into both branches
/// keep exact aggregates, and deletes clean both away.
#[test]
fn shared_prefix_aggregate_v13_and_v14_layouts_coexist() {
    let platform_version_v13 = PlatformVersion::get(13).expect("platform version 13 must be known");
    let platform_version_v14 = PlatformVersion::get(14).expect("platform version 14 must be known");

    // countable + rangeCountable + summable prefix → ProvableCountSumTree
    // value trees at v13; sum-bearing (rangeSummable) child so the shape
    // is insertable at v13.
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = build_review_contract(IndexFlags::COUNT_SUM_RANGE_COUNT, IndexFlags::RANGE_SUM)
        .expect("contract must build");
    apply_contract(&drive, &contract, platform_version_v13).expect("contract must apply at v13");

    let first_document = insert_review_document(
        &drive,
        &contract,
        1,
        "resource-1",
        5,
        true,
        platform_version_v13,
    )
    .expect("v13 insert must succeed");

    let v13_tree =
        probe_value_tree(&drive, &contract, "resource-1").expect("v13 value tree must exist");
    match &v13_tree {
        Element::ProvableCountSumTree(_, count, sum, _) => {
            assert_eq!((*count, *sum), (1, 5), "v13 provable value tree aggregates");
        }
        other => panic!("expected ProvableCountSumTree at v13, got {other:?}"),
    }

    // v14 insert into the EXISTING v13-created provable branch.
    let second_document = insert_review_document(
        &drive,
        &contract,
        2,
        "resource-1",
        3,
        true,
        platform_version_v14,
    )
    .expect("v14 insert into the v13 branch must succeed");
    let v13_tree =
        probe_value_tree(&drive, &contract, "resource-1").expect("v13 value tree must survive");
    match &v13_tree {
        Element::ProvableCountSumTree(_, count, sum, _) => {
            assert_eq!(
                (*count, *sum),
                (2, 8),
                "v13-created provable value tree must keep exact aggregates at v14"
            );
        }
        other => panic!("v13-created value tree must keep its type at v14, got {other:?}"),
    }

    // v14 insert materializing a NEW branch — demoted CountSumTree.
    let third_document = insert_review_document(
        &drive,
        &contract,
        3,
        "resource-2",
        4,
        true,
        platform_version_v14,
    )
    .expect("v14 insert into a new branch must succeed");
    let v14_tree =
        probe_value_tree(&drive, &contract, "resource-2").expect("v14 value tree must exist");
    match &v14_tree {
        Element::CountSumTree(_, count, sum, _) => {
            assert_eq!((*count, *sum), (1, 4), "v14 demoted value tree aggregates");
        }
        other => panic!("expected demoted CountSumTree at v14, got {other:?}"),
    }

    // Deletes at v14 must clean both layouts away entirely.
    for document in [&first_document, &second_document] {
        delete_review_document(&drive, &contract, document, platform_version_v14)
            .expect("v14 delete from the v13 branch must succeed");
    }
    assert!(
        probe_value_tree(&drive, &contract, "resource-1").is_none(),
        "v13-created value tree must be cleaned up once empty"
    );
    delete_review_document(&drive, &contract, &third_document, platform_version_v14)
        .expect("v14 delete from the v14 branch must succeed");
    assert!(
        probe_value_tree(&drive, &contract, "resource-2").is_none(),
        "v14-created value tree must be cleaned up once empty"
    );
}

/// The two demotions on one doctype: a shared-prefix aggregating pair
/// (`[resourceId]` count+sum terminator with a plain `[resourceId,
/// rating]` continuation → `NonCounted` under a `CountSumTree`) next to
/// a prefix-ranked chain with a plain count-exempt sibling
/// (`rankedCountable: { at: "author" }` on `[author, rating]` with the
/// sibling `[author, day, rating]` → `NonCounted` under the chain's
/// `CountTree`). Both wrappers must land, and both aggregate surfaces
/// must stay exact with all four indexes populated by the same rows.
#[test]
fn range_demotion_and_at_chain_exemption_coexist_in_one_doctype() {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(None);

    let factory = DataContractFactory::new(platform_version.protocol_version)
        .expect("expected to create factory");
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
            "author": {
                "type": "string",
                "minLength": 1,
                "maxLength": 61,
                "position": 2,
            },
            "day": {
                "type": "string",
                "minLength": 1,
                "maxLength": 16,
                "position": 3,
            },
        },
        "required": ["resourceId", "rating", "author", "day"],
        "additionalProperties": false,
        "indices": [
            {
                "name": "resourceAggregate",
                "properties": [{"resourceId": "asc"}],
                "countable": "countable",
                "summable": "rating",
            },
            {
                "name": "resourceRating",
                "properties": [{"resourceId": "asc"}, {"rating": "asc"}],
            },
            {
                "name": "authorRating",
                "properties": [{"author": "asc"}, {"rating": "asc"}],
                "countable": "countable",
                "rangeCountable": true,
                "rankedCountable": {"at": "author"},
            },
            {
                "name": "authorDayRating",
                "properties": [{"author": "asc"}, {"day": "asc"}, {"rating": "asc"}],
            },
        ],
    });
    let schemas = platform_value!({ "mixed": document_schema });
    let owner_id = generate_random_identifier_struct();
    let contract = factory
        .create_with_value_config(owner_id, 0, schemas, None, None)
        .expect("both demotions must be admitted on one doctype")
        .data_contract_owned();
    apply_contract(&drive, &contract, platform_version).expect("expected to apply the contract");

    let document_type = contract
        .document_type_for_name("mixed")
        .expect("mixed doctype exists");
    let rows: [(&str, u8, &str, &str); 4] = [
        ("r1", 5, "alice", "d1"),
        ("r1", 3, "alice", "d2"),
        ("r2", 4, "alice", "d1"),
        ("r1", 2, "bob", "d1"),
    ];
    for (seed, (resource, rating, author, day)) in rows.iter().enumerate() {
        let mut document = document_type
            .random_document(Some(seed as u64 + 1), platform_version)
            .expect("random mixed document");
        let mut properties = BTreeMap::new();
        properties.insert("resourceId".to_string(), Value::Text(resource.to_string()));
        properties.insert("rating".to_string(), Value::U8(*rating));
        properties.insert("author".to_string(), Value::Text(author.to_string()));
        properties.insert("day".to_string(), Value::Text(day.to_string()));
        document.set_properties(properties);
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert a mixed document");
    }

    let probe_mixed = |sub_path: &[&str], key: &[u8]| -> Option<Element> {
        use crate::drive::RootTree;
        use crate::util::grove_operations::DirectQueryType;
        use grovedb_path::SubtreePath;
        let contract_id = contract.id().to_buffer();
        let mut path: Vec<Vec<u8>> = vec![
            vec![RootTree::DataContractDocuments as u8],
            contract_id.to_vec(),
            vec![1u8],
            b"mixed".to_vec(),
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
    };

    // Shared-prefix surface: r1's value tree aggregates exactly its three
    // reviews' (count, sum) and the compound continuation is wrapped.
    match probe_mixed(&["resourceId"], b"r1") {
        Some(Element::CountSumTree(_, count, sum, _)) => {
            assert_eq!((count, sum), (3, 10), "r1 aggregates its own rows exactly");
        }
        other => panic!("expected a CountSumTree value tree for r1, got {other:?}"),
    }
    assert!(
        matches!(
            probe_mixed(&["resourceId", "r1"], b"rating"),
            Some(Element::NonCounted(inner)) if matches!(inner.as_ref(), Element::Tree(..))
        ),
        "the shared-prefix continuation must stay NonCounted-wrapped"
    );

    // Ranked surface: alice counts exactly her three rows, the chain
    // continuation contributes, and the sibling branch is wrapped.
    match probe_mixed(&["author"], b"alice") {
        Some(element @ Element::CountTree(..)) => {
            assert_eq!(element.count_value_or_default(), 3);
        }
        other => panic!("expected a CountTree value tree for alice, got {other:?}"),
    }
    assert!(
        matches!(
            probe_mixed(&["author", "alice"], b"rating"),
            Some(Element::ProvableCountTree(..))
        ),
        "the chain continuation must stay contributing"
    );
    assert!(
        matches!(
            probe_mixed(&["author", "alice"], b"day"),
            Some(Element::NonCounted(inner)) if matches!(inner.as_ref(), Element::Tree(..))
        ),
        "the count-exempt sibling branch must be NonCounted-wrapped"
    );

    // The grouping secondary ranks by the exact totals.
    let grouping_path: Vec<Vec<u8>> = vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().to_buffer().to_vec(),
        vec![1u8],
        b"mixed".to_vec(),
        b"author".to_vec(),
    ];
    let path_query = grovedb::PathQuery::new_axis(
        grouping_path,
        grovedb_query::AxisQuery::top_k(grovedb_query::IndexAxis::Count, 10, 0, true).keys_only(),
    );
    let pairs = match drive
        .grove
        .run_path_query(
            &path_query,
            true,
            true,
            true,
            grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType,
            None,
            &platform_version.drive.grove_version,
        )
        .unwrap()
        .expect("the keys-only axis read must succeed")
    {
        grovedb::PathQueryRun::AxisKeys {
            keys: grovedb::AxisKeys::Count(pairs),
            ..
        } => pairs,
        other => panic!("expected count keys, got {other:?}"),
    };
    assert_eq!(
        pairs,
        vec![(3, b"alice".to_vec()), (1, b"bob".to_vec())],
        "the ranking must key on exact totals with all four indexes populated"
    );

    let issues = drive
        .grove
        .verify_grovedb(None, true, false, &platform_version.drive.grove_version)
        .expect("verify_grovedb must run");
    assert!(
        issues.is_empty(),
        "grovedb integrity verification reported issues: {issues:?}"
    );
}
