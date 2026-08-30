//! End-to-end coverage for **prefix-level ranked count indexes**
//! (`rankedCountable: { "at": … }`, meta schema v3 / PV14): the Count-axis
//! indexed tree placed at a NON-terminal index level, ranking that
//! property's values by their **whole-subtree** document count instead of
//! ranking the terminal level's groups by direct member count.
//!
//! Everything runs against the `trending` fixture at
//! `tests/supporting_files/contract/trending/trending-contract.json`:
//!
//! | doctype | index                 | properties                    | at        | exercises                                   |
//! |---------|-----------------------|-------------------------------|-----------|---------------------------------------------|
//! | `like`  | `byHashtagPost`       | `[hashtag, postId]`           | `hashtag` | the standard two-property shape             |
//! | `deep`  | `byHashtagRegionPost` | `[hashtag, region, postId]`   | `hashtag` | a count-propagating level between at/terminal |
//! | `mid`   | `byRegionHashtagPost` | `[region, hashtag, postId]`   | `hashtag` | grouping at a middle level, materialized lazily per prefix |
//! | `ilike` | `byHashtagPost`       | `[hashtag, postId]` → `$ownerId` | `hashtag` | the indexOnly entries-as-rows shape      |
//!
//! The on-disk chain under test, for `like`:
//!
//! ```text
//! "hashtag"  property-name tree   ProvableCountIndexedTree   ← the grouping tree
//!   <value>  value tree           CountTree (count = subtree total)
//!     "postId" property-name tree ProvableCountTree (contributes its count!)
//!       <value> value tree        CountTree
//!         [0]  member bucket      CountTree of references / Items
//! ```
//!
//! Two layers are checked, mirroring `ranked_index_e2e_tests`: the *shape*
//! (registration and lazy materialization lay down exactly these tree
//! types, with the continuation inserted contributing rather than
//! zero-wrapped) and the *behaviour* (inserts, updates, deletes and
//! drains keep the hashtag-level secondary ranking by subtree totals,
//! with grovedb's integrity sweep clean after every test).

use crate::drive::Drive;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use grovedb::Element;

fn platform_version() -> &'static PlatformVersion {
    PlatformVersion::latest()
}

fn setup_trending() -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/trending/trending-contract.json",
        false,
        pv,
    )
    .expect("expected to parse the trending contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the trending contract");
    (drive, contract)
}

/// `[DataContractDocuments, contract_id, 1, doctype, segments…]`.
fn level_path(
    contract: &DataContract,
    document_type_name: &str,
    segments: &[&[u8]],
) -> Vec<Vec<u8>> {
    let mut path = vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        document_type_name.as_bytes().to_vec(),
    ];
    path.extend(segments.iter().map(|segment| segment.to_vec()));
    path
}

fn read_grove_element(drive: &Drive, path: &[Vec<u8>], key: &[u8]) -> Option<Element> {
    let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
    drive
        .grove_get_raw_optional(
            path_refs.as_slice().into(),
            key,
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &platform_version().drive,
        )
        .expect("grove_get_raw_optional should succeed")
}

/// grovedb's integrity sweep, including the primary↔secondary
/// content-consistency walk for indexed trees.
fn assert_grovedb_is_consistent(drive: &Drive) {
    let issues = drive
        .grove
        .verify_grovedb(None, true, false, &platform_version().drive.grove_version)
        .expect("verify_grovedb must run");
    assert!(
        issues.is_empty(),
        "grovedb integrity verification reported issues: {issues:?}"
    );
}

