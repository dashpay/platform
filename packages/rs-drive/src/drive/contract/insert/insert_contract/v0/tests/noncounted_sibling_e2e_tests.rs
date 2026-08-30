//! End-to-end coverage for **count-exempt sibling indexes** beside a
//! prefix-level ranking (`rankedCountable: { at }` + a PLAIN sibling
//! index sharing the `at` level and continuing below it, meta schema
//! v3 / PV14).
//!
//! Everything runs against the `trending-sibling` fixture at
//! `tests/supporting_files/contract/trending/trending-sibling-contract.json`:
//!
//! | doctype | ranked index (at)                     | sibling                         | exercises                              |
//! |---------|---------------------------------------|---------------------------------|----------------------------------------|
//! | `like`  | `[postAuthor, postId]` (`postAuthor`) | `[postAuthor, day, postId]`     | the yappr indexOnly shape              |
//! | `slike` | same, stored                          | same                            | stored insert / update / delete walkers|
//! | `deep`  | `[tag, region, postId]` (`tag`)       | `[tag, region, day]`            | branch at a count-propagating level    |
//! | `ext`   | `[postAuthor, postId]` (`postAuthor`) | `[postAuthor, postId, day]`     | branch below the ranked terminal       |
//! | `plike` | preallocated, refersTo `post`         | `[hashtag, day, postId]`        | preallocated chain + lazy sibling      |
//!
//! The on-disk shape under test, for `like`:
//!
//! ```text
//! "postAuthor" property-name tree  ProvableCountIndexedTree   ← the grouping tree
//!   <alice>    value tree          CountTree (count = alice's real likes, exact)
//!     "postId" property-name tree  ProvableCountTree            ← chain: contributes
//!       …
//!     "day"    property-name tree  NonCounted(Tree)             ← sibling: readable,
//!       …                                                          count-invisible
//! ```
//!
//! Both layers are checked: the *shape* (the sibling branch is stored
//! `Element::NonCounted`-wrapped, the chain stays contributing) and the
//! *behaviour* (counts stay exact — N, never 2N — sibling reads serve
//! and prove, deletes are symmetric, sibling-only writes never re-key
//! the ranking secondary, preallocated groups stay rankable at zero
//! with the sibling present, and grovedb's integrity sweep is clean
//! after every test).

use crate::drive::Drive;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::{Element, GroveDb};

const FIXTURE_PATH: &str =
    "tests/supporting_files/contract/trending/trending-sibling-contract.json";

fn platform_version() -> &'static PlatformVersion {
    PlatformVersion::latest()
}

fn setup_sibling_contract() -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let contract = json_document_to_contract(FIXTURE_PATH, false, pv)
        .expect("expected to parse the trending-sibling contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the trending-sibling contract");
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

/// The stored element at `path`/`key` must be the sibling branch's
/// `NonCounted(Tree)` wrapper — the count-exempt layout under test.
fn assert_branch_is_non_counted(drive: &Drive, path: &[Vec<u8>], key: &[u8], context: &str) {
    match read_grove_element(drive, path, key) {
        Some(Element::NonCounted(inner)) => {
            assert!(
                matches!(inner.as_ref(), Element::Tree(..)),
                "{context}: the wrapped sibling branch must be a plain tree, got {inner:?}"
            );
        }
        other => panic!("{context}: expected NonCounted(Tree), got {other:?}"),
    }
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

fn count_top_k_named(
    drive: &Drive,
    path: &[Vec<u8>],
    k: u16,
    descending: bool,
) -> Vec<(u64, String)> {
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
        } => pairs
            .into_iter()
            .map(|(count, key)| {
                (
                    count,
                    String::from_utf8(key).expect("group keys are utf-8 in this suite"),
                )
            })
            .collect(),
        other => panic!("expected count keys, got {other:?}"),
    }
}

fn named(pairs: &[(u64, &str)]) -> Vec<(u64, String)> {
    pairs
        .iter()
        .map(|(count, name)| (*count, name.to_string()))
        .collect()
}

