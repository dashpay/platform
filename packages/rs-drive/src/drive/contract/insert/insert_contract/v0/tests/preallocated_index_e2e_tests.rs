//! End-to-end coverage for **preallocated indexOnly indexes** (meta schema
//! v3 / PV14): the `preallocated` index keyword, under which creating a
//! refersTo-referenced document also creates the referring index's dynamic
//! trees — every value tree down to the empty `0` member bucket — so the
//! FIRST entry referencing that document costs the same as every later one,
//! and deleting the last entry keeps the trees instead of pruning them.
//!
//! Runs against the `yappr-likes-preallocated` fixture: byte-identical to
//! the `yappr-likes` contract the sibling [`super::index_only_e2e_tests`]
//! suite uses, except `byHashtagPost` and `byPost` declare
//! `preallocated: true` (`byLiker` cannot — its `$ownerId` prefix is not
//! determined by the referenced post). Three behaviors are pinned:
//!
//! - **Post-insert preallocation**: inserting a `post` creates the like
//!   trees for both preallocated indexes (and only those), the ranked
//!   secondary reports the post as a zero-count group, and querying the
//!   empty index works with proof parity.
//! - **Delete asymmetry**: unliking the last like keeps the preallocated
//!   trees (contrast the sibling suite's
//!   `like_delete_removes_entries_and_prunes_drained_groups`) while the
//!   non-preallocated `byLiker` entry still prunes; re-liking works across
//!   repeated cycles.
//! - **Fees**: the poster pays more than under the plain contract, the
//!   first liker pays less (no tree creation), and the post's `apply:
//!   false` dry-run — which now sweeps the referring type's layers —
//!   upper-bounds the applied storage fee.

use super::chained_query_e2e_tests::remove_post;
use super::index_only_e2e_tests::{
    assert_grovedb_is_consistent, assert_live_root_hash, build_like, count_top_k, delete_like,
    doctype_path, insert_like, likes_query, platform_version, read_grove_element,
};
use crate::drive::Drive;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::moderation::{ContractModerationConfig, ContractModerators};
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::tests::json_document::{json_document_to_contract, json_document_to_json_value};
use serde_json::json;

const OWNER_POSTER: [u8; 32] = [0x0F; 32];
const OWNER_1: [u8; 32] = [0x11; 32];
const OWNER_2: [u8; 32] = [0x22; 32];

fn setup_contract(drive: &Drive, fixture: &str) -> DataContract {
    let pv = platform_version();
    let contract = json_document_to_contract(fixture, false, pv)
        .expect("expected to parse the yappr-likes contract variant");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the contract");
    contract
}

fn setup_preallocated_likes() -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = setup_contract(
        &drive,
        "tests/supporting_files/contract/yappr-likes/yappr-likes-preallocated-contract.json",
    );
    (drive, contract)
}

fn setup_plain_likes() -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = setup_contract(
        &drive,
        "tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json",
    );
    (drive, contract)
}

/// A `post` under `hashtag` by [`OWNER_POSTER`], deterministic per seed.
fn build_post(contract: &DataContract, hashtag: &str, seed: u64) -> Document {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("post")
        .expect("post doctype exists");
    let mut doc = document_type
        .random_document(Some(seed), pv)
        .expect("random post");
    let mut props = std::collections::BTreeMap::new();
    props.insert("hashtag".to_string(), Value::Text(hashtag.to_string()));
    doc.set_properties(props);
    doc.set_owner_id(Identifier::from(OWNER_POSTER));
    doc
}

fn insert_post(
    drive: &Drive,
    contract: &DataContract,
    doc: &Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("post")
        .expect("post doctype exists");
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

// ---------------------------------------------------------------------------
// Post-insert preallocation
// ---------------------------------------------------------------------------

/// Inserting a post creates every dynamic tree of both preallocated like
/// indexes — the hashtag value tree, the deeper property-name and value
/// trees, and the empty `0` member buckets — while the non-preallocated
/// `byLiker` subtree stays untouched. The ranked secondaries see the post
/// as a real group with count 0.
#[test]
fn should_preallocate_referring_like_trees_on_post_insert() {
    let (drive, contract) = setup_preallocated_likes();
    let post = build_post(&contract, "dash", 1);
    let post_id = post.id().to_buffer();
    insert_post(&drive, &contract, &post, true).expect("insert post");

    let base = doctype_path(&contract);

    // byHashtagPost: hashtag → "dash" → postId → <post> → 0, all present.
    let mut level = base.clone();
    level.push(b"hashtag".to_vec());
    assert!(
        read_grove_element(&drive, &level, b"dash").is_some(),
        "the hashtag value tree must be preallocated"
    );
    level.push(b"dash".to_vec());
    assert!(
        read_grove_element(&drive, &level, b"postId").is_some(),
        "the continuation property-name tree must be preallocated"
    );
    level.push(b"postId".to_vec());
    assert!(
        read_grove_element(&drive, &level, &post_id).is_some(),
        "the post's value tree must be preallocated"
    );
    level.push(post_id.to_vec());
    assert!(
        read_grove_element(&drive, &level, &[0]).is_some(),
        "the empty member bucket must be preallocated"
    );

    // byPost: postId → <post> → 0, all present, and the ranked secondary
    // already carries the post as a zero-count group.
    let mut by_post_level = base.clone();
    by_post_level.push(b"postId".to_vec());
    assert!(read_grove_element(&drive, &by_post_level, &post_id).is_some());
    let mut member_bucket = by_post_level.clone();
    member_bucket.push(post_id.to_vec());
    assert!(read_grove_element(&drive, &member_bucket, &[0]).is_some());
    assert_eq!(
        count_top_k(&drive, &by_post_level, 10, true),
        vec![(0, post_id.to_vec())],
        "the preallocated group must rank with count 0"
    );

    // byLiker is not preallocatable: no owner value tree appears.
    let mut by_liker_level = base.clone();
    by_liker_level.push(b"$ownerId".to_vec());
    assert!(
        read_grove_element(&drive, &by_liker_level, &OWNER_POSTER).is_none(),
        "the non-preallocated byLiker index must stay empty"
    );

    assert_grovedb_is_consistent(&drive);
}

/// A referring index may be preallocated through an agreement on the
/// referenced document's `$ownerId`: inserting a post creates the
/// `authorLike` type's `byAuthorPost` trees `authorId -> <poster> -> postId
/// -> <post> -> 0`, keyed from the post's owner exactly as an entry insert
/// would key them, and an agreeing entry then lands in the member bucket.
#[test]
fn should_preallocate_trees_bound_to_the_referenced_owner_id() {
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = setup_contract(
        &drive,
        "tests/supporting_files/contract/yappr-likes/yappr-likes-author-preallocated-contract.json",
    );
    let post = build_post(&contract, "dash", 1);
    let post_id = post.id().to_buffer();
    insert_post(&drive, &contract, &post, true).expect("insert post");

    let mut level = vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        b"authorLike".to_vec(),
    ];
    level.push(b"authorId".to_vec());
    assert!(
        read_grove_element(&drive, &level, &OWNER_POSTER).is_some(),
        "the poster's value tree must be preallocated from the post's $ownerId"
    );
    level.push(OWNER_POSTER.to_vec());
    assert!(
        read_grove_element(&drive, &level, b"postId").is_some(),
        "the continuation property-name tree must be preallocated"
    );
    level.push(b"postId".to_vec());
    assert!(
        read_grove_element(&drive, &level, &post_id).is_some(),
        "the post's value tree must be preallocated"
    );
    level.push(post_id.to_vec());
    assert!(
        read_grove_element(&drive, &level, &[0]).is_some(),
        "the empty member bucket must be preallocated"
    );

    // An entry agreeing with the post lands in the preallocated bucket.
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("authorLike")
        .expect("authorLike doctype exists");
    let mut entry = document_type
        .random_document(Some(2), pv)
        .expect("random author like");
    let mut props = std::collections::BTreeMap::new();
    props.insert("authorId".to_string(), Value::Identifier(OWNER_POSTER));
    props.insert("postId".to_string(), Value::Identifier(post_id));
    entry.set_properties(props);
    entry.set_owner_id(Identifier::from(OWNER_1));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&entry, None)),
                    owner_id: None,
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
        .expect("insert author like");
    level.push(vec![0]);
    assert!(
        read_grove_element(&drive, &level, &OWNER_1).is_some(),
        "the agreeing entry must sit in the preallocated member bucket"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The empty preallocated index is queryable: zero results, working proofs
/// — a present-but-empty member bucket, a state the pruning delete path
/// never used to leave behind.
#[test]
fn should_serve_queries_with_proof_parity_on_empty_preallocated_index() {
    use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;
    use crate::query::{WhereClause, WhereOperator};

    let (drive, contract) = setup_preallocated_likes();
    let post = build_post(&contract, "dash", 1);
    let post_id = post.id().to_buffer();
    insert_post(&drive, &contract, &post, true).expect("insert post");

    let query = likes_query(
        &contract,
        vec![
            WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("dash".to_string()),
            },
            WhereClause {
                field: "postId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(post_id),
            },
        ],
        Some(10),
    );
    let outcome = drive
        .query_documents(query.clone(), None, false, None, None)
        .expect("querying an empty preallocated index executes");
    assert_eq!(
        outcome.documents().len(),
        0,
        "no likes exist yet — the empty bucket must read as zero results"
    );

    let (proof, _) = query
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("proof over the empty preallocated index generates");
    let (_root_hash, verified) = query
        .verify_proof(proof.as_slice(), platform_version())
        .expect("proof over the empty preallocated index verifies");
    assert!(verified.is_empty(), "the proof must verify to zero likes");
}