/// A document of `document_type_name` with exactly the given text
/// properties, owned by `owner` when given (indexOnly entries key their
/// member rows by `$ownerId`, so those tests need distinct owners).
fn build_row(
    contract: &DataContract,
    document_type_name: &str,
    properties: &[(&str, &str)],
    owner: Option<[u8; 32]>,
    seed: u64,
) -> Document {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(document_type_name)
        .unwrap_or_else(|_| panic!("{document_type_name} doctype exists"));
    let mut doc = document_type
        .random_document(Some(seed), pv)
        .expect("random document");
    let mut props = std::collections::BTreeMap::new();
    for (name, value) in properties {
        props.insert(name.to_string(), Value::Text(value.to_string()));
    }
    doc.set_properties(props);
    if let Some(owner) = owner {
        doc.set_owner_id(Identifier::from(owner));
    }
    doc
}

fn insert_row(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    doc: &Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(document_type_name)
        .unwrap_or_else(|_| panic!("{document_type_name} doctype exists"));
    drive.add_document_for_contract(
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((doc, None)),
                owner_id: None,
            },
            contract,
            document_type,
        },
        false,
        BlockInfo::default(),
        apply,
        None,
        pv,
        None,
    )
}

fn insert_rows(
    drive: &Drive,
    contract: &DataContract,
    document_type_name: &str,
    rows: &[&[(&str, &str)]],
) -> Vec<Document> {
    rows.iter()
        .enumerate()
        .map(|(i, properties)| {
            let doc = build_row(contract, document_type_name, properties, None, i as u64 + 1);
            insert_row(drive, contract, document_type_name, &doc, true)
                .unwrap_or_else(|e| panic!("expected to insert {document_type_name} row: {e}"));
            doc
        })
        .collect()
}

fn count_top_k(drive: &Drive, path: &[Vec<u8>], k: u16, descending: bool) -> Vec<(u64, Vec<u8>)> {
    let path_query = grovedb::PathQuery::new_axis(
        path.to_vec(),
        grovedb_query::AxisQuery::top_k(grovedb_query::IndexAxis::Count, k, 0, descending)
            .keys_only(),
    );
    match drive
        .grove
        .run_path_query(
            &path_query,
            true,
            true,
            true,
            grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType,
            None,
            &platform_version().drive.grove_version,
        )
        .unwrap()
        .expect("the keys-only axis read must succeed")
    {
        grovedb::PathQueryRun::AxisKeys {
            keys: grovedb::AxisKeys::Count(pairs),
            ..
        } => pairs,
        other => panic!("expected count keys, got {other:?}"),
    }
}

/// `(count, group)` pairs with the group keys decoded for readable
/// assertions.
fn count_top_k_named(
    drive: &Drive,
    path: &[Vec<u8>],
    k: u16,
    descending: bool,
) -> Vec<(u64, String)> {
    count_top_k(drive, path, k, descending)
        .into_iter()
        .map(|(count, key)| {
            (
                count,
                String::from_utf8(key).expect("group keys are utf-8 in this suite"),
            )
        })
        .collect()
}