/// Parse a (possibly mutated) fixture JSON through the same
/// non-validating serialization-format path `json_document_to_contract`
/// uses — the structural ranked-overlap rule is unconditional, so it
/// must fire on this path too.
fn contract_from_json_value(value: serde_json::Value) -> Result<DataContract, ProtocolError> {
    let format: DataContractInSerializationFormat =
        serde_json::from_value(value).map_err(|e| ProtocolError::DecodingError(e.to_string()))?;
    DataContract::try_from_platform_versioned(format, false, &mut vec![], platform_version())
}

fn fixture_json() -> serde_json::Value {
    let file = std::fs::File::open(FIXTURE_PATH).expect("fixture file exists");
    serde_json::from_reader(std::io::BufReader::new(file)).expect("fixture parses as JSON")
}

// ---------------------------------------------------------------------------
// Registration: the accept / reject matrix
// ---------------------------------------------------------------------------

/// The fixture itself — every doctype carrying the plain continuing
/// sibling — must parse and register (it is applied by every test's
/// setup; this pins the parse in isolation, on the same code path the
/// mutation rejects below run through).
#[test]
fn registration_admits_the_plain_continuing_sibling() {
    contract_from_json_value(fixture_json())
        .expect("the plain continuing sibling shape must be admitted");

    let (drive, contract) = setup_sibling_contract();
    // Registration creates the grouping PCIT with the sibling declared.
    let path = level_path(&contract, "like", &[]);
    assert!(
        matches!(
            read_grove_element(&drive, &path, b"postAuthor"),
            Some(Element::ProvableCountIndexedTree(..))
        ),
        "registration must create the grouping PCIT with the sibling present"
    );
    assert_grovedb_is_consistent(&drive);
}

/// Everything the shape matrix keeps rejecting must still reject, with
/// the (updated) exclusivity error: aggregating / range / ranked
/// siblings, and a plain sibling terminating exactly at the `at` level.
#[test]
fn registration_still_rejects_non_plain_and_exact_at_siblings() {
    fn sibling(json: &mut serde_json::Value) -> &mut serde_json::Value {
        &mut json["documentSchemas"]["like"]["indices"][1]
    }

    // Aggregating sibling: countable.
    let mut countable = fixture_json();
    sibling(&mut countable)["countable"] = serde_json::json!("countable");
    assert!(
        contract_from_json_value(countable).is_err(),
        "a countable sibling must stay rejected"
    );

    // Range-countable sibling.
    let mut range_countable = fixture_json();
    sibling(&mut range_countable)["countable"] = serde_json::json!("countable");
    sibling(&mut range_countable)["rangeCountable"] = serde_json::json!(true);
    assert!(
        contract_from_json_value(range_countable).is_err(),
        "a range-countable sibling must stay rejected"
    );

    // A sibling carrying its own prefix ranking.
    let mut ranked = fixture_json();
    sibling(&mut ranked)["countable"] = serde_json::json!("countable");
    sibling(&mut ranked)["rangeCountable"] = serde_json::json!(true);
    sibling(&mut ranked)["rankedCountable"] = serde_json::json!({ "at": "day" });
    assert!(
        contract_from_json_value(ranked).is_err(),
        "a sibling with its own ranking must stay rejected"
    );

    // A plain sibling terminating EXACTLY at the at level.
    let mut exact_at = fixture_json();
    sibling(&mut exact_at)["properties"] = serde_json::json!([{ "postAuthor": "asc" }]);
    assert!(
        contract_from_json_value(exact_at).is_err(),
        "an exact-at plain terminator must stay rejected (possible follow-up, not this change)"
    );
}

// ---------------------------------------------------------------------------
// Behaviour: the yappr indexOnly shape
// ---------------------------------------------------------------------------