// ---------------------------------------------------------------------------
// Delete asymmetry
// ---------------------------------------------------------------------------

/// Unliking the last like keeps every preallocated tree — the group stays
/// rankable at count 0 and a re-like needs no tree creation — while the
/// same delete still prunes the non-preallocated byLiker subtree. Repeated
/// like/unlike cycles stay consistent.
#[test]
fn should_keep_preallocated_trees_on_unlike_and_serve_relikes() {
    let (drive, contract) = setup_preallocated_likes();
    let post = build_post(&contract, "dash", 1);
    let post_id: [u8; 32] = post.id().to_buffer();
    insert_post(&drive, &contract, &post, true).expect("insert post");

    let base = doctype_path(&contract);
    let mut by_post_level = base.clone();
    by_post_level.push(b"postId".to_vec());
    let mut member_bucket_path = by_post_level.clone();
    member_bucket_path.push(post_id.to_vec());

    let like = build_like(&contract, "dash", post_id, OWNER_1, 1);
    insert_like(&drive, &contract, &like, true).expect("insert like");
    assert_eq!(
        count_top_k(&drive, &by_post_level, 10, true),
        vec![(1, post_id.to_vec())]
    );

    delete_like(&drive, &contract, like.clone(), true).expect("delete like");

    // The preallocated apparatus survives the drained group…
    assert!(
        read_grove_element(&drive, &by_post_level, &post_id).is_some(),
        "the preallocated value tree must survive the last unlike"
    );
    assert!(
        read_grove_element(&drive, &member_bucket_path, &[0]).is_some(),
        "the preallocated member bucket must survive the last unlike"
    );
    assert_eq!(
        count_top_k(&drive, &by_post_level, 10, true),
        vec![(0, post_id.to_vec())],
        "the drained group must stay ranked at count 0"
    );

    // …while the non-preallocated byLiker subtree pruned as always.
    let mut by_liker_level = base.clone();
    by_liker_level.push(b"$ownerId".to_vec());
    assert!(
        read_grove_element(&drive, &by_liker_level, &OWNER_1).is_none(),
        "the non-preallocated byLiker group must still prune away"
    );

    // Re-like / unlike cycles keep working against the retained trees.
    for _cycle in 0..2 {
        insert_like(&drive, &contract, &like, true).expect("re-like");
        assert_eq!(
            count_top_k(&drive, &by_post_level, 10, true),
            vec![(1, post_id.to_vec())]
        );
        delete_like(&drive, &contract, like.clone(), true).expect("unlike again");
        assert_eq!(
            count_top_k(&drive, &by_post_level, 10, true),
            vec![(0, post_id.to_vec())]
        );
    }

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Fees
// ---------------------------------------------------------------------------

/// The whole point of the flag, in fee form: against the preallocated
/// contract the post costs MORE (it buys the like trees) and the first
/// like costs LESS (it creates nothing) than the byte-identical plain
/// contract charges for the same two inserts.
#[test]
fn should_move_tree_costs_from_first_liker_to_poster() {
    let (preallocated_drive, preallocated_contract) = setup_preallocated_likes();
    let (plain_drive, plain_contract) = setup_plain_likes();

    let post = build_post(&preallocated_contract, "dash", 1);
    let post_id = post.id().to_buffer();
    let post_fee_preallocated =
        insert_post(&preallocated_drive, &preallocated_contract, &post, true).expect("insert post");
    let post_fee_plain =
        insert_post(&plain_drive, &plain_contract, &post, true).expect("insert post");
    assert!(
        post_fee_preallocated.storage_fee > post_fee_plain.storage_fee,
        "the poster must pay for the preallocated trees: {} <= {}",
        post_fee_preallocated.storage_fee,
        post_fee_plain.storage_fee
    );

    let like = build_like(&preallocated_contract, "dash", post_id, OWNER_1, 1);
    let like_fee_preallocated =
        insert_like(&preallocated_drive, &preallocated_contract, &like, true).expect("insert like");
    let like_fee_plain =
        insert_like(&plain_drive, &plain_contract, &like, true).expect("insert like");
    assert!(
        like_fee_preallocated.storage_fee < like_fee_plain.storage_fee,
        "the first like must not pay for tree creation: {} >= {}",
        like_fee_preallocated.storage_fee,
        like_fee_plain.storage_fee
    );

    assert_grovedb_is_consistent(&preallocated_drive);
    assert_grovedb_is_consistent(&plain_drive);
}

/// The post's `apply: false` dry-run sweeps the referring type's layers
/// too now; it must keep upper-bounding the applied storage fee — the
/// invariant consensus fee validation depends on. Same for the (still
/// tree-free) first like against preallocated state.
#[test]
fn should_upper_bound_actual_fees_with_preallocation() {
    let (drive, contract) = setup_preallocated_likes();

    let post = build_post(&contract, "dash", 1);
    let post_id = post.id().to_buffer();
    let estimated_post =
        insert_post(&drive, &contract, &post, false).expect("estimated post insert must work");
    let actual_post = insert_post(&drive, &contract, &post, true).expect("actual post insert");
    assert!(
        estimated_post.storage_fee >= actual_post.storage_fee,
        "estimated post storage fee {} must upper-bound actual {}",
        estimated_post.storage_fee,
        actual_post.storage_fee
    );

    let like = build_like(&contract, "dash", post_id, OWNER_1, 1);
    let estimated_like =
        insert_like(&drive, &contract, &like, false).expect("estimated like insert must work");
    let actual_like = insert_like(&drive, &contract, &like, true).expect("actual like insert");
    assert!(
        estimated_like.storage_fee >= actual_like.storage_fee,
        "estimated like storage fee {} must upper-bound actual {}",
        estimated_like.storage_fee,
        actual_like.storage_fee
    );

    // The delete dry-run exercises the average-case up-tree sweep with the
    // preallocated stop height (member level only, no pruning climb).
    let estimated_delete =
        delete_like(&drive, &contract, like.clone(), false).expect("estimated delete must work");
    let actual_delete = delete_like(&drive, &contract, like, true).expect("actual delete");
    assert!(estimated_delete.processing_fee > 0);
    assert!(actual_delete.processing_fee > 0);
    assert!(
        estimated_delete.processing_fee >= actual_delete.processing_fee,
        "estimated delete processing fee {} must upper-bound actual {}",
        estimated_delete.processing_fee,
        actual_delete.processing_fee,
    );

    assert_grovedb_is_consistent(&drive);
}

/// Preallocation composes across posts sharing a hashtag: the second post
/// finds the shared `dash` value tree already present (if-not-exists) and
/// only pays for its own subtrees; likes on both posts land in their own
/// buckets; unliking one post's like never disturbs the other's trees.
#[test]
fn should_preallocate_shared_hashtag_prefixes_idempotently() {
    let (drive, contract) = setup_preallocated_likes();

    let post_a = build_post(&contract, "dash", 1);
    let post_b = build_post(&contract, "dash", 2);
    let post_a_id = post_a.id().to_buffer();
    let post_b_id = post_b.id().to_buffer();
    insert_post(&drive, &contract, &post_a, true).expect("insert post A");
    insert_post(&drive, &contract, &post_b, true).expect("insert post B");

    let base = doctype_path(&contract);
    let mut posts_under_dash = base.clone();
    posts_under_dash.extend([b"hashtag".to_vec(), b"dash".to_vec(), b"postId".to_vec()]);
    assert!(read_grove_element(&drive, &posts_under_dash, &post_a_id).is_some());
    assert!(read_grove_element(&drive, &posts_under_dash, &post_b_id).is_some());

    let like_a = build_like(&contract, "dash", post_a_id, OWNER_1, 1);
    let like_b = build_like(&contract, "dash", post_b_id, OWNER_2, 2);
    insert_like(&drive, &contract, &like_a, true).expect("like A");
    insert_like(&drive, &contract, &like_b, true).expect("like B");

    let mut by_post_level = base.clone();
    by_post_level.push(b"postId".to_vec());
    assert_eq!(
        count_top_k(&drive, &by_post_level, 10, true).len(),
        2,
        "both posts rank"
    );

    delete_like(&drive, &contract, like_a, true).expect("unlike A");
    let mut post_b_bucket = posts_under_dash.clone();
    post_b_bucket.push(post_b_id.to_vec());
    assert!(
        read_grove_element(&drive, &post_b_bucket, &[0]).is_some(),
        "post B's trees are untouched by post A's unlike"
    );
    let mut post_a_bucket = posts_under_dash.clone();
    post_a_bucket.push(post_a_id.to_vec());
    assert!(
        read_grove_element(&drive, &post_a_bucket, &[0]).is_some(),
        "post A's preallocated trees survive its unlike"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Summable preallocated indexes (tip.byPost: count + sum + both ranked axes)
// ---------------------------------------------------------------------------

/// A summable preallocated index gets its sum-bearing member bucket at
/// post insert (the shared `terminal_member_tree_type` derivation), sums
/// track tips, and the drained group survives the last untip at sum 0 —
/// the sum-surface mirror of the count assertions above.
#[test]
fn should_preallocate_summable_index_and_survive_drain() {
    use super::index_only_e2e_tests::{
        build_tip, delete_tip, insert_tip, sum_top_k, tip_doctype_path,
    };

    let (drive, contract) = setup_preallocated_likes();
    let post = build_post(&contract, "dash", 1);
    let post_id: [u8; 32] = post.id().to_buffer();
    insert_post(&drive, &contract, &post, true).expect("insert post");

    // tip.byPost's trees exist before any tip: postId value tree and the
    // `0` member bucket, and the ranked secondaries carry the post as a
    // zero group on BOTH axes.
    let mut by_post_level = tip_doctype_path(&contract);
    by_post_level.push(b"postId".to_vec());
    assert!(
        read_grove_element(&drive, &by_post_level, &post_id).is_some(),
        "the tip value tree must be preallocated"
    );
    let mut member_bucket = by_post_level.clone();
    member_bucket.push(post_id.to_vec());
    assert!(
        read_grove_element(&drive, &member_bucket, &[0]).is_some(),
        "the sum-bearing member bucket must be preallocated"
    );
    assert_eq!(
        count_top_k(&drive, &by_post_level, 10, true),
        vec![(0, post_id.to_vec())]
    );
    assert_eq!(
        sum_top_k(&drive, &by_post_level, 10, true),
        vec![(0, post_id.to_vec())]
    );

    // Tips contribute; untipping the last one keeps the trees at sum 0.
    let tip = build_tip(&contract, post_id, OWNER_1, 250, 1);
    insert_tip(&drive, &contract, &tip, true).expect("insert tip");
    assert_eq!(
        sum_top_k(&drive, &by_post_level, 10, true),
        vec![(250, post_id.to_vec())]
    );

    delete_tip(&drive, &contract, tip.clone(), true).expect("delete tip");
    assert!(
        read_grove_element(&drive, &member_bucket, &[0]).is_some(),
        "the preallocated sum bucket must survive the last untip"
    );
    assert_eq!(
        sum_top_k(&drive, &by_post_level, 10, true),
        vec![(0, post_id.to_vec())],
        "the drained group must stay ranked at sum 0"
    );

    // Re-tip against the retained trees, and estimation still upper-bounds.
    let estimated = insert_tip(&drive, &contract, &tip, false).expect("estimated re-tip");
    let actual = insert_tip(&drive, &contract, &tip, true).expect("re-tip");
    assert!(
        estimated.storage_fee >= actual.storage_fee,
        "estimated tip storage fee {} must upper-bound actual {}",
        estimated.storage_fee,
        actual.storage_fee
    );
    assert_eq!(
        sum_top_k(&drive, &by_post_level, 10, true),
        vec![(250, post_id.to_vec())]
    );

    assert_grovedb_is_consistent(&drive);
}

/// A preallocated tree's one deletion path is the contract's own — entry
/// deletes retain it by design — so on a non-deletable contract the trees
/// carry NO storage flags even when the post insert supplies them (flags
/// would be unrefundable dead bytes charged to the poster), while a like's
/// member ENTRY inserted with flags carries them as always (entries delete
/// and refund through their element flags).
#[test]
fn should_not_attach_flags_to_preallocated_trees_of_a_permanent_contract() {
    use std::borrow::Cow;

    let (drive, contract) = setup_preallocated_likes();
    let pv = platform_version();
    let post = build_post(&contract, "dash", 1);
    let post_id: [u8; 32] = post.id().to_buffer();

    let post_type = contract
        .document_type_for_name("post")
        .expect("post doctype exists");
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &post,
                        Some(Cow::Owned(StorageFlags::SingleEpochOwned(0, OWNER_POSTER))),
                    )),
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
        .expect("insert post with owned flags");

    let mut post_value_tree_path = doctype_path(&contract);
    post_value_tree_path.extend([b"postId".to_vec(), post_id.to_vec()]);
    let member_bucket = read_grove_element(&drive, &post_value_tree_path, &[0])
        .expect("the member bucket must be preallocated");
    assert!(
        member_bucket.get_flags().is_none(),
        "a preallocated tree of a non-deletable contract must carry no flags, got {:?}",
        member_bucket.get_flags()
    );

    let like = build_like(&contract, "dash", post_id, OWNER_1, 1);
    let like_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &like,
                        Some(Cow::Owned(StorageFlags::SingleEpochOwned(0, OWNER_1))),
                    )),
                    owner_id: None,
                },
                contract: &contract,
                document_type: like_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("insert like with owned flags");

    let mut member_bucket_path = post_value_tree_path.clone();
    member_bucket_path.push(vec![0]);
    let entry = read_grove_element(&drive, &member_bucket_path, &OWNER_1)
        .expect("the like's member entry exists");
    assert!(
        entry.get_flags().is_some(),
        "the member entry must keep its refund-routing flags"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The complementary flags branch: on a DELETABLE contract the
/// preallocated trees must retain the creator's flags, so a whole-contract
/// deletion can route the structural-byte refunds back to the poster —
/// the one deletion path these trees have.
#[test]
fn should_attach_creator_flags_to_preallocated_trees_of_a_deletable_contract() {
    use dpp::data_contract::config::v0::DataContractConfigSettersV0;
    use std::borrow::Cow;

    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let mut contract = json_document_to_contract(
        "tests/supporting_files/contract/yappr-likes/yappr-likes-preallocated-contract.json",
        false,
        pv,
    )
    .expect("expected to parse the preallocated contract");
    contract.config_mut().set_can_be_deleted(true);
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the deletable contract");

    let post = build_post(&contract, "dash", 1);
    let post_id: [u8; 32] = post.id().to_buffer();
    let post_type = contract
        .document_type_for_name("post")
        .expect("post doctype exists");
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &post,
                        Some(Cow::Owned(StorageFlags::SingleEpochOwned(0, OWNER_POSTER))),
                    )),
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
        .expect("insert post with owned flags");

    let mut post_value_tree_path = doctype_path(&contract);
    post_value_tree_path.extend([b"postId".to_vec(), post_id.to_vec()]);
    let member_bucket = read_grove_element(&drive, &post_value_tree_path, &[0])
        .expect("the member bucket must be preallocated");
    let flags = StorageFlags::map_some_element_flags_ref(member_bucket.get_flags())
        .expect("flags must parse")
        .expect("a deletable contract's preallocated trees must carry flags");
    assert_eq!(
        flags,
        StorageFlags::SingleEpochOwned(0, OWNER_POSTER),
        "the poster's epoch-owned flags must ride the preallocated tree"
    );

    assert_grovedb_is_consistent(&drive);
}