fn named(pairs: &[(u64, &str)]) -> Vec<(u64, String)> {
    pairs
        .iter()
        .map(|(count, name)| (*count, name.to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// Shape
// ---------------------------------------------------------------------------

/// A compound index ranked at its FIRST property must get its grouping
/// tree — the single-axis `ProvableCountIndexedTree` — at contract
/// registration, exactly where an unranked compound index gets a
/// `NormalTree`. A middle-level `at` leaves the top level normal (its
/// grouping tree is materialized lazily, per prefix, by the walkers).
#[test]
fn grouping_first_level_is_a_pcit_at_registration() {
    let (drive, contract) = setup_trending();

    for doctype in ["like", "deep", "ilike"] {
        let path = level_path(&contract, doctype, &[]);
        match read_grove_element(&drive, &path, b"hashtag") {
            Some(Element::ProvableCountIndexedTree(
                primary_root_key,
                secondary_root_key,
                count,
                _,
            )) => {
                assert!(primary_root_key.is_none(), "{doctype}: fresh primary");
                assert!(secondary_root_key.is_none(), "{doctype}: fresh secondary");
                assert_eq!(count, 0, "{doctype}: fresh grouping tree counts nothing");
            }
            other => panic!("{doctype}: expected ProvableCountIndexedTree, got {other:?}"),
        }
    }

    let mid_path = level_path(&contract, "mid", &[]);
    match read_grove_element(&drive, &mid_path, b"region") {
        Some(Element::Tree(..)) => {}
        other => panic!("mid: the level above the grouping stays a NormalTree, got {other:?}"),
    }

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Behaviour: the two-property shape
// ---------------------------------------------------------------------------

/// Inserting documents through the ordinary document-insert path must key
/// the hashtag-level secondary by each hashtag's whole-subtree count —
/// counted across posts — and lay the chain down with the documented tree
/// types, the continuation contributing its count to the value tree.
#[test]
fn likes_rank_hashtags_by_whole_subtree_count() {
    let (drive, contract) = setup_trending();

    insert_rows(
        &drive,
        &contract,
        "like",
        &[
            &[("hashtag", "alpha"), ("postId", "p1")],
            &[("hashtag", "alpha"), ("postId", "p1")],
            &[("hashtag", "alpha"), ("postId", "p2")],
            &[("hashtag", "beta"), ("postId", "p1")],
            &[("hashtag", "beta"), ("postId", "p3")],
            &[("hashtag", "gamma"), ("postId", "p2")],
        ],
    );

    let grouping_path = level_path(&contract, "like", &[b"hashtag"]);
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(3, "alpha"), (2, "beta"), (1, "gamma")]),
        "hashtags must rank by total like count across all their posts"
    );
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 2, false),
        named(&[(1, "gamma"), (2, "beta")]),
        "bottom-k reads the same secondary from the other end"
    );

    // The chain's tree types, level by level under `alpha`.
    match read_grove_element(&drive, &grouping_path, b"alpha") {
        Some(element @ Element::CountTree(..)) => {
            assert_eq!(
                element.count_value_or_default(),
                3,
                "alpha's value tree count must be its subtree total"
            );
        }
        other => panic!("expected a CountTree value tree for alpha, got {other:?}"),
    }
    let alpha_path = level_path(&contract, "like", &[b"hashtag", b"alpha"]);
    match read_grove_element(&drive, &alpha_path, b"postId") {
        Some(element @ Element::ProvableCountTree(..)) => {
            assert_eq!(
                element.count_value_or_default(),
                3,
                "the contributing continuation carries the same total upward"
            );
        }
        other => panic!("expected a contributing ProvableCountTree continuation, got {other:?}"),
    }
    let alpha_posts_path = level_path(&contract, "like", &[b"hashtag", b"alpha", b"postId"]);
    match read_grove_element(&drive, &alpha_posts_path, b"p1") {
        Some(element @ Element::CountTree(..)) => {
            assert_eq!(element.count_value_or_default(), 2);
        }
        other => panic!("expected a CountTree post value tree, got {other:?}"),
    }

    assert_grovedb_is_consistent(&drive);
}

/// Changing a document's hashtag must move its contribution between
/// groups through the real update path — decrementing (and re-keying) the
/// old group, incrementing the new one.
#[test]
fn updating_a_hashtag_moves_its_document_between_groups() {
    let (drive, contract) = setup_trending();
    let pv = platform_version();

    let docs = insert_rows(
        &drive,
        &contract,
        "like",
        &[
            &[("hashtag", "alpha"), ("postId", "p1")],
            &[("hashtag", "alpha"), ("postId", "p2")],
            &[("hashtag", "beta"), ("postId", "p1")],
        ],
    );

    let grouping_path = level_path(&contract, "like", &[b"hashtag"]);
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(2, "alpha"), (1, "beta")])
    );

    // Move beta's only like to alpha: beta drains, alpha reaches 3.
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let mut updated = docs[2].clone();
    let mut props = updated.properties().clone();
    props.insert("hashtag".to_string(), Value::Text("alpha".to_string()));
    updated.set_properties(props);
    updated.set_revision(Some(2));

    drive
        .update_document_for_contract(
            &updated,
            &contract,
            document_type,
            Some(updated.owner_id().to_buffer()),
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
            None,
        )
        .expect("expected to update the like document");

    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(3, "alpha")]),
        "the moved like must drain beta and grow alpha's subtree total"
    );

    assert_grovedb_is_consistent(&drive);
}