/// N likes → the ranking counts exactly N per author (never 2N), the
/// sibling branch is stored `NonCounted`-wrapped, its entries read
/// back, and unliking decrements exactly 1 while removing the sibling
/// entry — the delete symmetry.
#[test]
fn sibling_entries_never_pollute_the_ranking_and_deletes_are_symmetric() {
    let (drive, contract) = setup_sibling_contract();
    let pv = platform_version();
    let like_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");

    let rows: [(&str, &str, &str, [u8; 32]); 5] = [
        ("alice", "d1", "p1", [1u8; 32]),
        ("alice", "d1", "p1", [2u8; 32]),
        ("alice", "d2", "p2", [1u8; 32]),
        ("bob", "d1", "p1", [1u8; 32]),
        ("bob", "d2", "p3", [2u8; 32]),
    ];
    let docs: Vec<Document> = rows
        .iter()
        .enumerate()
        .map(|(i, (author, day, post, owner))| {
            let doc = build_row(
                &contract,
                "like",
                &[("postAuthor", author), ("day", day), ("postId", post)],
                Some(*owner),
                i as u64 + 1,
            );
            insert_row(&drive, &contract, "like", &doc, true)
                .unwrap_or_else(|e| panic!("expected to insert like entry: {e}"));
            doc
        })
        .collect();

    // Exact counts: 3 for alice, 2 for bob — the sibling's entries are
    // invisible to the ranking.
    let grouping_path = level_path(&contract, "like", &[b"postAuthor"]);
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(3, "alice"), (2, "bob")]),
        "counts must be exact — one per like, sibling entries contributing zero"
    );

    // The chain continuation contributes; the sibling branch is wrapped.
    let alice_path = level_path(&contract, "like", &[b"postAuthor", b"alice"]);
    match read_grove_element(&drive, &alice_path, b"postId") {
        Some(element @ Element::ProvableCountTree(..)) => {
            assert_eq!(element.count_value_or_default(), 3);
        }
        other => panic!("expected the contributing chain continuation, got {other:?}"),
    }
    assert_branch_is_non_counted(&drive, &alice_path, b"day", "like/alice");

    // The sibling serves reads: alice's likes on day d1.
    let alice_d1_posts = level_path(
        &contract,
        "like",
        &[b"postAuthor", b"alice", b"day", b"d1", b"postId", b"p1"],
    );
    match read_grove_element(&drive, &alice_d1_posts, &[0]) {
        Some(element @ Element::Tree(..)) => {
            let _ = element;
        }
        other => panic!("expected the sibling member bucket, got {other:?}"),
    }
    let bucket_path = {
        let mut path = alice_d1_posts.clone();
        path.push(vec![0]);
        path
    };
    for owner in [[1u8; 32], [2u8; 32]] {
        assert!(
            matches!(
                read_grove_element(&drive, &bucket_path, &owner),
                Some(Element::Item(..))
            ),
            "the sibling entry for owner {:?} must exist",
            owner[0]
        );
    }

    // Unlike: exactly -1 on the ranking, and the sibling entry is gone.
    drive
        .delete_index_only_document_for_contract(
            docs[1].clone(),
            &contract,
            like_type,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to delete the like entry");
    // Equal counts tie; compare as sorted sets of (count, name).
    let mut after_unlike = count_top_k_named(&drive, &grouping_path, 10, true);
    after_unlike.sort();
    assert_eq!(
        after_unlike,
        named(&[(2, "alice"), (2, "bob")]),
        "an unlike must decrement exactly one"
    );
    assert!(
        read_grove_element(&drive, &bucket_path, &[2u8; 32]).is_none(),
        "the sibling entry must disappear with the unlike"
    );
    assert!(
        matches!(
            read_grove_element(&drive, &bucket_path, &[1u8; 32]),
            Some(Element::Item(..))
        ),
        "the other owner's sibling entry must survive"
    );

    // Drain bob entirely: the group leaves the ranking and the whole
    // chain — sibling branch included — prunes.
    for doc in [docs[3].clone(), docs[4].clone()] {
        drive
            .delete_index_only_document_for_contract(
                doc,
                &contract,
                like_type,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("expected to delete bob's like entry");
    }
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(2, "alice")]),
        "bob must drain out of the ranking"
    );
    assert!(
        read_grove_element(&drive, &grouping_path, b"bob").is_none(),
        "bob's drained value tree (with its sibling branch) must be pruned"
    );

    assert_grovedb_is_consistent(&drive);
}

