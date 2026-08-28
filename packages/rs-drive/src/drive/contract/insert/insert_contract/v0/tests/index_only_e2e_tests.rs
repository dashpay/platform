//! End-to-end coverage for **indexOnly document types** (meta schema v3 /
//! PV14): the storage mode where a document is never written to primary
//! storage — the index entries ARE the rows, each terminating in an empty
//! `Item` keyed by the index's `terminal` property instead of a `Reference`
//! keyed by the document id.
//!
//! Everything runs against the `yappr-likes` fixture at
//! `tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json`:
//! a `like` doctype with two properties (`hashtag`, `postId`) and three
//! indexes —
//!
//! | index           | properties            | terminal   | axes                        |
//! |-----------------|-----------------------|------------|-----------------------------|
//! | `byHashtagPost` | `[hashtag, postId]`   | `$ownerId` | countable + range + ranked  |
//! | `byPost`        | `[postId]`            | `$ownerId` | countable + range + ranked  |
//! | `byLiker`       | `[$ownerId]`          | `postId`   | none                        |
//!
//! Three layers are pinned. The *registration shape*: no `[0]` primary-key
//! tree is created for the doctype, while the top-level property-name trees
//! (including `byPost`'s `ProvableCountIndexedTree`) are. The *entry
//! layout*: an insert writes one `[…values, 0, <terminal value>] → Item`
//! per index and nothing else, Items count exactly as References under
//! count and ranked trees, and a duplicate entry is refused. And the
//! *delete symmetry*: delete-by-values removes every entry, prunes drained
//! groups so ranked secondaries drop them, and leaves grovedb's integrity
//! sweep clean.

use crate::drive::Drive;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use grovedb::Element;

const DOCTYPE: &str = "like";

fn platform_version() -> &'static PlatformVersion {
    PlatformVersion::latest()
}

fn setup_likes() -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json",
        false,
        pv,
    )
    .expect("expected to parse the yappr-likes contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the yappr-likes contract");
    (drive, contract)
}

/// `[DataContractDocuments, contract_id, 1, "like"]` — the doctype tree.
fn doctype_path(contract: &DataContract) -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        DOCTYPE.as_bytes().to_vec(),
    ]
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

/// A like on `post` under `hashtag` by `owner`.
fn build_like(
    contract: &DataContract,
    hashtag: &str,
    post: [u8; 32],
    owner: [u8; 32],
    seed: u64,
) -> Document {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");
    let mut doc = document_type
        .random_document(Some(seed), pv)
        .expect("random document");
    let mut props = std::collections::BTreeMap::new();
    props.insert("hashtag".to_string(), Value::Text(hashtag.to_string()));
    props.insert("postId".to_string(), Value::Identifier(post));
    doc.set_properties(props);
    doc.set_owner_id(Identifier::from(owner));
    doc
}