/// Deleting a group's last document must drop the group from the axis
/// entirely; earlier deletes only decrement it.
#[test]
fn draining_a_hashtag_removes_it_from_the_ranking() {
    let (drive, contract) = setup_trending();
    let pv = platform_version();

    let docs = insert_rows(
        &drive,
        &contract,
        "like",
        &[
            &[("hashtag", "alpha"), ("postId", "p1")],
            &[("hashtag", "beta"), ("postId", "p1")],
            &[("hashtag", "beta"), ("postId", "p2")],
        ],
    );

    let grouping_path = level_path(&contract, "like", &[b"hashtag"]);
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(2, "beta"), (1, "alpha")])
    );

    for (doc_index, expected) in [
        (2usize, named(&[(1, "alpha"), (1, "beta")])),
        (1usize, named(&[(1, "alpha")])),
    ] {
        drive
            .delete_document_for_contract(
                docs[doc_index].id(),
                &contract,
                "like",
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to delete the like document");
        let mut after = count_top_k_named(&drive, &grouping_path, 10, true);
        // Equal counts tie; compare as sets of (count, name).
        after.sort();
        let mut expected = expected;
        expected.sort();
        assert_eq!(after, expected);
    }

    assert!(
        read_grove_element(&drive, &grouping_path, b"beta").is_none(),
        "the drained group's value tree must be pruned"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The estimation (dry-run) path must traverse the same layouts without
/// erroring and price real work — a stateless walker claiming the wrong
/// tree types would fail or misprice here.
#[test]
fn dry_run_insert_estimates_fees_for_the_prefix_ranked_layout() {
    let (drive, contract) = setup_trending();

    for doctype in ["like", "deep", "mid", "ilike"] {
        let properties: &[(&str, &str)] = match doctype {
            "like" | "ilike" => &[("hashtag", "alpha"), ("postId", "p1")],
            _ => &[("hashtag", "alpha"), ("region", "east"), ("postId", "p1")],
        };
        let doc = build_row(&contract, doctype, properties, Some([7u8; 32]), 1);
        let estimated = insert_row(&drive, &contract, doctype, &doc, false)
            .unwrap_or_else(|e| panic!("{doctype}: dry-run insert must succeed: {e}"));
        assert!(
            estimated.processing_fee > 0,
            "{doctype}: the dry run must price the write"
        );
        let applied = insert_row(&drive, &contract, doctype, &doc, true)
            .unwrap_or_else(|e| panic!("{doctype}: applied insert must succeed: {e}"));
        assert!(applied.processing_fee > 0);
    }

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Behaviour: deeper shapes
// ---------------------------------------------------------------------------

/// With a level between `at` and the terminal, the chain runs through a
/// count-propagating `CountTree` property-name tree — the grouping
/// secondary must still see the whole-subtree totals.
#[test]
fn deep_chain_propagates_counts_through_the_propagating_level() {
    let (drive, contract) = setup_trending();

    insert_rows(
        &drive,
        &contract,
        "deep",
        &[
            &[("hashtag", "alpha"), ("region", "east"), ("postId", "p1")],
            &[("hashtag", "alpha"), ("region", "east"), ("postId", "p2")],
            &[("hashtag", "alpha"), ("region", "west"), ("postId", "p1")],
            &[("hashtag", "beta"), ("region", "east"), ("postId", "p1")],
        ],
    );

    let grouping_path = level_path(&contract, "deep", &[b"hashtag"]);
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(3, "alpha"), (1, "beta")]),
        "the grouping secondary must rank by totals across regions AND posts"
    );

    // The propagating level's property-name tree is a contributing
    // CountTree carrying alpha's total.
    let alpha_path = level_path(&contract, "deep", &[b"hashtag", b"alpha"]);
    match read_grove_element(&drive, &alpha_path, b"region") {
        Some(element @ Element::CountTree(..)) => {
            assert_eq!(element.count_value_or_default(), 3);
        }
        other => panic!("expected a CountTree propagating level, got {other:?}"),
    }
    let alpha_regions_path = level_path(&contract, "deep", &[b"hashtag", b"alpha", b"region"]);
    match read_grove_element(&drive, &alpha_regions_path, b"east") {
        Some(element @ Element::CountTree(..)) => {
            assert_eq!(element.count_value_or_default(), 2);
        }
        other => panic!("expected a CountTree region value tree, got {other:?}"),
    }

    assert_grovedb_is_consistent(&drive);
}

