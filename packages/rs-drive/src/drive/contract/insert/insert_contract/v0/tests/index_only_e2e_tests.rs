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
//! plus the `tip` doctype (sum axes), the `beat` doctype (timeRange
//! buckets) — see their sections at the bottom — and the `mark` doctype
//! (two single-property indexes, the splice-prone shape).
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

pub(super) fn platform_version() -> &'static PlatformVersion {
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
pub(super) fn doctype_path(contract: &DataContract) -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        DOCTYPE.as_bytes().to_vec(),
    ]
}

pub(super) fn read_grove_element(drive: &Drive, path: &[Vec<u8>], key: &[u8]) -> Option<Element> {
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

pub(super) fn assert_grovedb_is_consistent(drive: &Drive) {
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
pub(super) fn build_like(
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

pub(super) fn insert_like(
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

pub(super) fn delete_like(
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

pub(super) fn count_top_k(
    drive: &Drive,
    path: &[Vec<u8>],
    k: u16,
    descending: bool,
) -> Vec<(u64, Vec<u8>)> {
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

pub(super) fn likes_query<'a>(
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

/// The raw serialized read — the wire the non-proof GetDocuments endpoint
/// returns — must carry the synthesized documents' serialized bytes when
/// the queried index covers every property, and refuse a subset
/// projection with guidance: the serialized document format has no way to
/// express a missing required property, so partial projections only
/// travel the proved read surface, where the client synthesizes them
/// itself.
#[test]
fn should_serialize_synthesized_documents_on_raw_no_proof_reads() {
    use crate::error::query::QuerySyntaxError;
    use crate::error::Error;
    use crate::query::{WhereClause, WhereOperator};
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::{Document, DocumentV0Getters};
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    let like = build_like(&contract, "dash", POST_A, OWNER_1, 1);
    insert_like(&drive, &contract, &like, true).expect("insert like");

    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");

    // Covering index ([hashtag, postId] → $ownerId): the synthesized
    // document is complete and must round-trip through the wire bytes.
    let covering = likes_query(
        &contract,
        vec![WhereClause {
            field: "hashtag".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("dash".to_string()),
        }],
        Some(10),
    );
    let (serialized, _, _) = covering
        .execute_raw_results_no_proof(&drive, None, None, platform_version())
        .expect("covering raw read executes");
    assert_eq!(serialized.len(), 1, "one like under #dash");
    let document = Document::from_bytes(&serialized[0], document_type, platform_version())
        .expect("wire bytes must round-trip through from_bytes");
    assert_eq!(
        document.properties().get("hashtag"),
        Some(&Value::Text("dash".to_string()))
    );
    assert_eq!(
        document
            .properties()
            .get("postId")
            .expect("postId recovered")
            .to_identifier_bytes()
            .expect("identifier"),
        POST_A.to_vec()
    );
    assert_eq!(document.owner_id().to_buffer(), OWNER_1);

    // Subset index ([$ownerId] → postId): the projection has no hashtag,
    // which is required — it cannot be serialized, so the raw read must
    // refuse with guidance instead of returning bytes no client can parse.
    let subset = likes_query(
        &contract,
        vec![WhereClause {
            field: "$ownerId".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(OWNER_1),
        }],
        Some(10),
    );
    let error = subset
        .execute_raw_results_no_proof(&drive, None, None, platform_version())
        .expect_err("a subset projection cannot travel the serialized wire");
    assert!(
        matches!(
            &error,
            Error::Query(QuerySyntaxError::Unsupported(message))
                if message.contains("does not cover every required property")
        ),
        "expected covering guidance, got: {error}"
    );

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

/// Mixed shape: a range on a PREFIX property (the pivot) with a terminal
/// equality — `hashtag == h AND postId > p AND $ownerId == me`. The path
/// stops at the pivot, the range selects its values, and the terminal
/// equality runs beneath each of them. Proved and unproved paths agree.
#[test]
fn should_serve_prefix_pivot_with_terminal_equality() {
    use crate::query::{OrderClause, WhereClause, WhereOperator};
    use dpp::document::DocumentV0Getters;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    for (post, owner, seed) in [
        (POST_A, OWNER_1, 1u64),
        (POST_B, OWNER_1, 2),
        (POST_B, OWNER_2, 3),
    ] {
        let like = build_like(&contract, "dash", post, owner, seed);
        insert_like(&drive, &contract, &like, true).expect("insert like");
    }

    let mut query = likes_query(
        &contract,
        vec![
            WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("dash".to_string()),
            },
            WhereClause {
                field: "postId".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Identifier(POST_A),
            },
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(OWNER_1),
            },
        ],
        Some(10),
    );
    query.order_by.insert(
        "postId".to_string(),
        OrderClause {
            field: "postId".to_string(),
            ascending: true,
        },
    );

    let outcome = drive
        .query_documents(query.clone(), None, false, None, None)
        .expect("pivot query executes");
    let documents = outcome.documents();
    assert_eq!(documents.len(), 1, "only OWNER_1's like beyond POST_A");
    assert_eq!(documents[0].owner_id().to_buffer(), OWNER_1);
    assert_eq!(
        documents[0]
            .properties()
            .get("postId")
            .expect("postId recovered")
            .to_identifier_bytes()
            .expect("identifier"),
        POST_B.to_vec()
    );

    let (proof, _) = query
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("pivot proof generation");
    let (_root, verified) = query
        .verify_proof(proof.as_slice(), platform_version())
        .expect("pivot proof verification");
    assert_eq!(verified.len(), 1);
    assert_eq!(verified[0].id(), documents[0].id());

    assert_grovedb_is_consistent(&drive);
}

/// A pivot on the FIRST property with the one below it unconstrained:
/// `hashtag >= h AND $ownerId == me` walks every post under each matched
/// hashtag through an insert-all level before the terminal equality.
#[test]
fn should_serve_first_property_pivot_with_unconstrained_below() {
    use crate::query::{OrderClause, WhereClause, WhereOperator};
    use dpp::document::DocumentV0Getters;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    for (post, owner, seed) in [
        (POST_A, OWNER_1, 1u64),
        (POST_B, OWNER_1, 2),
        (POST_B, OWNER_2, 3),
    ] {
        let like = build_like(&contract, "dash", post, owner, seed);
        insert_like(&drive, &contract, &like, true).expect("insert like");
    }

    let mut query = likes_query(
        &contract,
        vec![
            WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::GreaterThanOrEquals,
                value: Value::Text("dash".to_string()),
            },
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(OWNER_1),
            },
        ],
        Some(10),
    );
    query.order_by.insert(
        "hashtag".to_string(),
        OrderClause {
            field: "hashtag".to_string(),
            ascending: true,
        },
    );

    let outcome = drive
        .query_documents(query.clone(), None, false, None, None)
        .expect("first-property pivot query executes");
    let documents = outcome.documents();
    assert_eq!(documents.len(), 2, "both of OWNER_1's likes");
    let mut posts: Vec<Vec<u8>> = documents
        .iter()
        .map(|document| {
            assert_eq!(document.owner_id().to_buffer(), OWNER_1);
            document
                .properties()
                .get("postId")
                .expect("postId recovered")
                .to_identifier_bytes()
                .expect("identifier")
        })
        .collect();
    posts.sort();
    assert_eq!(posts, vec![POST_A.to_vec(), POST_B.to_vec()]);

    let (proof, _) = query
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("first-property pivot proof generation");
    let (_root, verified) = query
        .verify_proof(proof.as_slice(), platform_version())
        .expect("first-property pivot proof verification");
    let mut verified_ids: Vec<_> = verified.iter().map(|d| d.id()).collect();
    let mut queried_ids: Vec<_> = documents.iter().map(|d| d.id()).collect();
    verified_ids.sort();
    queried_ids.sort();
    assert_eq!(verified_ids, queried_ids);

    assert_grovedb_is_consistent(&drive);
}

/// A pivot demands a terminal EQUALITY — two simultaneous non-equality
/// levels have no single pagination order.
#[test]
fn should_refuse_pivot_with_terminal_range() {
    use crate::error::query::QuerySyntaxError;
    use crate::query::{OrderClause, WhereClause, WhereOperator};
    use assert_matches::assert_matches;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    let mut query = likes_query(
        &contract,
        vec![
            WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("dash".to_string()),
            },
            WhereClause {
                field: "postId".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(vec![Value::Identifier(POST_A), Value::Identifier(POST_B)]),
            },
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Identifier(OWNER_1),
            },
        ],
        Some(10),
    );
    for field in ["postId", "$ownerId"] {
        query.order_by.insert(
            field.to_string(),
            OrderClause {
                field: field.to_string(),
                ascending: true,
            },
        );
    }
    let error = drive
        .query_documents(query, None, false, None, None)
        .expect_err("a pivot with a terminal range must be refused");
    assert_matches!(
        &error,
        crate::error::Error::Query(QuerySyntaxError::Unsupported(message))
            if message.contains("EQUALITY clause on the terminal"),
        "unexpected error: {error}"
    );
}

/// An equality BELOW the pivot is not yet supported and must be refused
/// rather than silently scanned. Since protocol version 14's contiguous
/// matching, the candidate is rejected at index-selection time (the
/// equality does not cover the index prefix the range pivots on), so
/// the refusal is the generic no-index error rather than the synthesis
/// route's targeted shape error.
#[test]
fn should_refuse_equality_below_pivot() {
    use crate::error::query::QuerySyntaxError;
    use crate::query::{OrderClause, WhereClause, WhereOperator};
    use assert_matches::assert_matches;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    let mut query = likes_query(
        &contract,
        vec![
            WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Text("c".to_string()),
            },
            WhereClause {
                field: "postId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(POST_A),
            },
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(OWNER_1),
            },
        ],
        Some(10),
    );
    for field in ["hashtag", "postId"] {
        query.order_by.insert(
            field.to_string(),
            OrderClause {
                field: field.to_string(),
                ascending: true,
            },
        );
    }
    let error = drive
        .query_documents(query, None, false, None, None)
        .expect_err("an equality below the pivot must be refused");
    assert_matches!(
        &error,
        crate::error::Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(_)),
        "unexpected error: {error}"
    );
}