/// Sibling reads must PROVE: a proved path query through the wrapped
/// branch verifies against the grove root.
#[test]
fn sibling_reads_prove_through_the_wrapped_branch() {
    let (drive, contract) = setup_sibling_contract();
    let pv = platform_version();

    for (i, (author, day, post, owner)) in [
        ("alice", "d1", "p1", [1u8; 32]),
        ("alice", "d1", "p1", [2u8; 32]),
        ("alice", "d2", "p2", [1u8; 32]),
    ]
    .iter()
    .enumerate()
    {
        let doc = build_row(
            &contract,
            "like",
            &[("postAuthor", author), ("day", day), ("postId", post)],
            Some(*owner),
            i as u64 + 1,
        );
        insert_row(&drive, &contract, "like", &doc, true)
            .unwrap_or_else(|e| panic!("expected to insert like entry: {e}"));
    }

    let bucket_path = level_path(
        &contract,
        "like",
        &[
            b"postAuthor",
            b"alice",
            b"day",
            b"d1",
            b"postId",
            b"p1",
            &[0],
        ],
    );
    let mut query = grovedb::Query::new();
    query.insert_all();
    let path_query = grovedb::PathQuery::new_unsized(bucket_path, query);

    let proof = drive
        .grove_get_proved_path_query(&path_query, None, &mut vec![], &pv.drive)
        .expect("the sibling bucket must prove");
    let (_root_hash, elements) =
        GroveDb::verify_query(&proof, &path_query, &pv.drive.grove_version)
            .expect("the sibling proof must verify");
    assert_eq!(
        elements.len(),
        2,
        "both owners' entries under (alice, d1, p1) must be in the proved result"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The estimation (dry-run) path must traverse the sibling layout —
/// including the wrapped branch — and price real work for both the
/// indexOnly and the stored doctype.
#[test]
fn dry_run_insert_estimates_fees_for_the_sibling_layout() {
    let (drive, contract) = setup_sibling_contract();

    for doctype in ["like", "slike", "deep", "ext"] {
        let properties: &[(&str, &str)] = match doctype {
            "deep" => &[
                ("tag", "alpha"),
                ("region", "east"),
                ("postId", "p1"),
                ("day", "d1"),
            ],
            _ => &[("postAuthor", "alice"), ("day", "d1"), ("postId", "p1")],
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
// Behaviour: stored documents (insert / update / delete walkers)
// ---------------------------------------------------------------------------

/// Stored-doctype writes take the update walker too: a day-only update
/// rewrites only the sibling branch and must NOT re-key the ranking
/// secondary (the grouping element is byte-identical after it), while
/// an author move re-keys exactly the chain.
#[test]
fn sibling_only_writes_skip_the_ranked_re_key() {
    let (drive, contract) = setup_sibling_contract();
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("slike")
        .expect("slike doctype exists");

    let docs: Vec<Document> = [
        ("alice", "d1", "p1"),
        ("alice", "d1", "p2"),
        ("bob", "d1", "p1"),
    ]
    .iter()
    .enumerate()
    .map(|(i, (author, day, post))| {
        let doc = build_row(
            &contract,
            "slike",
            &[("postAuthor", author), ("day", day), ("postId", post)],
            None,
            i as u64 + 1,
        );
        insert_row(&drive, &contract, "slike", &doc, true)
            .unwrap_or_else(|e| panic!("expected to insert slike row: {e}"));
        doc
    })
    .collect();

    let doctype_path = level_path(&contract, "slike", &[]);
    let grouping_path = level_path(&contract, "slike", &[b"postAuthor"]);
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(2, "alice"), (1, "bob")])
    );
    let grouping_before = read_grove_element(&drive, &doctype_path, b"postAuthor")
        .expect("the grouping element exists");

    // Day-only update: the chain index's tuple is unchanged, so the
    // grouping tree (primary root, secondary root, count) must be
    // byte-identical — no ranked re-key for a sibling-only write.
    let mut moved_day = docs[0].clone();
    let mut props = moved_day.properties().clone();
    props.insert("day".to_string(), Value::Text("d2".to_string()));
    moved_day.set_properties(props);
    moved_day.set_revision(Some(2));
    drive
        .update_document_for_contract(
            &moved_day,
            &contract,
            document_type,
            Some(moved_day.owner_id().to_buffer()),
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
            None,
        )
        .expect("expected to update the day");
    let grouping_after = read_grove_element(&drive, &doctype_path, b"postAuthor")
        .expect("the grouping element still exists");
    assert_eq!(
        grouping_before, grouping_after,
        "a sibling-only write must not touch the ranking (no re-key, no count change)"
    );
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(2, "alice"), (1, "bob")])
    );

    // The sibling moved: d1 lost the entry, d2 gained it — under a
    // still-wrapped branch.
    let alice_path = level_path(&contract, "slike", &[b"postAuthor", b"alice"]);
    assert_branch_is_non_counted(&drive, &alice_path, b"day", "slike/alice after day update");
    let alice_days_path = level_path(&contract, "slike", &[b"postAuthor", b"alice", b"day"]);
    assert!(
        read_grove_element(&drive, &alice_days_path, b"d2").is_some(),
        "the new day value tree must exist"
    );

    // Author move: both indexes move; the ranking re-keys exactly once
    // per group and counts stay exact.
    let mut moved_author = docs[2].clone();
    let mut props = moved_author.properties().clone();
    props.insert("postAuthor".to_string(), Value::Text("alice".to_string()));
    moved_author.set_properties(props);
    moved_author.set_revision(Some(2));
    drive
        .update_document_for_contract(
            &moved_author,
            &contract,
            document_type,
            Some(moved_author.owner_id().to_buffer()),
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
            None,
        )
        .expect("expected to move the like between authors");
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(3, "alice")]),
        "the moved like must drain bob and grow alice exactly once"
    );

    // Delete decrements exactly one and removes both indexes' entries.
    drive
        .delete_document_for_contract(
            docs[1].id(),
            &contract,
            "slike",
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("expected to delete the slike document");
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(2, "alice")])
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Behaviour: deeper branch points
// ---------------------------------------------------------------------------