/// A bound target property may be optional on the referenced type. When it
/// is absent, NO part of the preallocated path may be emitted — a partial
/// path (the levels before the unresolvable one) would be dead structure no
/// entry can reach. The fixture's `annotation.byNoteTopic` binds
/// `[noteId, topic]` to `note`, whose `topic` is optional, putting the
/// unresolvable key at the SECOND level.
#[test]
fn should_not_emit_partial_paths_when_a_bound_property_is_absent() {
    let (drive, contract) = setup_preallocated_likes();
    let pv = platform_version();
    let note_type = contract
        .document_type_for_name("note")
        .expect("note doctype exists");

    let insert_note = |doc: &Document| {
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((doc, None)),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type: note_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                pv,
                None,
            )
            .expect("insert note")
    };

    // A note WITHOUT the optional bound `topic`: nothing preallocates —
    // not even the first-level noteId value tree.
    let mut bare_note = note_type.random_document(Some(1), pv).expect("random note");
    bare_note.set_properties(std::collections::BTreeMap::new());
    bare_note.set_owner_id(Identifier::from(OWNER_POSTER));
    let bare_note_id: [u8; 32] = bare_note.id().to_buffer();
    insert_note(&bare_note);

    let mut annotation_by_note = vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        b"annotation".to_vec(),
        b"noteId".to_vec(),
    ];
    assert!(
        read_grove_element(&drive, &annotation_by_note, &bare_note_id).is_none(),
        "no level of the path may be emitted when a bound property is absent"
    );

    // A note WITH the topic preallocates the full path down to the bucket.
    let mut note = note_type.random_document(Some(2), pv).expect("random note");
    let mut props = std::collections::BTreeMap::new();
    props.insert("topic".to_string(), Value::Text("rust".to_string()));
    note.set_properties(props);
    note.set_owner_id(Identifier::from(OWNER_POSTER));
    let note_id: [u8; 32] = note.id().to_buffer();
    insert_note(&note);

    assert!(read_grove_element(&drive, &annotation_by_note, &note_id).is_some());
    annotation_by_note.push(note_id.to_vec());
    assert!(read_grove_element(&drive, &annotation_by_note, b"topic").is_some());
    annotation_by_note.push(b"topic".to_vec());
    assert!(read_grove_element(&drive, &annotation_by_note, b"rust").is_some());
    annotation_by_note.push(b"rust".to_vec());
    assert!(
        read_grove_element(&drive, &annotation_by_note, &[0]).is_some(),
        "the full path must reach the empty member bucket"
    );

    assert_grovedb_is_consistent(&drive);
}