/// A pivot range without an orderBy on it is refused, mirroring the
/// stored-document rule.
#[test]
fn should_require_order_by_for_pivot_range() {
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
                operator: WhereOperator::GreaterThan,
                value: Value::Identifier(POST_A),
            },
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Identifier(OWNER_1),
            },
        ],
        Some(10),
    );
    let error = drive
        .query_documents(query, None, false, None, None)
        .expect_err("a pivot range without orderBy must be refused");
    assert_matches!(
        &error,
        crate::error::Error::Query(QuerySyntaxError::MissingOrderByForRange(_)),
        "unexpected error: {error}"
    );
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

/// Clause-field classification is the modeled answer to "where may a
/// clause sit": roles are derived once against the doctype, and a field
/// can hold several at once (`postId` is a prefix property of two yappr
/// indexes AND `byLiker`'s terminal).
#[test]
fn should_classify_clause_fields_against_the_doctype() {
    use crate::query::{InternalClauses, WhereClause, WhereOperator};
    use dpp::platform_value::Value;

    let (_drive, contract) = setup_likes();
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");

    // Dual role: prefix property of byHashtagPost/byPost, terminal of byLiker.
    let post_id_roles = InternalClauses::classify_field(document_type, "postId");
    assert!(post_id_roles.index_property && post_id_roles.terminal);
    assert!(!post_id_roles.primary_key && !post_id_roles.unindexed());

    // Terminal-only ($ownerId is byHashtagPost/byPost's terminal and
    // byLiker's prefix property — also dual on this fixture), and a
    // genuinely unindexed field.
    let owner_roles = InternalClauses::classify_field(document_type, "$ownerId");
    assert!(owner_roles.index_property && owner_roles.terminal);
    let id_roles = InternalClauses::classify_field(document_type, "$id");
    assert!(id_roles.primary_key && !id_roles.index_property && !id_roles.terminal);
    assert!(InternalClauses::classify_field(document_type, "nope").unindexed());

    // classify_fields covers every clause field exactly once.
    let clauses = InternalClauses::extract_from_clauses(
        vec![
            WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("dash".to_string()),
            },
            WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::GreaterThan,
                value: Value::Identifier(OWNER_1),
            },
        ],
        platform_version(),
    )
    .expect("clauses extract");
    let classified = clauses.classify_fields(document_type);
    assert_eq!(classified.len(), 2);
    assert!(classified["hashtag"].index_property && !classified["hashtag"].terminal);
    assert!(classified["$ownerId"].terminal);
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

// ---------------------------------------------------------------------------
// Sum axes: ItemWithSumItem terminal entries (the `tip` doctype)
// ---------------------------------------------------------------------------
//
// The `tip` doctype pins the sum-axis storage mode: `byPost` —
// `[postId] → $ownerId` with the full count AND sum axis set
// (`summable: "amount"` + range + ranked on both) — stores
// `ItemWithSumItem(commitment, amount)` terminals, while
// `byTipperAmount` — `[$ownerId, amount] → postId`, no axes — keeps
// plain `Item(commitment)` terminals on the same rows.

const TIP_DOCTYPE: &str = "tip";

pub(super) fn tip_doctype_path(contract: &DataContract) -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        TIP_DOCTYPE.as_bytes().to_vec(),
    ]
}

/// A tip of `amount` on `post` by `owner`.
pub(super) fn build_tip(
    contract: &DataContract,
    post: [u8; 32],
    owner: [u8; 32],
    amount: u64,
    seed: u64,
) -> Document {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(TIP_DOCTYPE)
        .expect("tip doctype exists");
    let mut doc = document_type
        .random_document(Some(seed), pv)
        .expect("random document");
    let mut props = std::collections::BTreeMap::new();
    props.insert("postId".to_string(), Value::Identifier(post));
    props.insert("amount".to_string(), Value::U64(amount));
    doc.set_properties(props);
    doc.set_owner_id(Identifier::from(owner));
    doc
}