/// A sibling diverging at a count-propagating level (`deep`: chain
/// `[tag, region, postId]` at `tag`, sibling `[tag, region, day]`)
/// wraps its branch inside the propagating level's value trees, and the
/// whole-subtree totals stay exact through the propagating level.
#[test]
fn sibling_branch_at_a_propagating_level_stays_count_exempt() {
    let (drive, contract) = setup_sibling_contract();

    for (i, (tag, region, post, day)) in [
        ("alpha", "east", "p1", "d1"),
        ("alpha", "east", "p2", "d1"),
        ("alpha", "west", "p1", "d2"),
        ("beta", "east", "p1", "d1"),
    ]
    .iter()
    .enumerate()
    {
        let doc = build_row(
            &contract,
            "deep",
            &[
                ("tag", tag),
                ("region", region),
                ("postId", post),
                ("day", day),
            ],
            None,
            i as u64 + 1,
        );
        insert_row(&drive, &contract, "deep", &doc, true)
            .unwrap_or_else(|e| panic!("expected to insert deep row: {e}"));
    }

    let grouping_path = level_path(&contract, "deep", &[b"tag"]);
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(3, "alpha"), (1, "beta")]),
        "totals must stay exact with the sibling populated at the propagating level"
    );

    // The branch hangs inside the propagating level's value tree,
    // wrapped; the chain child there keeps contributing.
    let alpha_east_path = level_path(&contract, "deep", &[b"tag", b"alpha", b"region", b"east"]);
    assert_branch_is_non_counted(&drive, &alpha_east_path, b"day", "deep/alpha/east");
    match read_grove_element(&drive, &alpha_east_path, b"postId") {
        Some(element @ Element::ProvableCountTree(..)) => {
            assert_eq!(element.count_value_or_default(), 2);
        }
        other => panic!("expected the contributing chain terminal, got {other:?}"),
    }

    assert_grovedb_is_consistent(&drive);
}