fn insert_like(
    drive: &Drive,
    contract: &DataContract,
    doc: &Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");
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

fn delete_like(
    drive: &Drive,
    contract: &DataContract,
    doc: Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");
    drive.delete_index_only_document_for_contract(
        doc,
        contract,
        document_type,
        BlockInfo::default(),
        apply,
        None,
        pv,
        None,
    )
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

const POST_A: [u8; 32] = [0xA1; 32];
const POST_B: [u8; 32] = [0xB2; 32];
const OWNER_1: [u8; 32] = [0x11; 32];
const OWNER_2: [u8; 32] = [0x22; 32];
const OWNER_3: [u8; 32] = [0x33; 32];

// ---------------------------------------------------------------------------
// Registration shape
// ---------------------------------------------------------------------------

/// An indexOnly doctype registers WITHOUT a `[0]` primary-key tree, while
/// its top-level property-name trees exist: `hashtag` and `$ownerId` as
/// plain trees (compound prefix / plain index), `postId` as the
/// `ProvableCountIndexedTree` the single-property ranked `byPost` demands.
#[test]
fn index_only_doctype_registers_without_primary_key_tree() {
    let (drive, contract) = setup_likes();
    let path = doctype_path(&contract);

    assert!(
        read_grove_element(&drive, &path, &[0]).is_none(),
        "an indexOnly doctype must not create the [0] primary-key tree"
    );

    assert!(
        matches!(
            read_grove_element(&drive, &path, b"hashtag"),
            Some(Element::Tree(..))
        ),
        "the compound index's leading property-name tree must exist as a plain tree"
    );
    assert!(
        matches!(
            read_grove_element(&drive, &path, b"$ownerId"),
            Some(Element::Tree(..))
        ),
        "byLiker's leading property-name tree must exist as a plain tree"
    );
    assert!(
        matches!(
            read_grove_element(&drive, &path, b"postId"),
            Some(Element::ProvableCountIndexedTree(..))
        ),
        "byPost's ranked property-name tree must be a ProvableCountIndexedTree"
    );

    // The sibling non-indexOnly doctype in the same contract keeps its
    // primary-key tree — the skip is per-doctype, not per-contract.
    let mut post_path = doctype_path(&contract);
    *post_path.last_mut().expect("path non-empty") = b"post".to_vec();
    assert!(
        read_grove_element(&drive, &post_path, &[0]).is_some(),
        "a stored doctype in the same contract keeps its primary-key tree"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Entry layout
// ---------------------------------------------------------------------------

/// One like writes exactly its three index entries: empty Items keyed by
/// the terminal value, in the docId slot of the ordinary non-unique layout.
#[test]
fn like_insert_writes_terminal_items_for_every_index() {
    let (drive, contract) = setup_likes();
    let like = build_like(&contract, "dash", POST_A, OWNER_1, 1);
    insert_like(&drive, &contract, &like, true).expect("insert like");

    let base = doctype_path(&contract);

    // byHashtagPost: [.., hashtag, "dash", postId, POST_A, 0] key OWNER_1
    let mut by_hashtag_post = base.clone();
    by_hashtag_post.extend([
        b"hashtag".to_vec(),
        b"dash".to_vec(),
        b"postId".to_vec(),
        POST_A.to_vec(),
        vec![0],
    ]);
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");
    let expected_commitment =
        crate::drive::document::index_only_row_commitment(&like, document_type, platform_version())
            .expect("commitment computes");
    match read_grove_element(&drive, &by_hashtag_post, &OWNER_1) {
        Some(Element::Item(data, _)) => {
            assert_eq!(
                data,
                expected_commitment.to_vec(),
                "the terminal item carries the row commitment binding it to the full tuple"
            )
        }
        other => {
            panic!("expected the row-commitment Item at the byHashtagPost terminal, got {other:?}")
        }
    }

    // byPost: [.., postId, POST_A, 0] key OWNER_1
    let mut by_post = base.clone();
    by_post.extend([b"postId".to_vec(), POST_A.to_vec(), vec![0]]);
    match read_grove_element(&drive, &by_post, &OWNER_1) {
        Some(Element::Item(data, _)) => assert_eq!(
            data,
            expected_commitment.to_vec(),
            "every index's entry carries the SAME row commitment"
        ),
        other => panic!("expected the byPost terminal item, got {other:?}"),
    }

    // byLiker: [.., $ownerId, OWNER_1, 0] key POST_A — the generalized
    // terminal: a refersTo-typed property as member key.
    let mut by_liker = base.clone();
    by_liker.extend([b"$ownerId".to_vec(), OWNER_1.to_vec(), vec![0]]);
    assert!(
        matches!(
            read_grove_element(&drive, &by_liker, &POST_A),
            Some(Element::Item(..))
        ),
        "byLiker terminal item must be keyed by the postId value"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The any-entry-exists rule at the storage layer: re-inserting the same
/// like refuses on the first colliding entry.
#[test]
fn duplicate_like_insert_is_refused() {
    let (drive, contract) = setup_likes();
    let like = build_like(&contract, "dash", POST_A, OWNER_1, 1);
    insert_like(&drive, &contract, &like, true).expect("first insert");

    let error = insert_like(&drive, &contract, &like, true)
        .expect_err("the identical like must be refused");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(crate::error::drive::DriveError::CorruptedContractIndexes(
                _
            ))
        ),
        "expected the CorruptedContractIndexes backstop, got: {error}"
    );

    // A different owner liking the same post is a different member key and
    // must go through.
    let like_2 = build_like(&contract, "dash", POST_A, OWNER_2, 2);
    insert_like(&drive, &contract, &like_2, true).expect("a second owner may like the same post");

    assert_grovedb_is_consistent(&drive);
}

/// Items count exactly as references: group counts and the ranked Count
/// axis order posts by their number of likes, globally and per hashtag.
#[test]
fn likes_count_and_rank_posts() {
    let (drive, contract) = setup_likes();

    // POST_A: 2 likes under #dash; POST_B: 1 like under #dash.
    for (post, owner, seed) in [
        (POST_A, OWNER_1, 1u64),
        (POST_A, OWNER_2, 2),
        (POST_B, OWNER_3, 3),
    ] {
        let like = build_like(&contract, "dash", post, owner, seed);
        insert_like(&drive, &contract, &like, true).expect("insert like");
    }

    let base = doctype_path(&contract);

    // Group counts on the byPost value trees.
    let mut by_post_level = base.clone();
    by_post_level.push(b"postId".to_vec());
    match read_grove_element(&drive, &by_post_level, &POST_A) {
        Some(Element::CountTree(_, count, _)) => assert_eq!(count, 2),
        other => panic!("expected POST_A's group to be a CountTree, got {other:?}"),
    }

    // Global ranking: POST_A (2) before POST_B (1).
    let global = count_top_k(&drive, &by_post_level, 10, true);
    assert_eq!(
        global,
        vec![(2, POST_A.to_vec()), (1, POST_B.to_vec())],
        "byPost's Count axis must rank posts by like count"
    );

    // Per-hashtag ranking through the compound index's terminal level.
    let mut per_hashtag_level = base.clone();
    per_hashtag_level.extend([b"hashtag".to_vec(), b"dash".to_vec(), b"postId".to_vec()]);
    let per_hashtag = count_top_k(&drive, &per_hashtag_level, 10, true);
    assert_eq!(
        per_hashtag,
        vec![(2, POST_A.to_vec()), (1, POST_B.to_vec())],
        "byHashtagPost's Count axis must rank posts within the hashtag"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Delete symmetry
// ---------------------------------------------------------------------------

/// Delete-by-values removes every entry; a drained group prunes away and
/// the ranked secondary drops it.
#[test]
fn like_delete_removes_entries_and_prunes_drained_groups() {
    let (drive, contract) = setup_likes();

    let like_a1 = build_like(&contract, "dash", POST_A, OWNER_1, 1);
    let like_a2 = build_like(&contract, "dash", POST_A, OWNER_2, 2);
    let like_b3 = build_like(&contract, "dash", POST_B, OWNER_3, 3);
    for like in [&like_a1, &like_a2, &like_b3] {
        insert_like(&drive, &contract, like, true).expect("insert like");
    }

    let base = doctype_path(&contract);
    let mut by_post_level = base.clone();
    by_post_level.push(b"postId".to_vec());

    // Unlike POST_B's only like: the group must vanish entirely.
    delete_like(&drive, &contract, like_b3, true).expect("delete like");
    assert!(
        read_grove_element(&drive, &by_post_level, &POST_B).is_none(),
        "POST_B's drained group must prune away"
    );
    assert_eq!(
        count_top_k(&drive, &by_post_level, 10, true),
        vec![(2, POST_A.to_vec())],
        "the ranked secondary must drop the drained group"
    );

    // Unlike one of POST_A's likes: count drops, group stays.
    delete_like(&drive, &contract, like_a2, true).expect("delete like");
    assert_eq!(
        count_top_k(&drive, &by_post_level, 10, true),
        vec![(1, POST_A.to_vec())]
    );

    // The byLiker entries mirror: OWNER_2's entry is gone, OWNER_1's stays.
    let mut owner_2_likes = base.clone();
    owner_2_likes.extend([b"$ownerId".to_vec(), OWNER_2.to_vec(), vec![0]]);
    assert!(read_grove_element(&drive, &owner_2_likes, &POST_A).is_none());
    let mut owner_1_likes = base.clone();
    owner_1_likes.extend([b"$ownerId".to_vec(), OWNER_1.to_vec(), vec![0]]);
    assert!(read_grove_element(&drive, &owner_1_likes, &POST_A).is_some());

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Estimation
// ---------------------------------------------------------------------------

/// The `apply: false` dry-run must work (every traversed layer emits its
/// estimation info) and must not under-estimate the applied storage fee —
/// the invariant consensus fee validation depends on.
#[test]
fn estimated_fees_upper_bound_actual_fees() {
    let (drive, contract) = setup_likes();
    let like = build_like(&contract, "dash", POST_A, OWNER_1, 1);

    let estimated_insert =
        insert_like(&drive, &contract, &like, false).expect("estimated insert must work");
    let actual_insert = insert_like(&drive, &contract, &like, true).expect("actual insert");
    assert!(
        estimated_insert.storage_fee >= actual_insert.storage_fee,
        "estimated insert storage fee {} must upper-bound actual {}",
        estimated_insert.storage_fee,
        actual_insert.storage_fee
    );

    let estimated_delete =
        delete_like(&drive, &contract, like.clone(), false).expect("estimated delete must work");
    let actual_delete = delete_like(&drive, &contract, like, true).expect("actual delete");
    // Deletes refund storage; both processing fees must be present and the
    // estimation path must simply not error or under-run the layer sweep.
    assert!(estimated_delete.processing_fee > 0);
    assert!(actual_delete.processing_fee > 0);

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Queries and proofs (Phase 4): synthesis from proved index positions
// ---------------------------------------------------------------------------

use crate::drive::document::query::QueryDocumentsOutcomeV0Methods;

fn likes_query<'a>(
    contract: &'a dpp::prelude::DataContract,
    clauses: Vec<crate::query::WhereClause>,
    limit: Option<u16>,
) -> crate::query::DriveDocumentQuery<'a> {
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");
    crate::query::DriveDocumentQuery {
        contract,
        document_type,
        internal_clauses: crate::query::InternalClauses::extract_from_clauses(
            clauses,
            platform_version(),
        )
        .expect("clauses extract"),
        offset: None,
        limit,
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
        resolved_time_ranges: vec![],
    }
}

/// The one builder both sides call: the server's no-proof execution and the
/// proof verification must synthesize identical documents, and each
/// synthesized document must carry exactly what its index recovers.
#[test]
fn should_synthesize_query_documents_with_proof_parity() {
    use crate::query::{WhereClause, WhereOperator};
    use dpp::document::DocumentV0Getters;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    for (post, owner, seed) in [
        (POST_A, OWNER_1, 1u64),
        (POST_A, OWNER_2, 2),
        (POST_B, OWNER_3, 3),
    ] {
        let like = build_like(&contract, "dash", post, owner, seed);
        insert_like(&drive, &contract, &like, true).expect("insert like");
    }

    // ── who liked anything under #dash (compound index, full synthesis) ──
    let query = likes_query(
        &contract,
        vec![WhereClause {
            field: "hashtag".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("dash".to_string()),
        }],
        Some(10),
    );

    let outcome = drive
        .query_documents(query.clone(), None, false, None, None)
        .expect("indexOnly query executes");
    let documents = outcome.documents();
    assert_eq!(documents.len(), 3, "three likes under #dash");
    let mut seen: Vec<([u8; 32], [u8; 32])> = documents
        .iter()
        .map(|document| {
            assert_eq!(
                document.properties().get("hashtag"),
                Some(&Value::Text("dash".to_string())),
                "prefix property must be recovered from the path"
            );
            let post: [u8; 32] = document
                .properties()
                .get("postId")
                .expect("postId recovered")
                .to_identifier_bytes()
                .expect("postId is an identifier")
                .try_into()
                .expect("32 bytes");
            (post, document.owner_id().to_buffer())
        })
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec![(POST_A, OWNER_1), (POST_A, OWNER_2), (POST_B, OWNER_3)],
        "synthesis must recover every (post, owner) pair"
    );

    // ── proof parity: the verifier synthesizes the same documents ──
    let (proof, _) = query
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("proof generation");
    let (_root_hash, verified) = query
        .verify_proof(proof.as_slice(), platform_version())
        .expect("proof verification synthesizes");
    let mut verified_ids: Vec<_> = verified.iter().map(|d| d.id()).collect();
    let mut queried_ids: Vec<_> = documents.iter().map(|d| d.id()).collect();
    verified_ids.sort();
    queried_ids.sort();
    assert_eq!(
        verified_ids, queried_ids,
        "proved and unproved synthesis must agree document for document"
    );

    // ── projection through byLiker: what did OWNER_1 like ──
    let my_likes = likes_query(
        &contract,
        vec![WhereClause {
            field: "$ownerId".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(OWNER_1),
        }],
        Some(10),
    );
    let outcome = drive
        .query_documents(my_likes.clone(), None, false, None, None)
        .expect("projection query executes");
    let documents = outcome.documents();
    assert_eq!(documents.len(), 1);
    let projection = &documents[0];
    assert_eq!(projection.owner_id().to_buffer(), OWNER_1);
    assert_eq!(
        projection
            .properties()
            .get("postId")
            .expect("terminal recovered")
            .to_identifier_bytes()
            .expect("identifier"),
        POST_A.to_vec()
    );
    assert!(
        !projection.properties().contains_key("hashtag"),
        "a subset index yields a projection — hashtag is not in byLiker"
    );
    // Projection proofs agree too.
    let (proof, _) = my_likes
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("projection proof generation");
    let (_root, verified) = my_likes
        .verify_proof(proof.as_slice(), platform_version())
        .expect("projection proof verification");
    assert_eq!(verified.len(), 1);
    assert_eq!(verified[0].id(), projection.id());

    assert_grovedb_is_consistent(&drive);
}

/// "Did I like X" as a single query: equality on the terminal property
/// through `byLiker` ([$ownerId] → postId) — the prefix equality fixes the
/// path, the terminal equality selects one member key. Proved and
/// unproved paths agree.
#[test]
fn should_serve_terminal_equality_did_i_like_queries() {
    use crate::query::{WhereClause, WhereOperator};
    use dpp::document::DocumentV0Getters;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    for (post, owner, seed) in [
        (POST_A, OWNER_1, 1u64),
        (POST_A, OWNER_2, 2),
        (POST_B, OWNER_3, 3),
    ] {
        let like = build_like(&contract, "dash", post, owner, seed);
        insert_like(&drive, &contract, &like, true).expect("insert like");
    }

    let did_owner_1_like_post_a = likes_query(
        &contract,
        vec![
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(OWNER_1),
            },
            WhereClause {
                field: "postId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(POST_A),
            },
        ],
        Some(1),
    );
    let outcome = drive
        .query_documents(did_owner_1_like_post_a.clone(), None, false, None, None)
        .expect("terminal-equality query executes");
    let documents = outcome.documents();
    assert_eq!(documents.len(), 1, "OWNER_1 liked POST_A");
    assert_eq!(documents[0].owner_id().to_buffer(), OWNER_1);

    // Proof parity for the existence answer.
    let (proof, _) = did_owner_1_like_post_a
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("terminal-equality proof generation");
    let (_root, verified) = did_owner_1_like_post_a
        .verify_proof(proof.as_slice(), platform_version())
        .expect("terminal-equality proof verification");
    assert_eq!(verified.len(), 1);
    assert_eq!(verified[0].id(), documents[0].id());

    // And the negative answer: OWNER_1 never liked POST_B.
    let did_owner_1_like_post_b = likes_query(
        &contract,
        vec![
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(OWNER_1),
            },
            WhereClause {
                field: "postId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(POST_B),
            },
        ],
        Some(1),
    );
    let outcome = drive
        .query_documents(did_owner_1_like_post_b.clone(), None, false, None, None)
        .expect("negative terminal-equality query executes");
    assert_eq!(outcome.documents().len(), 0, "no such like");
    let (proof, _) = did_owner_1_like_post_b
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("absence proof generation");
    let (_root, verified) = did_owner_1_like_post_b
        .verify_proof(proof.as_slice(), platform_version())
        .expect("absence proof verification");
    assert!(verified.is_empty(), "absence must verify as absence");

    assert_grovedb_is_consistent(&drive);
}

/// Keyset pagination through a range clause on the terminal: with the
/// prefix fully determined, `terminal > <last seen>` ordered by the
/// terminal walks the member keys page by page — the indexOnly
/// replacement for id-shaped startAt cursors. Every page agrees with its
/// proof.
#[test]
fn should_serve_terminal_range_keyset_pagination() {
    use crate::query::{OrderClause, WhereClause, WhereOperator};
    use dpp::document::DocumentV0Getters;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    for (owner, seed) in [(OWNER_1, 1u64), (OWNER_2, 2), (OWNER_3, 3)] {
        let like = build_like(&contract, "dash", POST_A, owner, seed);
        insert_like(&drive, &contract, &like, true).expect("insert like");
    }

    let page_query = |after: Option<[u8; 32]>| {
        let mut clauses = vec![
            WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("dash".to_string()),
            },
            WhereClause {
                field: "postId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(POST_A),
            },
        ];
        if let Some(after) = after {
            clauses.push(WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Identifier(after),
            });
        }
        let mut query = likes_query(&contract, clauses, Some(1));
        query.order_by.insert(
            "$ownerId".to_string(),
            OrderClause {
                field: "$ownerId".to_string(),
                ascending: true,
            },
        );
        query
    };

    // Walk the three likes one page at a time, keyed by the last owner.
    // Bounded at four iterations (three pages + the terminating empty
    // one) so a non-progress regression fails the walked assertion
    // instead of hanging the suite.
    let mut cursor: Option<[u8; 32]> = None;
    let mut walked: Vec<[u8; 32]> = vec![];
    for _ in 0..=3 {
        let query = page_query(cursor);
        let outcome = drive
            .query_documents(query.clone(), None, false, None, None)
            .expect("keyset page executes");
        let documents = outcome.documents();
        if documents.is_empty() {
            break;
        }
        assert_eq!(documents.len(), 1, "limit 1 per page");
        let owner = documents[0].owner_id().to_buffer();

        // The page's proof verifies to the same document.
        let (proof, _) = query
            .clone()
            .execute_with_proof(&drive, None, None, platform_version())
            .expect("page proof generation");
        let (_root, verified) = query
            .verify_proof(proof.as_slice(), platform_version())
            .expect("page proof verification");
        assert_eq!(verified.len(), 1);
        assert_eq!(verified[0].id(), documents[0].id());

        walked.push(owner);
        cursor = Some(owner);
    }
    assert_eq!(
        walked,
        vec![OWNER_1, OWNER_2, OWNER_3],
        "keyset pagination must walk every entry exactly once, in key order"
    );

    assert_grovedb_is_consistent(&drive);
}

/// A terminal clause whose index prefix is not fully determined is
/// refused with the shape requirement — never a wrong-answer scan.
#[test]
fn should_refuse_terminal_clause_without_full_prefix_equalities() {
    use crate::error::query::QuerySyntaxError;
    use crate::query::{WhereClause, WhereOperator};
    use assert_matches::assert_matches;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    // hashtag is a property of byHashtagPost, $ownerId its terminal — but
    // the prefix also needs postId, which carries no equality here.
    let query = likes_query(
        &contract,
        vec![
            WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("dash".to_string()),
            },
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(OWNER_1),
            },
        ],
        Some(1),
    );
    let error = drive
        .query_documents(query, None, false, None, None)
        .expect_err("an underdetermined terminal clause must be refused");
    assert_matches!(
        &error,
        crate::error::Error::Query(QuerySyntaxError::Unsupported(message))
            if message.contains("equality clauses"),
        "unexpected error: {error}"
    );
}