pub(super) fn insert_tip(
    drive: &Drive,
    contract: &DataContract,
    doc: &Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(TIP_DOCTYPE)
        .expect("tip doctype exists");
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

pub(super) fn delete_tip(
    drive: &Drive,
    contract: &DataContract,
    doc: Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(TIP_DOCTYPE)
        .expect("tip doctype exists");
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

pub(super) fn sum_top_k(
    drive: &Drive,
    path: &[Vec<u8>],
    k: u16,
    descending: bool,
) -> Vec<(i64, Vec<u8>)> {
    let path_query = grovedb::PathQuery::new_axis(
        path.to_vec(),
        grovedb_query::AxisQuery::top_k(grovedb_query::IndexAxis::Sum, k, 0, descending)
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
        .expect("the keys-only sum-axis read must succeed")
    {
        grovedb::PathQueryRun::AxisKeys {
            keys: grovedb::AxisKeys::Sum(pairs),
            ..
        } => pairs,
        other => panic!("expected sum keys, got {other:?}"),
    }
}

/// The unproved point-lookup sum of `amount` over tips whose `postId`
/// equals `post` — the query surface a "total tipped to this post"
/// feature reads.
fn tips_total_for_post(drive: &Drive, contract: &DataContract, post: [u8; 32]) -> i64 {
    use crate::query::drive_document_sum_query::{
        DocumentSumRequest, DocumentSumResponse, SumMode,
    };
    let document_type = contract
        .document_type_for_name(TIP_DOCTYPE)
        .expect("tip doctype exists");
    let drive_config = crate::config::DriveConfig::default();
    let request = DocumentSumRequest {
        contract,
        document_type,
        sum_property: "amount".to_string(),
        where_clauses: vec![crate::query::WhereClause {
            field: "postId".to_string(),
            operator: crate::query::WhereOperator::Equal,
            value: Value::Identifier(post),
        }],
        resolved_time_ranges: vec![],
        order_clauses: vec![],
        mode: SumMode::Aggregate,
        limit: None,
        prove: false,
        drive_config: &drive_config,
    };
    match drive
        .execute_document_sum_request(request, None, platform_version())
        .expect("the point-lookup sum must execute")
    {
        DocumentSumResponse::Aggregate(sum) => sum,
        other => panic!("expected an aggregate sum, got {other:?}"),
    }
}

/// A summable index's terminal entries are `ItemWithSumItem(commitment,
/// amount)` — the same commitment every plain entry of the row carries,
/// plus the summed property's value — while a non-summable index on the
/// same doctype keeps plain `Item` terminals.
#[test]
fn tip_insert_writes_sum_bearing_terminal_items() {
    let (drive, contract) = setup_likes();
    let tip = build_tip(&contract, POST_A, OWNER_1, 100, 1);
    insert_tip(&drive, &contract, &tip, true).expect("insert tip");

    let document_type = contract
        .document_type_for_name(TIP_DOCTYPE)
        .expect("tip doctype exists");
    let expected_commitment =
        crate::drive::document::index_only_row_commitment(&tip, document_type, platform_version())
            .expect("commitment computes");

    // byPost (summable): [.., postId, POST_A, 0] key OWNER_1 →
    // ItemWithSumItem(commitment, 100).
    let mut by_post = tip_doctype_path(&contract);
    by_post.extend([b"postId".to_vec(), POST_A.to_vec(), vec![0]]);
    match read_grove_element(&drive, &by_post, &OWNER_1) {
        Some(Element::ItemWithSumItem(data, sum_value, _)) => {
            assert_eq!(
                data,
                expected_commitment.to_vec(),
                "the sum-bearing terminal still carries the row commitment"
            );
            assert_eq!(sum_value, 100, "the sum item carries the tip's amount");
        }
        other => panic!("expected an ItemWithSumItem at the byPost terminal, got {other:?}"),
    }

    // byTipperAmount (no axes): [.., $ownerId, OWNER_1, amount, <100>, 0]
    // key POST_A → plain Item(commitment). The amount path segment uses the
    // property's key encoding.
    use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
    let encoded_amount = document_type
        .serialize_value_for_key("amount", &Value::U64(100), platform_version())
        .expect("amount encodes");
    let mut by_tipper_amount = tip_doctype_path(&contract);
    by_tipper_amount.extend([
        b"$ownerId".to_vec(),
        OWNER_1.to_vec(),
        b"amount".to_vec(),
        encoded_amount,
        vec![0],
    ]);
    match read_grove_element(&drive, &by_tipper_amount, &POST_A) {
        Some(Element::Item(data, _)) => assert_eq!(
            data,
            expected_commitment.to_vec(),
            "the non-summable index keeps a plain commitment Item on the same row"
        ),
        other => panic!("expected a plain Item at the byTipperAmount terminal, got {other:?}"),
    }

    assert_grovedb_is_consistent(&drive);
}

/// Sum aggregation across entries: per-post totals via the point-lookup
/// sum query (unproved and proved agree), and the ranked Sum axis orders
/// posts by total tipped.
#[test]
fn tips_sum_and_rank_posts() {
    use crate::query::drive_document_sum_query::index_picker::find_summable_index_for_where_clauses;
    use crate::query::drive_document_sum_query::{
        DocumentSumRequest, DocumentSumResponse, DriveDocumentSumQuery, SumMode,
    };
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

    let (drive, contract) = setup_likes();
    // POST_A: 100 + 250 = 350; POST_B: 40.
    insert_tip(
        &drive,
        &contract,
        &build_tip(&contract, POST_A, OWNER_1, 100, 1),
        true,
    )
    .expect("tip 1");
    insert_tip(
        &drive,
        &contract,
        &build_tip(&contract, POST_A, OWNER_2, 250, 2),
        true,
    )
    .expect("tip 2");
    insert_tip(
        &drive,
        &contract,
        &build_tip(&contract, POST_B, OWNER_1, 40, 3),
        true,
    )
    .expect("tip 3");

    assert_eq!(tips_total_for_post(&drive, &contract, POST_A), 350);
    assert_eq!(tips_total_for_post(&drive, &contract, POST_B), 40);

    // Proved parity: the proof of the same point lookup verifies and
    // yields the same total.
    let document_type = contract
        .document_type_for_name(TIP_DOCTYPE)
        .expect("tip doctype exists");
    let post_a_clause = crate::query::WhereClause {
        field: "postId".to_string(),
        operator: crate::query::WhereOperator::Equal,
        value: Value::Identifier(POST_A),
    };
    let drive_config = crate::config::DriveConfig::default();
    let request = DocumentSumRequest {
        contract: &contract,
        document_type,
        sum_property: "amount".to_string(),
        where_clauses: vec![post_a_clause.clone()],
        resolved_time_ranges: vec![],
        order_clauses: vec![],
        mode: SumMode::Aggregate,
        limit: None,
        prove: true,
        drive_config: &drive_config,
    };
    let proof_bytes = match drive
        .execute_document_sum_request(request, None, platform_version())
        .expect("the proved point-lookup sum must execute")
    {
        DocumentSumResponse::Proof(bytes) => bytes,
        other => panic!("expected proof bytes, got {other:?}"),
    };
    let index = find_summable_index_for_where_clauses(
        document_type.indexes(),
        std::slice::from_ref(&post_a_clause),
        "amount",
        &[],
    )
    .expect("byPost covers `postId == POST_A`");
    let sum_query = DriveDocumentSumQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: TIP_DOCTYPE.to_string(),
        index,
        where_clauses: vec![post_a_clause],
        sum_property: "amount".to_string(),
    };
    let verifier_path_query = sum_query
        .point_lookup_sum_path_query(platform_version())
        .expect("verifier path query builds");
    let (_root_hash, proved) = grovedb::GroveDb::verify_query(
        &proof_bytes,
        &verifier_path_query,
        &platform_version().drive.grove_version,
    )
    .expect("the sum proof must verify");
    let proved_total: i64 = proved
        .into_iter()
        .filter_map(|(_path, _key, element)| element)
        .map(|element| element.sum_value_or_default())
        .sum();
    assert_eq!(proved_total, 350, "proved and unproved totals must agree");

    // Ranked Sum axis on byPost's property-name tree: POST_A (350) above
    // POST_B (40).
    let mut post_tree = tip_doctype_path(&contract);
    post_tree.push(b"postId".to_vec());
    let top = sum_top_k(&drive, &post_tree, 10, true);
    assert_eq!(
        top,
        vec![(350, POST_A.to_vec()), (40, POST_B.to_vec())],
        "the Sum axis ranks posts by total tipped"
    );

    assert_grovedb_is_consistent(&drive);
}

/// Delete-by-values on a summable type: grovedb reads the amount off the
/// stored element and subtracts it from every ancestor sum — totals and
/// rankings follow, and drained groups prune.
#[test]
fn tip_delete_subtracts_sums_and_prunes_drained_groups() {
    let (drive, contract) = setup_likes();
    let tip_1 = build_tip(&contract, POST_A, OWNER_1, 100, 1);
    let tip_2 = build_tip(&contract, POST_A, OWNER_2, 250, 2);
    let tip_3 = build_tip(&contract, POST_B, OWNER_1, 40, 3);
    insert_tip(&drive, &contract, &tip_1, true).expect("tip 1");
    insert_tip(&drive, &contract, &tip_2, true).expect("tip 2");
    insert_tip(&drive, &contract, &tip_3, true).expect("tip 3");

    delete_tip(&drive, &contract, tip_2, true).expect("delete tip 2");
    assert_eq!(
        tips_total_for_post(&drive, &contract, POST_A),
        100,
        "the deleted tip's amount is subtracted from the post's total"
    );

    let mut post_tree = tip_doctype_path(&contract);
    post_tree.push(b"postId".to_vec());
    assert_eq!(
        sum_top_k(&drive, &post_tree, 10, true),
        vec![(100, POST_A.to_vec()), (40, POST_B.to_vec())],
        "the ranking re-orders after the subtraction"
    );

    // Draining POST_A prunes its group from the ranked tree entirely.
    delete_tip(&drive, &contract, tip_1, true).expect("delete tip 1");
    assert_eq!(
        sum_top_k(&drive, &post_tree, 10, true),
        vec![(40, POST_B.to_vec())],
        "a drained post drops out of the ranking"
    );
    assert!(
        read_grove_element(&drive, &post_tree, &POST_A).is_none(),
        "the drained post's value tree is pruned"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The commitment probe covers the amount: a delete whose values carry a
/// falsified amount addresses the same byPost entry (the amount is not in
/// that index's path) but fails the row-commitment comparison.
#[test]
fn tip_delete_with_falsified_amount_is_refused() {
    let (drive, contract) = setup_likes();
    let tip = build_tip(&contract, POST_A, OWNER_1, 100, 1);
    insert_tip(&drive, &contract, &tip, true).expect("insert tip");

    let falsified = build_tip(&contract, POST_A, OWNER_1, 999, 2);
    let error = delete_tip(&drive, &contract, falsified, true)
        .expect_err("a falsified amount must be refused");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(
                crate::error::drive::DriveError::DeletingDocumentThatDoesNotExist(_)
            )
        ),
        "expected the row-integrity refusal, got: {error}"
    );

    // The honest tuple still deletes, and the sum drains to zero.
    delete_tip(&drive, &contract, tip, true).expect("the real tuple must still delete");
    assert_eq!(tips_total_for_post(&drive, &contract, POST_A), 0);

    assert_grovedb_is_consistent(&drive);
}

/// The estimation twin covers the sum-bearing element and tree variants:
/// dry-run fees upper-bound applied fees for a summable indexOnly type.
#[test]
fn tip_estimated_fees_upper_bound_actual_fees() {
    let (drive, contract) = setup_likes();
    let tip = build_tip(&contract, POST_A, OWNER_1, 100, 1);

    let estimated_insert =
        insert_tip(&drive, &contract, &tip, false).expect("estimated insert must work");
    let actual_insert = insert_tip(&drive, &contract, &tip, true).expect("actual insert");
    assert!(
        estimated_insert.storage_fee >= actual_insert.storage_fee,
        "estimated insert storage fee {} must upper-bound actual {}",
        estimated_insert.storage_fee,
        actual_insert.storage_fee
    );

    let estimated_delete =
        delete_tip(&drive, &contract, tip.clone(), false).expect("estimated delete must work");
    let actual_delete = delete_tip(&drive, &contract, tip, true).expect("actual delete");
    assert!(estimated_delete.processing_fee > 0);
    assert!(actual_delete.processing_fee > 0);

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// timeRange buckets: bucketed entries (the `beat` doctype)
// ---------------------------------------------------------------------------
//
// The `beat` doctype pins the bucketed storage mode: `byHourHashtag` —
// `[$createdAt, hashtag] → $ownerId` with `timeRange { on: $createdAt,
// range: 3600, step: 900 }` (overlap factor 4) plus the count axes — writes
// one commitment entry per containing bucket under the grid-qualified
// level, while `byHashtag` — `[hashtag] → $ownerId`, no transform — is the
// $createdAt-free proof index. Bucketed entries serve the count aggregate
// surfaces only; document synthesis over them is refused.

const BEAT_DOCTYPE: &str = "beat";

/// A real timestamp and the four bucket starts (ms) containing it on the
/// 3600s/900s grid — pinned by arithmetic, independent of
/// `containing_buckets`.
const BEAT_T_MS: u64 = 1_700_000_000_000;
const BEAT_BUCKET_STARTS_MS: [u64; 4] = [
    1_699_996_500_000,
    1_699_997_400_000,
    1_699_998_300_000,
    1_699_999_200_000,
];
/// `TimeRangeTransform::storage_key("$createdAt")` for the fixture grid.
const BEAT_GRID_LEVEL: &[u8] = b"$createdAt#3600#900";

fn beat_doctype_path(contract: &DataContract) -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        BEAT_DOCTYPE.as_bytes().to_vec(),
    ]
}

fn encode_timestamp(ms: u64) -> Vec<u8> {
    dpp::data_contract::document_type::DocumentPropertyType::encode_date_timestamp(ms)
}

/// A beat under `hashtag` by `owner` created at `created_at_ms`.
fn build_beat(
    contract: &DataContract,
    hashtag: &str,
    owner: [u8; 32],
    created_at_ms: u64,
    seed: u64,
) -> Document {
    use dpp::document::DocumentV0Setters;
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(BEAT_DOCTYPE)
        .expect("beat doctype exists");
    let mut doc = document_type
        .random_document(Some(seed), pv)
        .expect("random document");
    let mut props = std::collections::BTreeMap::new();
    props.insert("hashtag".to_string(), Value::Text(hashtag.to_string()));
    doc.set_properties(props);
    doc.set_owner_id(Identifier::from(owner));
    doc.set_created_at(Some(created_at_ms));
    doc
}

fn insert_beat(
    drive: &Drive,
    contract: &DataContract,
    doc: &Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(BEAT_DOCTYPE)
        .expect("beat doctype exists");
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

fn delete_beat(
    drive: &Drive,
    contract: &DataContract,
    doc: Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(BEAT_DOCTYPE)
        .expect("beat doctype exists");
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

/// One commitment entry per containing bucket, under the grid-qualified
/// level key, at bucket starts pinned by arithmetic — plus the plain
/// `byHashtag` entry on the same row.
#[test]
fn beat_insert_writes_one_entry_per_containing_bucket() {
    let (drive, contract) = setup_likes();
    let beat = build_beat(&contract, "dash", OWNER_1, BEAT_T_MS, 1);
    insert_beat(&drive, &contract, &beat, true).expect("insert beat");

    let document_type = contract
        .document_type_for_name(BEAT_DOCTYPE)
        .expect("beat doctype exists");
    let expected_commitment =
        crate::drive::document::index_only_row_commitment(&beat, document_type, platform_version())
            .expect("commitment computes");

    for bucket_start in BEAT_BUCKET_STARTS_MS {
        let mut bucket_entry = beat_doctype_path(&contract);
        bucket_entry.extend([
            BEAT_GRID_LEVEL.to_vec(),
            encode_timestamp(bucket_start),
            b"hashtag".to_vec(),
            b"dash".to_vec(),
            vec![0],
        ]);
        match read_grove_element(&drive, &bucket_entry, &OWNER_1) {
            Some(Element::Item(data, _)) => assert_eq!(
                data,
                expected_commitment.to_vec(),
                "bucket {bucket_start}: every bucket's entry carries the row commitment"
            ),
            other => panic!("expected the entry in bucket {bucket_start}, got {other:?}"),
        }
    }

    // No entry exists one step below the earliest containing bucket.
    let mut outside = beat_doctype_path(&contract);
    outside.extend([
        BEAT_GRID_LEVEL.to_vec(),
        encode_timestamp(BEAT_BUCKET_STARTS_MS[0] - 900_000),
        b"hashtag".to_vec(),
        b"dash".to_vec(),
        vec![0],
    ]);
    assert!(
        read_grove_element(&drive, &outside, &OWNER_1).is_none(),
        "no entry outside the containing buckets"
    );

    // The plain proof index holds the same row's commitment.
    let mut by_hashtag = beat_doctype_path(&contract);
    by_hashtag.extend([b"hashtag".to_vec(), b"dash".to_vec(), vec![0]]);
    match read_grove_element(&drive, &by_hashtag, &OWNER_1) {
        Some(Element::Item(data, _)) => assert_eq!(data, expected_commitment.to_vec()),
        other => panic!("expected the byHashtag entry, got {other:?}"),
    }

    assert_grovedb_is_consistent(&drive);
}

/// The trending surface: a resolved `IN_TIME_RANGE` count over one bucket
/// counts the bucket's entries per hashtag, unproved and proved agreeing.
#[test]
fn beat_bucket_counts_serve_trending() {
    use crate::query::drive_document_count_query::{
        CountMode, DocumentCountRequest, DocumentCountResponse, DriveDocumentCountQuery,
    };
    use crate::query::ResolvedTimeRange;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::TimeRangeTransform;

    let (drive, contract) = setup_likes();
    // Two dash beats and one btc beat, all at BEAT_T_MS.
    for (owner, hashtag, seed) in [
        (OWNER_1, "dash", 1u64),
        (OWNER_2, "dash", 2),
        (OWNER_3, "btc", 3),
    ] {
        let beat = build_beat(&contract, hashtag, owner, BEAT_T_MS, seed);
        insert_beat(&drive, &contract, &beat, true).expect("insert beat");
    }

    let document_type = contract
        .document_type_for_name(BEAT_DOCTYPE)
        .expect("beat doctype exists");
    let transform = TimeRangeTransform {
        source: "$createdAt".to_string(),
        range_seconds: 3600,
        step_seconds: 900,
        phase_seconds: 0,
    };
    let resolved = vec![ResolvedTimeRange {
        transform: transform.clone(),
    }];
    let bucket = BEAT_BUCKET_STARTS_MS[3];
    let count_in_bucket = |hashtag: &str, prove: bool| {
        let drive_config = crate::config::DriveConfig::default();
        let request = DocumentCountRequest {
            contract: &contract,
            document_type,
            where_clauses: vec![
                crate::query::WhereClause {
                    field: "$createdAt".to_string(),
                    operator: crate::query::WhereOperator::Equal,
                    value: Value::U64(bucket),
                },
                crate::query::WhereClause {
                    field: "hashtag".to_string(),
                    operator: crate::query::WhereOperator::Equal,
                    value: Value::Text(hashtag.to_string()),
                },
            ],
            resolved_time_ranges: resolved.clone(),
            order_clauses: Vec::new(),
            mode: CountMode::Aggregate,
            limit: None,
            prove,
            drive_config: &drive_config,
        };
        drive
            .execute_document_count_request(request, None, platform_version())
            .expect("the bucketed count must execute")
    };

    match count_in_bucket("dash", false) {
        DocumentCountResponse::Aggregate(count) => {
            assert_eq!(count, 2, "two dash beats in the bucket")
        }
        other => panic!("expected an aggregate count, got {other:?}"),
    }
    match count_in_bucket("btc", false) {
        DocumentCountResponse::Aggregate(count) => {
            assert_eq!(count, 1, "one btc beat in the bucket")
        }
        other => panic!("expected an aggregate count, got {other:?}"),
    }

    // Proved parity for the dash count.
    let proof_bytes = match count_in_bucket("dash", true) {
        DocumentCountResponse::Proof(bytes) => bytes,
        other => panic!("expected proof bytes, got {other:?}"),
    };
    let where_clauses = vec![
        crate::query::WhereClause {
            field: "$createdAt".to_string(),
            operator: crate::query::WhereOperator::Equal,
            value: Value::U64(bucket),
        },
        crate::query::WhereClause {
            field: "hashtag".to_string(),
            operator: crate::query::WhereOperator::Equal,
            value: Value::Text("dash".to_string()),
        },
    ];
    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &resolved,
    )
    .expect("byHourHashtag covers the resolved bucket count");
    let count_query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: BEAT_DOCTYPE.to_string(),
        index,
        where_clauses,
    };
    let verifier_path_query = count_query
        .point_lookup_count_path_query(platform_version())
        .expect("verifier path query builds");
    let (_root_hash, proved) = grovedb::GroveDb::verify_query(
        &proof_bytes,
        &verifier_path_query,
        &platform_version().drive.grove_version,
    )
    .expect("the count proof must verify");
    let proved_count: u64 = proved
        .into_iter()
        .filter_map(|(_path, _key, element)| element)
        .map(|element| element.count_value_or_default())
        .sum();
    assert_eq!(proved_count, 2, "proved and unproved counts must agree");

    assert_grovedb_is_consistent(&drive);
}

/// Document synthesis over the bucketed index is refused with guidance —
/// the bucket level carries bucket-start granularity, not the document's
/// timestamp.
#[test]
fn beat_synthesis_over_bucketed_index_is_refused() {
    use crate::query::{DriveDocumentQuery, InternalClauses, ResolvedTimeRange};
    use dpp::data_contract::document_type::TimeRangeTransform;

    let (drive, contract) = setup_likes();
    let beat = build_beat(&contract, "dash", OWNER_1, BEAT_T_MS, 1);
    insert_beat(&drive, &contract, &beat, true).expect("insert beat");

    let document_type = contract
        .document_type_for_name(BEAT_DOCTYPE)
        .expect("beat doctype exists");
    let query = DriveDocumentQuery {
        contract: &contract,
        document_type,
        internal_clauses: InternalClauses::extract_from_clauses(
            vec![
                crate::query::WhereClause {
                    field: "$createdAt".to_string(),
                    operator: crate::query::WhereOperator::Equal,
                    value: Value::U64(BEAT_BUCKET_STARTS_MS[3]),
                },
                crate::query::WhereClause {
                    field: "hashtag".to_string(),
                    operator: crate::query::WhereOperator::Equal,
                    value: Value::Text("dash".to_string()),
                },
            ],
            platform_version(),
        )
        .expect("clauses extract"),
        offset: None,
        limit: Some(10),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
        resolved_time_ranges: vec![ResolvedTimeRange {
            transform: TimeRangeTransform {
                source: "$createdAt".to_string(),
                range_seconds: 3600,
                step_seconds: 900,
                phase_seconds: 0,
            },
        }],
    };
    let error = drive
        .query_documents(query, None, false, None, None)
        .expect_err("document synthesis over a bucketed indexOnly index must be refused");
    assert!(
        error
            .to_string()
            .contains("IN_TIME_RANGE document queries are not supported on an indexOnly type"),
        "expected the bucketed-synthesis refusal, got: {error}"
    );
}

/// Delete-by-values removes every bucket entry (the carried `$createdAt`
/// reproduces the exact bucket set), drained groups prune, and a falsified
/// timestamp addressing the same buckets dies on the commitment probe.
#[test]
fn beat_delete_removes_every_bucket_entry() {
    let (drive, contract) = setup_likes();
    let beat = build_beat(&contract, "dash", OWNER_1, BEAT_T_MS, 1);
    insert_beat(&drive, &contract, &beat, true).expect("insert beat");

    // A tuple whose timestamp differs by 1ms lands in the SAME buckets —
    // its entries exist — but the commitment binds the exact timestamp.
    let falsified = build_beat(&contract, "dash", OWNER_1, BEAT_T_MS + 1, 2);
    let error = delete_beat(&drive, &contract, falsified, true)
        .expect_err("a falsified timestamp must be refused");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(
                crate::error::drive::DriveError::DeletingDocumentThatDoesNotExist(_)
            )
        ),
        "expected the row-integrity refusal, got: {error}"
    );

    delete_beat(&drive, &contract, beat, true).expect("delete beat");

    let mut grid_tree = beat_doctype_path(&contract);
    grid_tree.push(BEAT_GRID_LEVEL.to_vec());
    for bucket_start in BEAT_BUCKET_STARTS_MS {
        assert!(
            read_grove_element(&drive, &grid_tree, &encode_timestamp(bucket_start)).is_none(),
            "bucket {bucket_start} must be pruned after the last entry leaves"
        );
    }
    let mut hashtag_tree = beat_doctype_path(&contract);
    hashtag_tree.push(b"hashtag".to_vec());
    assert!(
        read_grove_element(&drive, &hashtag_tree, b"dash").is_none(),
        "the plain index's group prunes too"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The estimation twin fans out identically: dry-run fees upper-bound
/// applied fees across the bucket fan-out.
#[test]
fn beat_estimated_fees_upper_bound_actual_fees() {
    let (drive, contract) = setup_likes();
    let beat = build_beat(&contract, "dash", OWNER_1, BEAT_T_MS, 1);

    let estimated_insert =
        insert_beat(&drive, &contract, &beat, false).expect("estimated insert must work");
    let actual_insert = insert_beat(&drive, &contract, &beat, true).expect("actual insert");
    assert!(
        estimated_insert.storage_fee >= actual_insert.storage_fee,
        "estimated insert storage fee {} must upper-bound actual {}",
        estimated_insert.storage_fee,
        actual_insert.storage_fee
    );

    let estimated_delete =
        delete_beat(&drive, &contract, beat.clone(), false).expect("estimated delete must work");
    let actual_delete = delete_beat(&drive, &contract, beat, true).expect("actual delete");
    assert!(estimated_delete.processing_fee > 0);
    assert!(actual_delete.processing_fee > 0);

    assert_grovedb_is_consistent(&drive);
}

/// Duplicate detection probes the bucketed index too: re-creating the same
/// beat is refused, and a beat one step-width later shares three of four
/// buckets yet is a distinct row.
#[test]
fn beat_duplicate_and_overlapping_rows() {
    let (drive, contract) = setup_likes();
    let beat = build_beat(&contract, "dash", OWNER_1, BEAT_T_MS, 1);
    insert_beat(&drive, &contract, &beat, true).expect("first insert");

    let error =
        insert_beat(&drive, &contract, &beat, true).expect_err("the identical beat must refuse");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(crate::error::drive::DriveError::CorruptedContractIndexes(
                _
            ))
        ),
        "expected the CorruptedContractIndexes backstop, got: {error}"
    );

    // Same owner and hashtag one step later is a different value tuple,
    // but it still collides: its bucketed entries share three of four
    // bucket positions with the first beat (same member key), and the
    // plain byHashtag entry collides outright — ANY colliding entry
    // refuses the create, which is the structural-uniqueness rule
    // (here: one beat per (hashtag, owner), and per shared bucket).
    let overlapping = build_beat(&contract, "dash", OWNER_1, BEAT_T_MS + 900_000, 2);
    let error = insert_beat(&drive, &contract, &overlapping, true)
        .expect_err("an overlapping beat by the same owner must refuse");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(crate::error::drive::DriveError::CorruptedContractIndexes(
                _
            ))
        ),
        "expected the duplicate refusal on the shared buckets, got: {error}"
    );

    // A different owner in the same buckets is a different member key.
    let other_owner = build_beat(&contract, "dash", OWNER_2, BEAT_T_MS, 3);
    insert_beat(&drive, &contract, &other_owner, true)
        .expect("another owner may beat in the same buckets");

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// skipIfAbsent: conditional index participation
// ---------------------------------------------------------------------------
//
// The `mark` doctype's `byB` index is `skipIfAbsent` with the optional `b`
// as its trigger (and count axes, so sparse counting is observable); the
// `pin` doctype adds the compound skip shape `byTagXY [tag, x, y]` with an
// optional BYTE-ARRAY trigger admitting the empty value — the one input
// class where "skip" (absent) and the null-layout empty key (present,
// empty encoding) sit one byte apart.