/// A sibling extending PAST the ranked terminal (`ext`: chain
/// `[postAuthor, postId]` at `postAuthor`, sibling `[postAuthor,
/// postId, day]`) relies on the pre-existing continuation demotion: the
/// extension is wrapped inside the terminal level's count-bearing value
/// trees, and counts stay exact.
#[test]
fn sibling_extension_below_the_ranked_terminal_stays_count_exempt() {
    let (drive, contract) = setup_sibling_contract();

    for (i, (author, post, day)) in [
        ("alice", "p1", "d1"),
        ("alice", "p1", "d2"),
        ("alice", "p2", "d1"),
        ("bob", "p1", "d1"),
    ]
    .iter()
    .enumerate()
    {
        let doc = build_row(
            &contract,
            "ext",
            &[("postAuthor", author), ("postId", post), ("day", day)],
            None,
            i as u64 + 1,
        );
        insert_row(&drive, &contract, "ext", &doc, true)
            .unwrap_or_else(|e| panic!("expected to insert ext row: {e}"));
    }

    let grouping_path = level_path(&contract, "ext", &[b"postAuthor"]);
    assert_eq!(
        count_top_k_named(&drive, &grouping_path, 10, true),
        named(&[(3, "alice"), (1, "bob")]),
        "totals must stay exact with the extension populated below the terminal"
    );

    // The extension is wrapped inside the terminal's CountTree value
    // tree, next to the counted `[0]` member bucket.
    let alice_p1_path = level_path(
        &contract,
        "ext",
        &[b"postAuthor", b"alice", b"postId", b"p1"],
    );
    assert_branch_is_non_counted(&drive, &alice_p1_path, b"day", "ext/alice/p1");
    match read_grove_element(&drive, &alice_p1_path, &[0]) {
        Some(element @ Element::CountTree(..)) => {
            assert_eq!(
                element.count_value_or_default(),
                2,
                "the member bucket must count alice's two p1 likes"
            );
        }
        other => panic!("expected the counted member bucket, got {other:?}"),
    }

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Behaviour: preallocated chain + lazy sibling
// ---------------------------------------------------------------------------

/// The preallocated composition: inserting the referenced `post`
/// preallocates the `plike` chain (the group is rankable at zero), the
/// sibling branch materializes lazily wrapped on the first like, and
/// draining keeps the group at zero with counts exact throughout.
#[test]
fn preallocated_groups_stay_rankable_at_zero_with_the_sibling() {
    let (drive, contract) = setup_sibling_contract();
    let pv = platform_version();

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
        "preallocation must surface the group at zero with the sibling declared"
    );

    // Two likes; the sibling branch materializes lazily, wrapped.
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
            props.insert("day".to_string(), Value::Text("d1".to_string()));
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
        named(&[(2, "alpha")]),
        "two likes count exactly two — the sibling entries are invisible"
    );
    let alpha_path = level_path(&contract, "plike", &[b"hashtag", b"alpha"]);
    assert_branch_is_non_counted(&drive, &alpha_path, b"day", "plike/alpha");

    // Drain: the group stays rankable at zero (preallocated chain kept),
    // while the non-preallocated sibling branch prunes away.
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
        "draining must keep the preallocated group rankable at zero"
    );
    assert!(
        read_grove_element(&drive, &alpha_path, b"day").is_none(),
        "the non-preallocated sibling branch prunes when drained"
    );

    assert_grovedb_is_consistent(&drive);
}