/// A range on the terminal requires an orderBy on it, mirroring the
/// stored-document rule.
#[test]
fn should_require_order_by_for_terminal_range() {
    use crate::error::query::QuerySyntaxError;
    use crate::query::{WhereClause, WhereOperator};
    use assert_matches::assert_matches;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
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
                value: Value::Identifier(POST_A),
            },
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Identifier(OWNER_1),
            },
        ],
        Some(1),
    );
    let error = drive
        .query_documents(query, None, false, None, None)
        .expect_err("a terminal range without orderBy must be refused");
    assert_matches!(
        &error,
        crate::error::Error::Query(QuerySyntaxError::MissingOrderByForRange(_)),
        "unexpected error: {error}"
    );
}

/// By-id fetches have no tree to land on and are refused with guidance.
#[test]
fn should_refuse_by_id_queries() {
    use crate::query::{WhereClause, WhereOperator};
    use assert_matches::assert_matches;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    let query = likes_query(
        &contract,
        vec![WhereClause {
            field: "$id".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier([9u8; 32]),
        }],
        None,
    );
    let error = drive
        .query_documents(query, None, false, None, None)
        .expect_err("by-id queries on indexOnly types must be refused");
    // Pin the typed variant, not just the display text: another error
    // carrying the same wording must not satisfy this test.
    assert_matches!(
        &error,
        crate::error::Error::Query(crate::error::query::QuerySyntaxError::Unsupported(message))
            if message.contains("cannot be fetched by id"),
        "unexpected error: {error}"
    );
}