fn mark_type_ref(contract: &DataContract) -> dpp::data_contract::document_type::DocumentTypeRef {
    contract
        .document_type_for_name("mark")
        .expect("mark doctype exists")
}

/// A property-name tree with no children — the registration-time state a
/// skip index's tree stays in while every row omits the trigger. Whatever
/// countability variant registration chose, "empty" means no root key
/// (and a zero count where the variant carries one).
fn assert_empty_index_tree(element: Option<Element>, context: &str) {
    match element {
        Some(Element::Tree(root, _)) => {
            assert!(root.is_none(), "{context}: expected an empty tree")
        }
        Some(Element::CountTree(root, count, _))
        | Some(Element::ProvableCountTree(root, count, _)) => {
            assert!(
                root.is_none() && count == 0,
                "{context}: expected an empty count tree, got root {root:?} count {count}"
            )
        }
        Some(Element::ProvableCountIndexedTree(root, secondary, count, _)) => assert!(
            root.is_none() && secondary.is_none() && count == 0,
            "{context}: expected an empty indexed count tree"
        ),
        other => panic!("{context}: expected a property-name tree, got {other:?}"),
    }
}

/// A mark with `a` always present and `b` present only when given — the
/// trigger-absent form is the whole point of this section.
fn build_mark(
    contract: &DataContract,
    a: &str,
    b: Option<&str>,
    owner: [u8; 32],
    seed: u64,
) -> Document {
    let mut doc = mark_type_ref(contract)
        .random_document(Some(seed), platform_version())
        .expect("random mark");
    let mut props = std::collections::BTreeMap::new();
    props.insert("a".to_string(), Value::Text(a.to_string()));
    if let Some(b) = b {
        props.insert("b".to_string(), Value::Text(b.to_string()));
    }
    doc.set_properties(props);
    doc.set_owner_id(Identifier::from(owner));
    doc
}