/// An UNTAGGED post preallocates nothing for the skipIfAbsent
/// `byHashtagPost` (its hashtag binding has no value to resolve) while
/// `byPost` — bound purely by the post's id — still preallocates, and an
/// untagged like then lands in those trees, skipping the hashtag index
/// entirely. The preallocated + skipIfAbsent composition end-to-end.
#[test]
fn should_preallocate_only_reference_bound_trees_for_an_untagged_post() {
    use super::index_only_e2e_tests::build_untagged_like;

    let (drive, contract) = setup_preallocated_likes();
    let post_type = contract
        .document_type_for_name("post")
        .expect("post doctype exists");
    let mut post = post_type
        .random_document(Some(41), platform_version())
        .expect("random post");
    post.set_properties(std::collections::BTreeMap::new());
    post.set_owner_id(Identifier::from(OWNER_POSTER));
    let post_id = post.id().to_buffer();
    insert_post(&drive, &contract, &post, true).expect("insert untagged post");

    let base = doctype_path(&contract);

    // byHashtagPost: nothing preallocated — the hashtag binding resolved
    // to an absent value and bailed before emitting any operation.
    assert!(
        matches!(
            read_grove_element(&drive, &base, b"hashtag"),
            Some(grovedb::Element::Tree(None, _))
        ),
        "no hashtag trees may be preallocated for an untagged post"
    );

    // byPost: fully preallocated down to the empty member bucket.
    let mut by_post_level = base.clone();
    by_post_level.push(b"postId".to_vec());
    assert!(
        read_grove_element(&drive, &by_post_level, &post_id).is_some(),
        "the id-bound byPost trees must still be preallocated"
    );
    let mut member_bucket = by_post_level.clone();
    member_bucket.push(post_id.to_vec());
    assert!(read_grove_element(&drive, &member_bucket, &[0]).is_some());

    // An untagged like lands in the preallocated byPost trees and skips
    // the hashtag index.
    let like = build_untagged_like(&contract, post_id, OWNER_1, 42);
    insert_like(&drive, &contract, &like, true).expect("insert untagged like");
    let mut entry_path = member_bucket.clone();
    entry_path.push(vec![0]);
    assert!(
        read_grove_element(&drive, &entry_path, &OWNER_1).is_some(),
        "the untagged like's byPost entry must sit in the preallocated bucket"
    );
    assert!(
        matches!(
            read_grove_element(&drive, &base, b"hashtag"),
            Some(grovedb::Element::Tree(None, _))
        ),
        "the like must not have touched the hashtag index"
    );

    // Unlike keeps the preallocated trees (the no-prune contract).
    delete_like(&drive, &contract, like, true).expect("delete untagged like");
    assert!(
        read_grove_element(&drive, &entry_path, &OWNER_1).is_none(),
        "the entry itself must be gone"
    );
    assert!(
        read_grove_element(&drive, &member_bucket, &[0]).is_some(),
        "the preallocated member bucket must survive the unlike"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The preallocated fixture with `post.hashtag` widened to 280 characters and
/// unindexed, while `like.hashtag`, which `byHashtagPost` keys, stays at 63:
/// the shape registration now refuses, as a contract applied before it did.
fn setup_likes_with_a_wide_post_hashtag() -> (Drive, DataContract) {
    let pv = platform_version();
    let drive = setup_drive_with_initial_state_structure(None);
    let mut schema = json_document_to_json_value(
        "tests/supporting_files/contract/yappr-likes/yappr-likes-preallocated-contract.json",
    )
    .expect("read contract fixture");
    let post = &mut schema["documentSchemas"]["post"];
    post["properties"]["hashtag"]["maxLength"] = json!(280);
    post.as_object_mut().expect("post schema").remove("indices");
    let contract = DataContract::try_from_platform_versioned(
        serde_json::from_value(schema).expect("contract serialization format"),
        true,
        &mut vec![],
        pv,
    )
    .expect("parse the wide-hashtag contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the contract");
    (drive, contract)
}

/// A post whose agreement-bound hashtag is wider than any like's can hold is
/// estimated and inserted, and preallocates nothing under `byHashtagPost`,
/// which no like could ever reach, while the id-bound `byPost` still
/// preallocates. A post whose hashtag a like can carry preallocates both.
#[test]
fn should_skip_preallocation_for_a_bound_value_wider_than_the_referring_property() {
    let (drive, contract) = setup_likes_with_a_wide_post_hashtag();
    let base = doctype_path(&contract);

    // 70 characters of four bytes each: within the post's 280 characters,
    // past the like's 63 and past the 255 bytes of any tree key.
    let wide_post = build_post(&contract, &"\u{1F600}".repeat(70), 1);
    let wide_post_id = wide_post.id().to_buffer();
    let estimated = insert_post(&drive, &contract, &wide_post, false)
        .expect("the dry-run of a post with a wide hashtag must work");
    let actual = insert_post(&drive, &contract, &wide_post, true)
        .expect("a post with a wide hashtag must be inserted");
    assert!(
        estimated.storage_fee >= actual.storage_fee,
        "estimated post storage fee {} must upper-bound actual {}",
        estimated.storage_fee,
        actual.storage_fee
    );
    assert!(
        matches!(
            read_grove_element(&drive, &base, b"hashtag"),
            Some(grovedb::Element::Tree(None, _))
        ),
        "no hashtag trees may be preallocated for a hashtag no like can carry"
    );
    let mut by_post_level = base.clone();
    by_post_level.push(b"postId".to_vec());
    assert!(
        read_grove_element(&drive, &by_post_level, &wide_post_id).is_some(),
        "the id-bound byPost trees must still be preallocated"
    );

    let post = build_post(&contract, "dash", 2);
    insert_post(&drive, &contract, &post, false).expect("estimate a post with a short hashtag");
    insert_post(&drive, &contract, &post, true).expect("insert a post with a short hashtag");
    let mut hashtag_level = base.clone();
    hashtag_level.push(b"hashtag".to_vec());
    assert!(
        read_grove_element(&drive, &hashtag_level, b"dash").is_some(),
        "a hashtag a like can carry is preallocated as before"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Through a moderatedDocument reference
// ---------------------------------------------------------------------------

/// The preallocated fixture's `post` and `like` alone, with the post taken
/// down only by the moderators, each removal on the record keeping the fields
/// `kept` lists (none when `None`), and `like.postId` a `moderatedDocument`
/// reference to it; with `preallocated` false, no like index preallocates.
fn setup_likes_of_a_moderated_post(
    kept: Option<&[&str]>,
    preallocated: bool,
) -> (Drive, DataContract) {
    let pv = platform_version();
    let drive = setup_drive_with_initial_state_structure(None);
    let mut schema = json_document_to_json_value(
        "tests/supporting_files/contract/yappr-likes/yappr-likes-preallocated-contract.json",
    )
    .expect("read contract fixture");
    let document_schemas = schema["documentSchemas"]
        .as_object_mut()
        .expect("document schemas");
    document_schemas.retain(|name, _| name == "post" || name == "like");
    document_schemas["post"]["moderatorAbilities"] = match kept {
        Some(kept) => json!({ "delete": true, "deleteKeepsFields": kept }),
        None => json!({ "delete": true }),
    };
    document_schemas["like"]["properties"]["postId"]["refersTo"]["type"] =
        json!("moderatedDocument");
    if !preallocated {
        for index in document_schemas["like"]["indices"]
            .as_array_mut()
            .expect("like indices")
        {
            index
                .as_object_mut()
                .expect("an index")
                .remove("preallocated");
        }
    }
    let config = DataContractConfig::default_for_version(pv)
        .expect("default config available")
        .with_moderation(Some(ContractModerationConfig {
            banlist: false,
            suspensions: false,
            moderators: ContractModerators::ContractOwner,
            warnings: false,
        }));
    schema["config"] = serde_json::to_value(config).expect("config serializes");
    let contract = DataContract::try_from_platform_versioned(
        serde_json::from_value(schema).expect("contract serialization format"),
        false,
        &mut vec![],
        pv,
    )
    .expect("parse the moderated-post contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the contract");
    (drive, contract)
}

/// A moderated post whose removal record keeps its hashtag preallocates the
/// like trees of both `byHashtagPost` and `byPost`, as a permanent post does.
/// A moderator's removal of the post leaves them, like the record; a like of the post is still removed, and the
/// restore puts the post back through the create path onto the trees already
/// there, paying for none of them again.
#[test]
fn should_preallocate_through_a_moderated_reference_whose_record_keeps_the_path() {
    let (drive, contract) = setup_likes_of_a_moderated_post(Some(&["hashtag"]), true);
    let post = build_post(&contract, "dash", 1);
    let post_id = post.id().to_buffer();
    let first_insert = insert_post(&drive, &contract, &post, true).expect("insert post");

    let base = doctype_path(&contract);
    let mut hashtag_post_level = base.clone();
    hashtag_post_level.extend([b"hashtag".to_vec(), b"dash".to_vec(), b"postId".to_vec()]);
    let mut hashtag_member_bucket = hashtag_post_level.clone();
    hashtag_member_bucket.push(post_id.to_vec());
    let mut by_post_level = base.clone();
    by_post_level.push(b"postId".to_vec());
    let mut by_post_member_bucket = by_post_level.clone();
    by_post_member_bucket.push(post_id.to_vec());
    let assert_preallocated = |when: &str| {
        assert!(
            read_grove_element(&drive, &hashtag_member_bucket, &[0]).is_some(),
            "the byHashtagPost member bucket must be there {when}"
        );
        assert!(
            read_grove_element(&drive, &by_post_member_bucket, &[0]).is_some(),
            "the byPost member bucket must be there {when}"
        );
    };
    assert_preallocated("once the post is inserted");

    let like = build_like(&contract, "dash", post_id, OWNER_1, 1);
    insert_like(&drive, &contract, &like, true).expect("insert like");

    remove_post(&drive, &contract, post_id, None);
    assert_preallocated("after the post is removed");
    assert_eq!(
        count_top_k(&drive, &by_post_level, 10, true),
        vec![(1, post_id.to_vec())],
        "the like of the removed post stays counted"
    );

    delete_like(&drive, &contract, like.clone(), true).expect("unlike the removed post");
    assert_preallocated("after the last like of the removed post is removed");

    let restore = insert_post(&drive, &contract, &post, true).expect("restore the post");
    // The restore pays for the post alone, as a post pays where nothing preallocates
    let (plain_drive, plain_contract) = setup_likes_of_a_moderated_post(Some(&["hashtag"]), false);
    let plain_insert =
        insert_post(&plain_drive, &plain_contract, &post, true).expect("insert post");
    assert!(first_insert.storage_fee > plain_insert.storage_fee);
    assert_eq!(
        restore.storage_fee, plain_insert.storage_fee,
        "the restore must not pay for the trees again"
    );
    insert_like(&drive, &contract, &like, true).expect("like the restored post");
    assert_eq!(
        count_top_k(&drive, &by_post_level, 10, true),
        vec![(1, post_id.to_vec())]
    );

    assert_grovedb_is_consistent(&drive);
}

/// Through a moderatedDocument reference, a post preallocates only the trees
/// its removal record would keep every key of: with no field kept,
/// `byHashtagPost`, keyed by the post's hashtag, is left to the first like,
/// while `byPost`, keyed by the post's id alone, is preallocated. (Such a
/// contract only parses without full validation: registration refuses a
/// preallocated index none of whose bindings the record keeps.)
#[test]
fn should_not_preallocate_through_a_moderated_reference_whose_record_drops_a_key() {
    let (drive, contract) = setup_likes_of_a_moderated_post(None, true);
    let post = build_post(&contract, "dash", 1);
    let post_id = post.id().to_buffer();
    let estimated = insert_post(&drive, &contract, &post, false).expect("estimate post");
    let actual = insert_post(&drive, &contract, &post, true).expect("insert post");
    assert!(
        estimated.storage_fee >= actual.storage_fee,
        "estimated post storage fee {} must upper-bound actual {}",
        estimated.storage_fee,
        actual.storage_fee
    );

    let base = doctype_path(&contract);
    assert!(
        matches!(
            read_grove_element(&drive, &base, b"hashtag"),
            Some(grovedb::Element::Tree(None, _))
        ),
        "no hashtag trees may be preallocated for a hashtag the record drops"
    );
    let mut by_post_member_bucket = base.clone();
    by_post_member_bucket.extend([b"postId".to_vec(), post_id.to_vec()]);
    assert!(
        read_grove_element(&drive, &by_post_member_bucket, &[0]).is_some(),
        "the id-bound byPost trees must still be preallocated"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Range reads over empty groups
// ---------------------------------------------------------------------------

/// A range read grouped by the last property of a preallocated index keeps a
/// post nobody tipped, an empty group created with the post, as a count of
/// zero, a sum of zero and an average of `(0, 0)`, unproved and proved alike:
/// the walk's limit counted it, so a page of one returns it and paging one
/// post at a time walks every post.
#[test]
fn should_keep_an_empty_preallocated_group_on_a_page_of_range_reads() {
    use super::index_only_e2e_tests::{build_tip, insert_tip};
    use crate::config::DriveConfig;
    use crate::query::drive_document_average_query::{
        AverageEntry, AverageMode, DocumentAverageRequest, DocumentAverageResponse,
    };
    use crate::query::drive_document_count_query::{
        CountMode, DocumentCountRequest, DocumentCountResponse, SplitCountEntry,
    };
    use crate::query::drive_document_sum_query::index_picker::{
        find_range_summable_index_for_where_clauses,
        find_range_summable_index_with_counts_for_where_clauses,
    };
    use crate::query::drive_document_sum_query::{
        DocumentSumRequest, DocumentSumResponse, DriveDocumentSumQuery, SumEntry, SumMode,
    };
    use crate::query::{DriveDocumentCountQuery, WhereClause, WhereOperator};
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

    let (drive, contract) = setup_preallocated_likes();
    let mut posts: Vec<[u8; 32]> = (1..=3)
        .map(|seed| {
            let post = build_post(&contract, "dash", seed);
            insert_post(&drive, &contract, &post, true).expect("insert post");
            post.id().to_buffer()
        })
        .collect();
    posts.sort();
    let tip = build_tip(&contract, posts[2], OWNER_1, 250, 1);
    insert_tip(&drive, &contract, &tip, true).expect("insert tip");

    let pv = platform_version();
    let drive_config = DriveConfig::default();
    let tip_type = contract
        .document_type_for_name("tip")
        .expect("tip doctype exists");
    let tip_indexes = contract
        .document_types()
        .get("tip")
        .expect("tip doctype exists")
        .indexes();
    let after = |post: [u8; 32]| {
        vec![WhereClause {
            field: "postId".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Identifier(post),
        }]
    };
    // One page of one per kind of read, unproved and proved: (post, count, sum).
    let page = |where_clauses: Vec<WhereClause>, kind: &str| -> Vec<(Vec<u8>, u64, i64)> {
        let count_request = |prove| DocumentCountRequest {
            contract: &contract,
            document_type: tip_type,
            where_clauses: where_clauses.clone(),
            resolved_time_ranges: vec![],
            order_clauses: vec![],
            mode: CountMode::GroupByRange,
            limit: Some(1),
            prove,
            drive_config: &drive_config,
        };
        let sum_request = |prove| DocumentSumRequest {
            contract: &contract,
            document_type: tip_type,
            sum_property: "amount".to_string(),
            where_clauses: where_clauses.clone(),
            resolved_time_ranges: vec![],
            order_clauses: vec![],
            mode: SumMode::GroupByRange,
            limit: Some(1),
            prove,
            drive_config: &drive_config,
        };
        let average_request = |prove| DocumentAverageRequest {
            contract: &contract,
            document_type: tip_type,
            sum_property: "amount".to_string(),
            where_clauses: where_clauses.clone(),
            resolved_time_ranges: vec![],
            order_clauses: vec![],
            mode: AverageMode::GroupByRange,
            limit: Some(1),
            prove,
            drive_config: &drive_config,
        };
        let sum_query = |index| DriveDocumentSumQuery {
            document_type: tip_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "tip".to_string(),
            index,
            where_clauses: where_clauses.clone(),
            sum_property: "amount".to_string(),
        };
        let (unproved, proved) = match kind {
            "count" => {
                let DocumentCountResponse::Entries(unproved) = drive
                    .execute_document_count_request(count_request(false), None, pv)
                    .expect("the count page executes")
                else {
                    panic!("expected count entries");
                };
                let DocumentCountResponse::Proof(proof) = drive
                    .execute_document_count_request(count_request(true), None, pv)
                    .expect("the count page proves")
                else {
                    panic!("expected a count proof");
                };
                let query = DriveDocumentCountQuery {
                    document_type: tip_type,
                    contract_id: contract.id().to_buffer(),
                    document_type_name: "tip".to_string(),
                    index: DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
                        tip_indexes,
                        &where_clauses,
                        &[],
                    )
                    .expect("byPost answers the range count"),
                    where_clauses: where_clauses.clone(),
                };
                let (root_hash, proved) = query
                    .verify_distinct_count_proof(&proof, 1, true, pv)
                    .expect("the count page's proof verifies");
                assert_live_root_hash(&drive, root_hash);
                let rows = |entries: Vec<SplitCountEntry>| {
                    entries
                        .into_iter()
                        .map(|entry| (entry.key, entry.count.expect("a verified count"), 0))
                        .collect::<Vec<_>>()
                };
                (rows(unproved), rows(proved))
            }
            "sum" => {
                let DocumentSumResponse::Entries(unproved) = drive
                    .execute_document_sum_request(sum_request(false), None, pv)
                    .expect("the sum page executes")
                else {
                    panic!("expected sum entries");
                };
                let DocumentSumResponse::Proof(proof) = drive
                    .execute_document_sum_request(sum_request(true), None, pv)
                    .expect("the sum page proves")
                else {
                    panic!("expected a sum proof");
                };
                let index = find_range_summable_index_for_where_clauses(
                    tip_indexes,
                    &where_clauses,
                    "amount",
                    &[],
                )
                .expect("byPost answers the range sum");
                let (root_hash, proved) = sum_query(index)
                    .verify_distinct_sum_proof(&proof, 1, true, pv)
                    .expect("the sum page's proof verifies");
                assert_live_root_hash(&drive, root_hash);
                let rows = |entries: Vec<SumEntry>| {
                    entries
                        .into_iter()
                        .map(|entry| (entry.key, 0, entry.sum.expect("a verified sum")))
                        .collect::<Vec<_>>()
                };
                (rows(unproved), rows(proved))
            }
            _ => {
                let DocumentAverageResponse::Entries(unproved) = drive
                    .execute_document_average_request(average_request(false), None, pv)
                    .expect("the average page executes")
                else {
                    panic!("expected average entries");
                };
                let DocumentAverageResponse::Proof(proof) = drive
                    .execute_document_average_request(average_request(true), None, pv)
                    .expect("the average page proves")
                else {
                    panic!("expected an average proof");
                };
                let index = find_range_summable_index_with_counts_for_where_clauses(
                    tip_indexes,
                    &where_clauses,
                    "amount",
                    &[],
                )
                .expect("byPost answers the range average");
                let (root_hash, proved) = sum_query(index)
                    .verify_distinct_count_and_sum_proof(&proof, 1, true, pv)
                    .expect("the average page's proof verifies");
                assert_live_root_hash(&drive, root_hash);
                let rows = |entries: Vec<AverageEntry>| {
                    entries
                        .into_iter()
                        .map(|entry| {
                            (
                                entry.key,
                                entry.count.expect("a verified count"),
                                entry.sum.expect("a verified sum"),
                            )
                        })
                        .collect::<Vec<_>>()
                };
                (rows(unproved), rows(proved))
            }
        };
        assert_eq!(proved, unproved, "a proved {kind} page");
        unproved
    };

    for (kind, tipped) in [("count", (1, 0)), ("sum", (0, 250)), ("average", (1, 250))] {
        let mut paged = Vec::new();
        let mut last = [0u8; 32];
        loop {
            let entries = page(after(last), kind);
            let Some((post, count, sum)) = entries.first().cloned() else {
                break;
            };
            assert_eq!(entries.len(), 1, "a {kind} page of one");
            last = post.clone().try_into().expect("a post id");
            paged.push((post, count, sum));
        }
        assert_eq!(
            paged,
            vec![
                (posts[0].to_vec(), 0, 0),
                (posts[1].to_vec(), 0, 0),
                (posts[2].to_vec(), tipped.0, tipped.1),
            ],
            "paging one {kind} at a time walks every post, the untipped ones at zero"
        );
    }
}

/// A range count over an index a preallocated sibling passes through keeps
/// the empty groups that sibling's preallocation creates: `byHashtag`, not
/// preallocated, ends at the `hashtag` level `byHashtagPost` preallocates, so
/// a post's hashtag is a group of zero there until liked. A page of one
/// returns it, proved and unproved alike, and paging walks every hashtag.
#[test]
fn should_keep_an_empty_group_a_preallocated_sibling_creates() {
    use crate::config::DriveConfig;
    use crate::query::drive_document_count_query::{
        CountMode, DocumentCountRequest, DocumentCountResponse,
    };
    use crate::query::{DriveDocumentCountQuery, WhereClause, WhereOperator};
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

    let pv = platform_version();
    let drive = setup_drive_with_initial_state_structure(None);
    let mut schema = json_document_to_json_value(
        "tests/supporting_files/contract/yappr-likes/yappr-likes-preallocated-contract.json",
    )
    .expect("read contract fixture");
    let indices = schema["documentSchemas"]["like"]["indices"]
        .as_array_mut()
        .expect("like indices");
    // A ranked `byHashtagPost` refuses a countable index ending at its prefix
    indices[0]
        .as_object_mut()
        .expect("byHashtagPost")
        .remove("rankedCountable");
    indices.push(json!({
        "name": "byHashtag",
        "properties": [{ "hashtag": "asc" }],
        "countable": "countable",
        "rangeCountable": true,
        "skipIfAbsent": true,
    }));
    let contract = DataContract::try_from_platform_versioned(
        serde_json::from_value(schema).expect("contract serialization format"),
        true,
        &mut vec![],
        pv,
    )
    .expect("parse the contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the contract");

    // Posts tagged "a", "b" and "c"; only the "c" post is liked
    let mut last_post = [0u8; 32];
    for (seed, hashtag) in ["a", "b", "c"].into_iter().enumerate() {
        let post = build_post(&contract, hashtag, seed as u64 + 1);
        insert_post(&drive, &contract, &post, true).expect("insert post");
        last_post = post.id().to_buffer();
    }
    let like = build_like(&contract, "c", last_post, OWNER_1, 1);
    insert_like(&drive, &contract, &like, true).expect("insert like");

    let drive_config = DriveConfig::default();
    let like_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let like_indexes = contract
        .document_types()
        .get("like")
        .expect("like doctype exists")
        .indexes();
    let page = |after: &str| -> Vec<(Vec<u8>, u64)> {
        let where_clauses = vec![WhereClause {
            field: "hashtag".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text(after.to_string()),
        }];
        let request = |prove| DocumentCountRequest {
            contract: &contract,
            document_type: like_type,
            where_clauses: where_clauses.clone(),
            resolved_time_ranges: vec![],
            order_clauses: vec![],
            mode: CountMode::GroupByRange,
            limit: Some(1),
            prove,
            drive_config: &drive_config,
        };
        let DocumentCountResponse::Entries(unproved) = drive
            .execute_document_count_request(request(false), None, pv)
            .expect("the page executes")
        else {
            panic!("expected count entries");
        };
        let DocumentCountResponse::Proof(proof) = drive
            .execute_document_count_request(request(true), None, pv)
            .expect("the page proves")
        else {
            panic!("expected a count proof");
        };
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            like_indexes,
            &where_clauses,
            &[],
        )
        .expect("an index answers the range count");
        assert_eq!(index.name, "byHashtag");
        let (root_hash, proved) = DriveDocumentCountQuery {
            document_type: like_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "like".to_string(),
            index,
            where_clauses: where_clauses.clone(),
        }
        .verify_distinct_count_proof(&proof, 1, true, pv)
        .expect("the page's proof verifies");
        assert_live_root_hash(&drive, root_hash);
        assert_eq!(proved, unproved, "proved page after {after:?}");
        unproved
            .into_iter()
            .map(|entry| (entry.key, entry.count.expect("a count")))
            .collect()
    };

    let mut paged = Vec::new();
    let mut after = "0".to_string();
    loop {
        let entries = page(&after);
        let Some((key, count)) = entries.first().cloned() else {
            break;
        };
        assert_eq!(entries.len(), 1, "a page of one");
        after = String::from_utf8(key.clone()).expect("a hashtag");
        paged.push((key, count));
    }
    assert_eq!(
        paged,
        vec![(b"a".to_vec(), 0), (b"b".to_vec(), 0), (b"c".to_vec(), 1)],
        "paging one hashtag at a time walks every one, the unliked ones at zero"
    );
}