/// A middle-level `at` keeps one grouping tree PER prefix value,
/// materialized lazily by the walkers, each ranking only its own
/// prefix's hashtags.
#[test]
fn middle_at_ranks_hashtags_within_each_region_prefix() {
    let (drive, contract) = setup_trending();

    insert_rows(
        &drive,
        &contract,
        "mid",
        &[
            &[("region", "r1"), ("hashtag", "alpha"), ("postId", "p1")],
            &[("region", "r1"), ("hashtag", "alpha"), ("postId", "p2")],
            &[("region", "r1"), ("hashtag", "beta"), ("postId", "p1")],
            &[("region", "r2"), ("hashtag", "beta"), ("postId", "p1")],
        ],
    );

    // One grouping PCIT per region value.
    let r1_value_path = level_path(&contract, "mid", &[b"region", b"r1"]);
    match read_grove_element(&drive, &r1_value_path, b"hashtag") {
        Some(Element::ProvableCountIndexedTree(_, _, count, _)) => {
            assert_eq!(count, 3, "r1's grouping tree counts r1's likes only");
        }
        other => panic!("expected a lazily-created ProvableCountIndexedTree, got {other:?}"),
    }

    let r1_grouping_path = level_path(&contract, "mid", &[b"region", b"r1", b"hashtag"]);
    assert_eq!(
        count_top_k_named(&drive, &r1_grouping_path, 10, true),
        named(&[(2, "alpha"), (1, "beta")])
    );
    let r2_grouping_path = level_path(&contract, "mid", &[b"region", b"r2", b"hashtag"]);
    assert_eq!(
        count_top_k_named(&drive, &r2_grouping_path, 10, true),
        named(&[(1, "beta")]),
        "each prefix ranks independently"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Behaviour: indexOnly entries
// ---------------------------------------------------------------------------

/// The indexOnly shape from the motivating issue: entries are the rows,
/// keyed per owner, and the hashtag level ranks by total like count.
/// Delete-by-values decrements and drains exactly like references do.
#[test]
fn index_only_entries_rank_hashtags_and_drain_on_delete() {
    let (drive, contract) = setup_trending();
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("ilike")
        .expect("ilike doctype exists");

    let rows: [(&str, &str, [u8; 32]); 4] = [
        ("alpha", "p1", [1u8; 32]),
        ("alpha", "p1", [2u8; 32]),
        ("alpha", "p2", [1u8; 32]),
        ("beta", "p1", [1u8; 32]),
    ];
    let docs: Vec<Document> = rows
        .iter()
        .enumerate()
        .map(|(i, (hashtag, post, owner))| {
            let doc = build_row(
                &contract,
                "ilike",
                &[("hashtag", hashtag), ("postId", post)],
                Some(*owner),
                i as u64 + 1,
            );
            insert_row(&drive, &contract, "ilike", &doc, true)
                .unwrap_or_else(|e| panic!("expected to insert ilike entry: {e}"));
            doc
        })
        .collect();

    let grouping_path = level_path(&contract, "ilike", &[b"hashtag"]);
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(3, "alpha"), (1, "beta")])
    );

    // Structural uniqueness stays per (hashtag, postId, owner): the same
    // owner liking the same post again is refused, but the same owner
    // across posts (rows 0 and 2) was fine.
    let duplicate = build_row(
        &contract,
        "ilike",
        &[("hashtag", "alpha"), ("postId", "p1")],
        Some([1u8; 32]),
        9,
    );
    assert!(
        insert_row(&drive, &contract, "ilike", &duplicate, true).is_err(),
        "a duplicate (hashtag, post, owner) entry must be refused"
    );
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(3, "alpha"), (1, "beta")]),
        "the refused duplicate must not have moved any count"
    );

    // Delete one alpha entry, then drain beta.
    drive
        .delete_index_only_document_for_contract(
            docs[1].clone(),
            &contract,
            document_type,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to delete the ilike entry");
    let mut after = count_top_k_named(&drive, &grouping_path, 10, true);
    after.sort();
    assert_eq!(after, named(&[(1, "beta"), (2, "alpha")]));

    drive
        .delete_index_only_document_for_contract(
            docs[3].clone(),
            &contract,
            document_type,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to delete beta's only entry");
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(2, "alpha")]),
        "beta must drain out of the ranking"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Behaviour: both rankings on one index (`at: ["hashtag", "postId"]`)