fn insert_mark(
    drive: &Drive,
    contract: &DataContract,
    doc: &Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    drive.add_document_for_contract(
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((doc, None)),
                owner_id: None,
            },
            contract,
            document_type: mark_type_ref(contract),
        },
        false,
        BlockInfo::default(),
        apply,
        None,
        platform_version(),
        None,
    )
}

fn delete_mark(
    drive: &Drive,
    contract: &DataContract,
    doc: Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    drive.delete_index_only_document_for_contract(
        doc,
        contract,
        mark_type_ref(contract),
        BlockInfo::default(),
        apply,
        None,
        platform_version(),
        None,
    )
}

/// `[.., "mark"]` / `[.., "pin"]` — sibling doctype trees.
fn sibling_doctype_path(contract: &DataContract, doctype: &str) -> Vec<Vec<u8>> {
    let mut path = doctype_path(contract);
    *path.last_mut().expect("path non-empty") = doctype.as_bytes().to_vec();
    path
}

/// A trigger-absent create writes entries ONLY in the non-skip indexes: no
/// value tree, no member bucket, not a byte under the skip index's
/// property-name tree — and its delete removes exactly what was written,
/// leaving other rows' skip entries untouched.
#[test]
fn skip_index_absent_trigger_writes_nothing_and_deletes_cleanly() {
    let (drive, contract) = setup_likes();
    let mark_path = sibling_doctype_path(&contract, "mark");

    let no_b = build_mark(&contract, "a1", None, OWNER_1, 1);
    insert_mark(&drive, &contract, &no_b, true).expect("insert trigger-absent mark");

    // byA carries the row: [.., "a", "a1", 0] key OWNER_1, payload = the
    // commitment over the PRESENT set (no `b` contribution).
    let expected_commitment = crate::drive::document::index_only_row_commitment(
        &no_b,
        mark_type_ref(&contract),
        platform_version(),
    )
    .expect("commitment computes");
    let mut by_a = mark_path.clone();
    by_a.extend([b"a".to_vec(), b"a1".to_vec(), vec![0]]);
    match read_grove_element(&drive, &by_a, &OWNER_1) {
        Some(Element::Item(data, _)) => assert_eq!(
            data,
            expected_commitment.to_vec(),
            "the non-skip entry carries the present-set commitment"
        ),
        other => panic!("expected the byA terminal item, got {other:?}"),
    }

    // byB's property-name tree exists (registration creates it) but holds
    // NOTHING: not a value tree, not a member bucket, not a byte.
    assert_empty_index_tree(
        read_grove_element(&drive, &mark_path, b"b"),
        "after a trigger-absent create",
    );

    // A trigger-present sibling row populates byB, independently.
    let with_b = build_mark(&contract, "a2", Some("b2"), OWNER_2, 2);
    insert_mark(&drive, &contract, &with_b, true).expect("insert trigger-present mark");
    let mut by_b_b2 = mark_path.clone();
    by_b_b2.extend([b"b".to_vec(), b"b2".to_vec(), vec![0]]);
    assert!(
        matches!(
            read_grove_element(&drive, &by_b_b2, &OWNER_2),
            Some(Element::Item(..))
        ),
        "a trigger-present row writes its skip-index entry normally"
    );

    // Deleting the trigger-absent row removes byA's entry and leaves the
    // sibling's skip entry alone.
    delete_mark(&drive, &contract, no_b, true).expect("delete trigger-absent mark");
    assert!(
        read_grove_element(&drive, &by_a, &OWNER_1).is_none(),
        "the non-skip entry must be gone"
    );
    assert!(
        read_grove_element(&drive, &by_b_b2, &OWNER_2).is_some(),
        "the sibling's skip entry must survive"
    );

    // Deleting the sibling drains byB back to the empty tree.
    delete_mark(&drive, &contract, with_b, true).expect("delete trigger-present mark");
    assert_empty_index_tree(
        read_grove_element(&drive, &mark_path, b"b"),
        "after the last trigger-present row deletes",
    );

    assert_grovedb_is_consistent(&drive);
}