/// A cursor (`startAt`/`startAfter`) would be resolved through the
/// primary-key tree an indexOnly type does not have — the path
/// constructors must refuse it with the typed `Unsupported` error before
/// any cursor storage lookup can turn it into `StartDocumentNotFound`.
#[test]
fn should_refuse_cursor_queries() {
    use crate::error::query::QuerySyntaxError;
    use crate::query::{WhereClause, WhereOperator};
    use assert_matches::assert_matches;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    let mut query = likes_query(
        &contract,
        vec![WhereClause {
            field: "hashtag".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("dash".to_string()),
        }],
        None,
    );
    query.start_at = Some([7u8; 32]);
    let error = drive
        .query_documents(query, None, false, None, None)
        .expect_err("cursor queries on indexOnly types must be refused");
    assert_matches!(
        &error,
        crate::error::Error::Query(QuerySyntaxError::Unsupported(message))
            if message.contains("paginate with a range clause on the terminal property"),
        "unexpected error: {error}"
    );
}

/// Terminal items carry owner-and-epoch storage flags, so deleting them in
/// a later epoch must refund the freed bytes to the inserting owner,
/// attributed to the insertion epoch — the same refund contract stored
/// documents follow.
#[test]
fn deletion_refunds_flow_from_terminal_item_flags() {
    use dpp::block::epoch::Epoch;
    use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
    use dpp::version::fee::FeeVersion;
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    let (drive, contract) = setup_likes();
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");
    let previous_fee_versions: CachedEpochIndexFeeVersions =
        BTreeMap::from([(0, FeeVersion::first())]);

    let like = build_like(&contract, "dash", POST_A, OWNER_1, 1);

    // Insert at epoch 0 with owner-attributed flags — what the production
    // create-operation converter writes.
    let pv = platform_version();
    drive
        .add_document_for_contract(
            crate::util::object_size_info::DocumentAndContractInfo {
                owned_document_info: crate::util::object_size_info::OwnedDocumentInfo {
                    document_info: crate::util::object_size_info::DocumentInfo::DocumentRefInfo((
                        &like,
                        Some(Cow::Owned(StorageFlags::SingleEpochOwned(0, OWNER_1))),
                    )),
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
        .expect("insert like with owned flags");

    // Delete at a later epoch: the freed bytes must come back to OWNER_1,
    // attributed to insertion epoch 0.
    let later_epoch_block = BlockInfo {
        epoch: Epoch::new(1).expect("epoch 1"),
        ..Default::default()
    };
    let fee_result = drive
        .delete_index_only_document_for_contract(
            like,
            &contract,
            document_type,
            later_epoch_block,
            true,
            None,
            pv,
            Some(&previous_fee_versions),
        )
        .expect("delete like in a later epoch");

    let refund = fee_result
        .fee_refunds
        .calculate_refunds_amount_for_identity(dpp::identifier::Identifier::from(OWNER_1))
        .expect("the refund must be attributed to the inserting owner");
    assert!(
        refund > 0,
        "deleting owner-flagged terminal items must refund storage to the owner"
    );

    assert_grovedb_is_consistent(&drive);
}

/// Row integrity: a values tuple spliced from two of the SAME owner's
/// documents addresses entries that all exist — but they carry different
/// row commitments, so the delete is refused and both documents keep every
/// projection. (The `mark` doctype's two single-property indexes are the
/// splice-prone shape; likes are immune because their compound index binds
/// the values together, but the layout must not depend on that.)
#[test]
fn should_refuse_a_delete_spliced_across_two_rows() {
    use dpp::document::DocumentV0Setters;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    let mark_type = contract
        .document_type_for_name("mark")
        .expect("mark doctype exists");

    let build_mark = |a: &str, b: &str, seed: u64| {
        let mut doc = mark_type
            .random_document(Some(seed), platform_version())
            .expect("random mark");
        let mut props = std::collections::BTreeMap::new();
        props.insert("a".to_string(), Value::Text(a.to_string()));
        props.insert("b".to_string(), Value::Text(b.to_string()));
        doc.set_properties(props);
        doc.set_owner_id(dpp::identifier::Identifier::from(OWNER_1));
        doc
    };
    let insert_mark = |doc: &dpp::document::Document| {
        drive
            .add_document_for_contract(
                crate::util::object_size_info::DocumentAndContractInfo {
                    owned_document_info: crate::util::object_size_info::OwnedDocumentInfo {
                        document_info: crate::util::object_size_info::DocumentInfo::DocumentRefInfo(
                            (doc, None),
                        ),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type: mark_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version(),
                None,
            )
            .expect("insert mark");
    };

    let mark_1 = build_mark("a1", "b1", 1);
    let mark_2 = build_mark("a2", "b2", 2);
    insert_mark(&mark_1);
    insert_mark(&mark_2);

    // The spliced tuple (a1, b2): byA's entry exists (mark_1's), byB's
    // entry exists (mark_2's) — but each carries its own row's commitment.
    let spliced = build_mark("a1", "b2", 3);
    let error = drive
        .delete_index_only_document_for_contract(
            spliced,
            &contract,
            mark_type,
            BlockInfo::default(),
            true,
            None,
            platform_version(),
            None,
        )
        .expect_err("a spliced tuple must be refused");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(
                crate::error::drive::DriveError::DeletingDocumentThatDoesNotExist(_)
            )
        ),
        "expected the row-integrity refusal, got: {error}"
    );

    // Both real rows survive intact and stay individually deletable.
    drive
        .delete_index_only_document_for_contract(
            mark_1,
            &contract,
            mark_type,
            BlockInfo::default(),
            true,
            None,
            platform_version(),
            None,
        )
        .expect("the real row must still delete");
    drive
        .delete_index_only_document_for_contract(
            mark_2,
            &contract,
            mark_type,
            BlockInfo::default(),
            true,
            None,
            platform_version(),
            None,
        )
        .expect("the other real row must still delete");

    assert_grovedb_is_consistent(&drive);
}