// ---------------------------------------------------------------------------

/// The `both` doctype declares the grouping level AND the terminal
/// ranking on one index: the chain nests two indexed trees, the inner
/// (terminal) one a CONTRIBUTING child inside the grouping level's
/// count-bearing value trees. Registration lays the outer one down;
/// document inserts materialize the inner one per hashtag; both
/// secondaries rank simultaneously and re-key on delete, drains prune
/// through both, and the dry-run prices the nested layout.
#[test]
fn both_levels_rank_on_one_index() {
    let (drive, contract) = setup_trending();
    let pv = platform_version();

    // Registration: the grouping first level is the PCIT.
    let doctype_path = level_path(&contract, "both", &[]);
    assert!(
        matches!(
            read_grove_element(&drive, &doctype_path, b"hashtag"),
            Some(Element::ProvableCountIndexedTree(..))
        ),
        "registration must create the grouping PCIT"
    );

    // Dry-run estimation traverses the nested layout before any state
    // exists.
    let dry_run_doc = build_row(
        &contract,
        "both",
        &[("hashtag", "alpha"), ("postId", "p1")],
        None,
        99,
    );
    let estimated = insert_row(&drive, &contract, "both", &dry_run_doc, false)
        .expect("the dry run must traverse the nested layout");
    assert!(estimated.processing_fee > 0);

    let docs = insert_rows(
        &drive,
        &contract,
        "both",
        &[
            &[("hashtag", "alpha"), ("postId", "p1")],
            &[("hashtag", "alpha"), ("postId", "p1")],
            &[("hashtag", "alpha"), ("postId", "p2")],
            &[("hashtag", "beta"), ("postId", "p1")],
        ],
    );

    // The inner (terminal) tree is ALSO an indexed tree, contributing
    // its count to the hashtag value tree above it.
    let grouping_path = level_path(&contract, "both", &[b"hashtag"]);
    match read_grove_element(&drive, &grouping_path, b"alpha") {
        Some(element @ Element::CountTree(..)) => {
            assert_eq!(
                element.count_value_or_default(),
                3,
                "alpha's value tree must receive the inner PCIT's contribution"
            );
        }
        other => panic!("expected a CountTree value tree for alpha, got {other:?}"),
    }
    let alpha_path = level_path(&contract, "both", &[b"hashtag", b"alpha"]);
    match read_grove_element(&drive, &alpha_path, b"postId") {
        Some(Element::ProvableCountIndexedTree(_, _, count, _)) => {
            assert_eq!(
                count, 3,
                "the contributing inner PCIT carries alpha's total"
            );
        }
        other => panic!("expected a contributing inner PCIT, got {other:?}"),
    }

    // Both secondaries serve their rankings.
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(3, "alpha"), (1, "beta")]),
        "the grouping secondary ranks hashtags by subtree total"
    );
    let alpha_posts_path = level_path(&contract, "both", &[b"hashtag", b"alpha", b"postId"]);
    assert_eq!(
        count_top_k_named(&drive, &alpha_posts_path, 10, true),
        named(&[(2, "p1"), (1, "p2")]),
        "the terminal secondary ranks alpha's posts by member count"
    );

    // A delete re-keys both secondaries.
    drive
        .delete_document_for_contract(
            docs[0].id(),
            &contract,
            "both",
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to delete a like");
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(2, "alpha"), (1, "beta")])
    );
    let mut alpha_after = count_top_k_named(&drive, &alpha_posts_path, 10, true);
    alpha_after.sort();
    assert_eq!(alpha_after, named(&[(1, "p1"), (1, "p2")]));

    // Draining beta prunes its whole chain — the group leaves the
    // grouping secondary.
    drive
        .delete_document_for_contract(
            docs[3].id(),
            &contract,
            "both",
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to delete beta's only like");
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(2, "alpha")])
    );
    assert!(
        read_grove_element(&drive, &grouping_path, b"beta").is_none(),
        "beta's drained chain must be pruned"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The `chain` doctype ranks EVERY level of `[tag, region, postId]`:
/// three nested indexed trees, each a contributing child of the level
/// above. All three secondaries rank simultaneously through the real
/// walkers, one delete re-keys all three, and the integrity sweep
/// stays clean — the unbounded-depth composition, end to end.
#[test]
fn every_level_of_an_index_can_rank() {
    let (drive, contract) = setup_trending();
    let pv = platform_version();

    let dry_run_doc = build_row(
        &contract,
        "chain",
        &[("tag", "alpha"), ("region", "east"), ("postId", "p1")],
        None,
        99,
    );
    let estimated = insert_row(&drive, &contract, "chain", &dry_run_doc, false)
        .expect("the dry run must traverse the fully ranked layout");
    assert!(estimated.processing_fee > 0);

    let docs = insert_rows(
        &drive,
        &contract,
        "chain",
        &[
            &[("tag", "alpha"), ("region", "east"), ("postId", "p1")],
            &[("tag", "alpha"), ("region", "east"), ("postId", "p1")],
            &[("tag", "alpha"), ("region", "east"), ("postId", "p2")],
            &[("tag", "alpha"), ("region", "west"), ("postId", "p1")],
            &[("tag", "beta"), ("region", "east"), ("postId", "p1")],
        ],
    );

    // Every level is an indexed tree; every level ranks.
    let tag_path = level_path(&contract, "chain", &[b"tag"]);
    assert!(matches!(
        read_grove_element(&drive, &level_path(&contract, "chain", &[]), b"tag"),
        Some(Element::ProvableCountIndexedTree(..))
    ));
    assert_eq!(
        count_top_k_named(&drive, &tag_path, 10, true),
        named(&[(4, "alpha"), (1, "beta")])
    );
    let alpha_regions_path = level_path(&contract, "chain", &[b"tag", b"alpha", b"region"]);
    assert!(matches!(
        read_grove_element(
            &drive,
            &level_path(&contract, "chain", &[b"tag", b"alpha"]),
            b"region"
        ),
        Some(Element::ProvableCountIndexedTree(..))
    ));
    assert_eq!(
        count_top_k_named(&drive, &alpha_regions_path, 10, true),
        named(&[(3, "east"), (1, "west")])
    );
    let alpha_east_posts_path = level_path(
        &contract,
        "chain",
        &[b"tag", b"alpha", b"region", b"east", b"postId"],
    );
    assert_eq!(
        count_top_k_named(&drive, &alpha_east_posts_path, 10, true),
        named(&[(2, "p1"), (1, "p2")])
    );

    // One delete re-keys all three secondaries on its path.
    drive
        .delete_document_for_contract(
            docs[0].id(),
            &contract,
            "chain",
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to delete a chain document");
    assert_eq!(
        count_top_k_named(&drive, &tag_path, 10, true),
        named(&[(3, "alpha"), (1, "beta")])
    );
    assert_eq!(
        count_top_k_named(&drive, &alpha_regions_path, 10, true),
        named(&[(2, "east"), (1, "west")])
    );
    let mut posts_after = count_top_k_named(&drive, &alpha_east_posts_path, 10, true);
    posts_after.sort();
    assert_eq!(posts_after, named(&[(1, "p1"), (1, "p2")]));

    assert_grovedb_is_consistent(&drive);
}

/// The `preallocated` composition the issue calls out: inserting the
/// referenced `post` preallocates the `plike` chain — the group appears
/// in the hashtag-level secondary **at zero before any like exists** —
/// and draining the entries keeps it rankable at zero instead of
/// pruning it (the non-preallocated drain behaviour is pinned above).
#[test]
fn preallocated_groups_stay_rankable_at_zero() {
    let (drive, contract) = setup_trending();
    let pv = platform_version();

    // Insert the referenced post; preallocation creates plike's chain.
    let post_type = contract
        .document_type_for_name("post")
        .expect("post doctype exists");
    let mut post = post_type
        .random_document(Some(400), pv)
        .expect("random post");
    let mut props = std::collections::BTreeMap::new();
    props.insert("hashtag".to_string(), Value::Text("alpha".to_string()));
    props.insert("message".to_string(), Value::Text("gm".to_string()));
    post.set_properties(props);
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&post, None)),
                    owner_id: None,
                },
                contract: &contract,
                document_type: post_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to insert the post");

    let grouping_path = level_path(&contract, "plike", &[b"hashtag"]);
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(0, "alpha")]),
        "preallocation must surface the group in the secondary at zero, before any like"
    );

    // Two likes on the post, then drain them both.
    let plike_type = contract
        .document_type_for_name("plike")
        .expect("plike doctype exists");
    let likes: Vec<Document> = [[1u8; 32], [2u8; 32]]
        .into_iter()
        .enumerate()
        .map(|(i, owner)| {
            let mut like = plike_type
                .random_document(Some(500 + i as u64), pv)
                .expect("random like");
            let mut props = std::collections::BTreeMap::new();
            props.insert("hashtag".to_string(), Value::Text("alpha".to_string()));
            props.insert(
                "postId".to_string(),
                Value::Identifier(post.id().to_buffer()),
            );
            like.set_properties(props);
            like.set_owner_id(Identifier::from(owner));
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&like, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type: plike_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    pv,
                    None,
                )
                .expect("expected to insert a plike entry");
            like
        })
        .collect();
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(2, "alpha")])
    );

    for like in likes {
        drive
            .delete_index_only_document_for_contract(
                like,
                &contract,
                plike_type,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to delete the plike entry");
    }
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(0, "alpha")]),
        "draining a preallocated group must keep it rankable at zero, not prune it"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The contract-insert **estimation** path (a dry-run `apply_contract`)
/// must price the registration of grouping first-level trees — the
/// branch that tallies the at-level PCIT into the doctype layer's
/// estimated weights.
#[test]
fn dry_run_contract_apply_prices_the_grouping_registration() {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/trending/trending-contract.json",
        false,
        pv,
    )
    .expect("expected to parse the trending contract");

    let estimated = drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            false,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("the dry-run contract apply must traverse the estimation path");
    assert!(estimated.processing_fee > 0);

    let applied = drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("the real apply must succeed after the dry run");
    assert!(applied.processing_fee > 0);
}