/// The commitment pins the exact present-set: a delete whose absence
/// pattern differs from the create — dropping the trigger, or inventing
/// one — fails the commitment probe in BOTH directions, so a skip index
/// can neither be force-pruned nor left with an orphan entry.
#[test]
fn skip_index_absence_pattern_binds_the_commitment() {
    let (drive, contract) = setup_likes();

    // Row 1 was created WITH the trigger; a delete carrying the same
    // values minus `b` addresses byA's real entry with a different
    // commitment (all of row 1's entries would survive an accepted probe
    // in byB only because probes there are vacuous for the forged shape).
    let with_b = build_mark(&contract, "a1", Some("b1"), OWNER_1, 1);
    insert_mark(&drive, &contract, &with_b, true).expect("insert trigger-present mark");
    let forged_without_b = build_mark(&contract, "a1", None, OWNER_1, 2);
    let error = delete_mark(&drive, &contract, forged_without_b, true)
        .expect_err("dropping the trigger from the carried values must be refused");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(
                crate::error::drive::DriveError::DeletingDocumentThatDoesNotExist(_)
            )
        ),
        "expected the row-integrity refusal, got: {error}"
    );

    // Row 2 was created WITHOUT the trigger; a delete carrying an invented
    // `b` — even one whose byB entry exists, courtesy of row 1 — fails on
    // byA's commitment.
    let no_b = build_mark(&contract, "a2", None, OWNER_1, 3);
    insert_mark(&drive, &contract, &no_b, true).expect("insert trigger-absent mark");
    let forged_with_b = build_mark(&contract, "a2", Some("b1"), OWNER_1, 4);
    let error = delete_mark(&drive, &contract, forged_with_b, true)
        .expect_err("inventing a trigger in the carried values must be refused");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(
                crate::error::drive::DriveError::DeletingDocumentThatDoesNotExist(_)
            )
        ),
        "expected the row-integrity refusal, got: {error}"
    );

    // Both real rows remain deletable with their true absence patterns.
    delete_mark(&drive, &contract, with_b, true).expect("the trigger-present row still deletes");
    delete_mark(&drive, &contract, no_b, true).expect("the trigger-absent row still deletes");

    assert_grovedb_is_consistent(&drive);
}

/// Structural uniqueness spans the skip boundary: a trigger-absent and a
/// trigger-present document of the same owner colliding on a NON-skip
/// index cannot coexist, in either creation order.
#[test]
fn skip_and_non_skip_rows_still_collide_on_shared_entries() {
    let (drive, contract) = setup_likes();

    let with_b = build_mark(&contract, "a1", Some("b1"), OWNER_1, 1);
    insert_mark(&drive, &contract, &with_b, true).expect("insert trigger-present mark");
    let no_b = build_mark(&contract, "a1", None, OWNER_1, 2);
    let error = insert_mark(&drive, &contract, &no_b, true)
        .expect_err("the byA entry collides regardless of the trigger");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(crate::error::drive::DriveError::CorruptedContractIndexes(
                _
            ))
        ),
        "expected the duplicate-entry backstop, got: {error}"
    );

    // Reverse order on a fresh value.
    let no_b_2 = build_mark(&contract, "a2", None, OWNER_1, 3);
    insert_mark(&drive, &contract, &no_b_2, true).expect("insert trigger-absent mark");
    let with_b_2 = build_mark(&contract, "a2", Some("b2"), OWNER_1, 4);
    let error = insert_mark(&drive, &contract, &with_b_2, true)
        .expect_err("the byA entry collides in the reverse order too");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(crate::error::drive::DriveError::CorruptedContractIndexes(
                _
            ))
        ),
        "expected the duplicate-entry backstop, got: {error}"
    );

    assert_grovedb_is_consistent(&drive);
}

/// A skip index's count axes see exactly the trigger-present rows — the
/// sparse-projection semantics the query gate makes opt-in.
#[test]
fn skip_index_counts_reflect_trigger_present_rows_only() {
    let (drive, contract) = setup_likes();
    let mark_path = sibling_doctype_path(&contract, "mark");

    for (a, b, owner, seed) in [
        ("a1", Some("shared"), OWNER_1, 1u64),
        ("a2", Some("shared"), OWNER_2, 2),
        ("a3", None, OWNER_3, 3),
    ] {
        let mark = build_mark(&contract, a, b, owner, seed);
        insert_mark(&drive, &contract, &mark, true).expect("insert mark");
    }

    let mut by_b_level = mark_path.clone();
    by_b_level.push(b"b".to_vec());
    match read_grove_element(&drive, &by_b_level, b"shared") {
        Some(Element::CountTree(_, count, _)) => {
            assert_eq!(count, 2, "the skip index counts trigger-present rows only")
        }
        other => panic!("expected the shared group's CountTree, got {other:?}"),
    }

    assert_grovedb_is_consistent(&drive);
}

/// Fee shape: skipping an index is cheaper than writing it, and the
/// dry-run stays a valid upper bound for both the trigger-absent create
/// (value-driven — it skips exactly when apply skips) and its delete
/// (shape-driven — it deliberately over-estimates the skipped branch).
#[test]
fn skip_estimated_fees_upper_bound_actual_fees() {
    let (drive, contract) = setup_likes();

    let no_b = build_mark(&contract, "a1", None, OWNER_1, 1);
    let estimated_insert =
        insert_mark(&drive, &contract, &no_b, false).expect("estimated insert must work");
    let actual_insert = insert_mark(&drive, &contract, &no_b, true).expect("actual insert");
    assert!(
        estimated_insert.storage_fee >= actual_insert.storage_fee,
        "estimated insert storage fee {} must upper-bound actual {}",
        estimated_insert.storage_fee,
        actual_insert.storage_fee
    );

    // The payoff, measured: the same row with the trigger present pays for
    // the extra index.
    let with_b = build_mark(&contract, "a2", Some("b2"), OWNER_2, 2);
    let actual_insert_with_b =
        insert_mark(&drive, &contract, &with_b, true).expect("actual insert with trigger");
    assert!(
        actual_insert_with_b.storage_fee > actual_insert.storage_fee,
        "a trigger-present row ({}) must cost more than a trigger-absent one ({})",
        actual_insert_with_b.storage_fee,
        actual_insert.storage_fee
    );

    let estimated_delete =
        delete_mark(&drive, &contract, no_b.clone(), false).expect("estimated delete must work");
    let actual_delete = delete_mark(&drive, &contract, no_b, true).expect("actual delete");
    assert!(estimated_delete.processing_fee > 0);
    assert!(actual_delete.processing_fee > 0);

    assert_grovedb_is_consistent(&drive);
}

// ── pin: the compound skip shape and the empty-encoding trigger ─────────

fn pin_type_ref(contract: &DataContract) -> dpp::data_contract::document_type::DocumentTypeRef {
    contract
        .document_type_for_name("pin")
        .expect("pin doctype exists")
}

/// A pin with required `x`/`y` and the byte-array trigger `tag` present
/// only when given — `Some(&[])` is the PRESENT-but-empty form, whose
/// tree-key encoding is empty bytes (the null-layout lane), one byte away
/// from absence.
fn build_pin(
    contract: &DataContract,
    tag: Option<&[u8]>,
    x: &str,
    y: &str,
    owner: [u8; 32],
    seed: u64,
) -> Document {
    let mut doc = pin_type_ref(contract)
        .random_document(Some(seed), platform_version())
        .expect("random pin");
    let mut props = std::collections::BTreeMap::new();
    if let Some(tag) = tag {
        props.insert("tag".to_string(), Value::Bytes(tag.to_vec()));
    }
    props.insert("x".to_string(), Value::Text(x.to_string()));
    props.insert("y".to_string(), Value::Text(y.to_string()));
    doc.set_properties(props);
    doc.set_owner_id(Identifier::from(owner));
    doc
}

fn insert_pin(
    drive: &Drive,
    contract: &DataContract,
    doc: &Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    drive.add_document_for_contract(
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((doc, None)),
                owner_id: None,
            },
            contract,
            document_type: pin_type_ref(contract),
        },
        false,
        BlockInfo::default(),
        apply,
        None,
        platform_version(),
        None,
    )
}

fn delete_pin(
    drive: &Drive,
    contract: &DataContract,
    doc: Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    drive.delete_index_only_document_for_contract(
        doc,
        contract,
        pin_type_ref(contract),
        BlockInfo::default(),
        apply,
        None,
        platform_version(),
        None,
    )
}

fn pin_query<'a>(
    contract: &'a DataContract,
    clauses: Vec<crate::query::WhereClause>,
    limit: Option<u16>,
) -> crate::query::DriveDocumentQuery<'a> {
    crate::query::DriveDocumentQuery {
        contract,
        document_type: pin_type_ref(contract),
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

/// The admissibility gate: a query binding everything BUT the trigger must
/// not route to the skip index — the matcher alone would admit it within
/// the difference budget and silently omit every trigger-absent row. The
/// same query WITH the trigger bound routes there and sees exactly the
/// trigger-present rows, with proof parity.
#[test]
fn trigger_unbound_queries_do_not_route_to_a_skip_index() {
    use crate::error::query::QuerySyntaxError;
    use crate::query::{WhereClause, WhereOperator};
    use dpp::document::DocumentV0Getters;

    let (drive, contract) = setup_likes();

    let tagged = build_pin(&contract, Some(&[1u8]), "x1", "y1", OWNER_1, 1);
    insert_pin(&drive, &contract, &tagged, true).expect("insert tagged pin");
    let untagged = build_pin(&contract, None, "x1", "y1", OWNER_2, 2);
    insert_pin(&drive, &contract, &untagged, true).expect("insert untagged pin");

    // {x, y} bound, trigger unbound: byX lacks y, byY lacks x, and the
    // skip index byTagXY must be inadmissible — so no index serves it.
    let unbound = pin_query(
        &contract,
        vec![
            WhereClause {
                field: "x".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("x1".to_string()),
            },
            WhereClause {
                field: "y".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("y1".to_string()),
            },
        ],
        Some(10),
    );
    let error = drive
        .query_documents(unbound, None, false, None, None)
        .expect_err("a trigger-unbound query must not route to the skip index");
    assert!(
        matches!(
            error,
            crate::error::Error::Query(QuerySyntaxError::WhereClauseOnNonIndexedProperty(_))
        ),
        "expected the no-index refusal, got: {error}"
    );

    // Binding the trigger opts into the sparse semantics: only the tagged
    // row comes back.
    let bound = pin_query(
        &contract,
        vec![
            WhereClause {
                field: "tag".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Bytes(vec![1u8]),
            },
            WhereClause {
                field: "x".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("x1".to_string()),
            },
            WhereClause {
                field: "y".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text("y1".to_string()),
            },
        ],
        Some(10),
    );
    let outcome = drive
        .query_documents(bound.clone(), None, false, None, None)
        .expect("the trigger-bound query executes");
    let documents = outcome.documents();
    assert_eq!(
        documents.len(),
        1,
        "only the tagged row lives in the skip index"
    );
    assert_eq!(documents[0].owner_id().to_buffer(), OWNER_1);

    // Proof parity on the sparse read.
    let (proof, _) = bound
        .clone()
        .execute_with_proof(&drive, None, None, platform_version())
        .expect("sparse proof generation");
    let (_root, verified) = bound
        .verify_proof(proof.as_slice(), platform_version())
        .expect("sparse proof verification");
    assert_eq!(verified.len(), 1);
    assert_eq!(verified[0].id(), documents[0].id());

    assert_grovedb_is_consistent(&drive);
}

/// Present-but-empty is NOT absent: an empty byte-array trigger encodes to
/// an empty tree key (the null-layout lane) and the row participates in
/// the skip index; walker, probes, and commitment all agree, and the two
/// forms coexist and delete independently.
#[test]
fn empty_byte_array_trigger_takes_the_value_lane_not_the_skip_lane() {
    let (drive, contract) = setup_likes();
    let pin_path = sibling_doctype_path(&contract, "pin");

    let empty_tag = build_pin(&contract, Some(&[]), "x1", "y1", OWNER_1, 1);
    let absent_tag = build_pin(&contract, None, "x2", "y2", OWNER_1, 2);

    // The commitments must differ even over otherwise-identical values:
    // absent emits nothing, empty emits `name ‖ 00000000`.
    let empty_on_same_values = build_pin(&contract, Some(&[]), "x2", "y2", OWNER_1, 3);
    let commitment_empty = crate::drive::document::index_only_row_commitment(
        &empty_on_same_values,
        pin_type_ref(&contract),
        platform_version(),
    )
    .expect("commitment computes");
    let commitment_absent = crate::drive::document::index_only_row_commitment(
        &absent_tag,
        pin_type_ref(&contract),
        platform_version(),
    )
    .expect("commitment computes");
    assert_ne!(
        commitment_empty, commitment_absent,
        "absent and present-but-empty must commit differently"
    );

    insert_pin(&drive, &contract, &empty_tag, true).expect("insert empty-tag pin");
    insert_pin(&drive, &contract, &absent_tag, true).expect("insert absent-tag pin");

    // The empty-tag row's skip-index entry sits under the EMPTY value key.
    let mut empty_lane = pin_path.clone();
    empty_lane.extend([
        b"tag".to_vec(),
        Vec::new(),
        b"x".to_vec(),
        b"x1".to_vec(),
        b"y".to_vec(),
        b"y1".to_vec(),
        vec![0],
    ]);
    assert!(
        matches!(
            read_grove_element(&drive, &empty_lane, &OWNER_1),
            Some(Element::Item(..))
        ),
        "the empty-encoding trigger writes a real entry under the empty key"
    );

    // Both rows delete with their true forms.
    delete_pin(&drive, &contract, empty_tag, true).expect("delete empty-tag pin");
    assert!(
        read_grove_element(&drive, &empty_lane, &OWNER_1).is_none(),
        "the empty-lane entry must be gone"
    );
    delete_pin(&drive, &contract, absent_tag, true).expect("delete absent-tag pin");

    assert_grovedb_is_consistent(&drive);
}
