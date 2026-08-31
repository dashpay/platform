//! End-to-end coverage for **chained document queries** (provable
//! semi-join): `SELECT * FROM post WHERE $id IN (SELECT postId FROM like
//! WHERE $ownerId = <me>)` against the `yappr-likes` fixture — the inner
//! byLiker terminal route's proven postIds become the outer post query's
//! primary keys, and both proofs must verify to ONE root hash.
//!
//! Pinned here: no-proof/proof parity (the verifier's composed result
//! equals the server's materialized result), the empty-inner shape (no
//! outer proof), pagination through the inner terminal cursor, the
//! validation rejections, and the verifier's root-equality check (two
//! proofs straddling a state change are refused).

use super::index_only_e2e_tests::{build_like, insert_like, platform_version, setup_likes};
use crate::error::Error;
use crate::query::drive_chained_document_query::DriveChainedDocumentQuery;
use crate::query::{DriveDocumentQuery, OrderClause, WhereClause, WhereOperator};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;

const POST_A: [u8; 32] = [0xA1; 32];
const POST_B: [u8; 32] = [0xB2; 32];
const POST_C: [u8; 32] = [0xC3; 32];
const OWNER_1: [u8; 32] = [0x11; 32];
const OWNER_2: [u8; 32] = [0x22; 32];
const OWNER_3: [u8; 32] = [0x33; 32];

/// Inserts a `post` document (regular, non-indexOnly type) with an
/// explicit id so likes can reference it.
fn insert_post(
    drive: &crate::drive::Drive,
    contract: &DataContract,
    id: [u8; 32],
    hashtag: &str,
    message: &str,
    seed: u64,
) {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("post")
        .expect("post doctype exists");
    let mut doc = document_type
        .random_document(Some(seed), pv)
        .expect("random post");
    let mut props = std::collections::BTreeMap::new();
    props.insert("hashtag".to_string(), Value::Text(hashtag.to_string()));
    props.insert("message".to_string(), Value::Text(message.to_string()));
    doc.set_properties(props);
    doc.set_id(Identifier::from(id));
    doc.set_owner_id(Identifier::from(OWNER_1));
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&doc, None)),
                    owner_id: None,
                },
                contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("insert post");
}

/// The inner byLiker query: `$ownerId == owner`, with an optional
/// pagination cursor `postId > after` (ordered by postId).
fn my_likes_query<'a>(
    contract: &'a DataContract,
    owner: [u8; 32],
    after: Option<[u8; 32]>,
    limit: Option<u16>,
) -> DriveDocumentQuery<'a> {
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let mut clauses = vec![WhereClause {
        field: "$ownerId".to_string(),
        operator: WhereOperator::Equal,
        value: Value::Identifier(owner),
    }];
    let mut order_by: indexmap::IndexMap<String, OrderClause> = Default::default();
    if let Some(after) = after {
        clauses.push(WhereClause {
            field: "postId".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Identifier(after),
        });
        order_by.insert(
            "postId".to_string(),
            OrderClause {
                field: "postId".to_string(),
                ascending: true,
            },
        );
    }
    DriveDocumentQuery {
        contract,
        document_type,
        internal_clauses: crate::query::InternalClauses::extract_from_clauses(
            clauses,
            platform_version(),
        )
        .expect("clauses extract"),
        offset: None,
        limit,
        order_by,
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
        resolved_time_ranges: vec![],
    }
}

fn chained_posts_i_liked<'a>(
    contract: &'a DataContract,
    owner: [u8; 32],
    after: Option<[u8; 32]>,
    limit: Option<u16>,
) -> DriveChainedDocumentQuery<'a> {
    DriveChainedDocumentQuery {
        inner: my_likes_query(contract, owner, after, limit),
        join_property: "postId".to_string(),
        outer_document_type: contract
            .document_type_for_name("post")
            .expect("post doctype exists"),
    }
}

/// The full round trip: the server's materialized result and the
/// verifier's composed result must agree half for half, and both proofs
/// must verify to one root hash.
#[test]
fn chained_query_returns_liked_posts_with_proof_parity() {
    let (drive, contract) = setup_likes();
    let pv = platform_version();

    insert_post(&drive, &contract, POST_A, "dash", "post a", 10);
    insert_post(&drive, &contract, POST_B, "dash", "post b", 11);
    insert_post(&drive, &contract, POST_C, "btc", "post c", 12);
    for (post, hashtag, owner, seed) in [
        (POST_A, "dash", OWNER_1, 1u64),
        (POST_B, "dash", OWNER_1, 2),
        (POST_C, "btc", OWNER_2, 3),
    ] {
        let like = build_like(&contract, hashtag, post, owner, seed);
        insert_like(&drive, &contract, &like, true).expect("insert like");
    }

    let chained = chained_posts_i_liked(&contract, OWNER_1, None, Some(10));

    // No-proof execution.
    let outcome = drive
        .query_chained_documents(&chained, None, None, pv)
        .expect("chained query executes");
    assert_eq!(
        outcome.result.inner_documents.len(),
        2,
        "OWNER_1 has 2 likes"
    );
    let outer_ids: Vec<[u8; 32]> = outcome
        .result
        .outer_documents
        .iter()
        .map(|d| d.id().to_buffer())
        .collect();
    assert_eq!(
        outer_ids,
        vec![POST_A, POST_B],
        "the liked posts come back in inner (postId) order"
    );
    assert_eq!(
        outcome.result.outer_documents[0]
            .properties()
            .get("message")
            .expect("post body present")
            .to_str()
            .expect("text"),
        "post a",
        "outer documents are the full post bodies"
    );

    // Proof round trip.
    let (bundle, proved_result) = drive
        .query_chained_documents_with_proofs(&chained, None, pv)
        .expect("chained proofs generate");
    let outer_proof = bundle
        .outer_proof
        .as_deref()
        .expect("non-empty inner page must carry an outer proof");
    let (_root_hash, verified) = chained
        .verify_chained_documents_proof(bundle.inner_proof.as_slice(), Some(outer_proof), pv)
        .expect("chained proof verifies");
    assert_eq!(
        verified
            .outer_documents
            .iter()
            .map(|d| d.id())
            .collect::<Vec<_>>(),
        proved_result
            .outer_documents
            .iter()
            .map(|d| d.id())
            .collect::<Vec<_>>(),
        "verifier and server agree on the outer half"
    );
    assert_eq!(
        verified
            .inner_documents
            .iter()
            .map(|d| d.id())
            .collect::<Vec<_>>(),
        proved_result
            .inner_documents
            .iter()
            .map(|d| d.id())
            .collect::<Vec<_>>(),
        "verifier and server agree on the inner half"
    );
}

/// An empty inner page proves alone: no outer proof exists, and the
/// verifier refuses a spurious one.
#[test]
fn chained_query_empty_inner_has_no_outer_proof() {
    let (drive, contract) = setup_likes();
    let pv = platform_version();
    insert_post(&drive, &contract, POST_A, "dash", "post a", 10);

    let chained = chained_posts_i_liked(&contract, OWNER_3, None, Some(10));
    let (bundle, result) = drive
        .query_chained_documents_with_proofs(&chained, None, pv)
        .expect("chained proofs generate");
    assert!(result.inner_documents.is_empty());
    assert!(bundle.outer_proof.is_none(), "empty inner ⇒ no outer proof");

    let (_root, verified) = chained
        .verify_chained_documents_proof(bundle.inner_proof.as_slice(), None, pv)
        .expect("empty chained proof verifies");
    assert!(verified.outer_documents.is_empty());

    // A spurious outer proof (any bytes prove SOMETHING here — reuse the
    // inner proof) must be refused before it is even parsed.
    let refused = chained.verify_chained_documents_proof(
        bundle.inner_proof.as_slice(),
        Some(bundle.inner_proof.as_slice()),
        pv,
    );
    assert!(
        matches!(refused, Err(Error::Proof(_))),
        "outer proof with an empty inner page must be refused, got {refused:?}"
    );
}

/// Pagination lives on the INNER query alone: each page re-derives its
/// own outer half from that page's proven join values.
#[test]
fn chained_query_paginates_through_inner_cursor() {
    let (drive, contract) = setup_likes();
    let pv = platform_version();
    insert_post(&drive, &contract, POST_A, "dash", "post a", 10);
    insert_post(&drive, &contract, POST_B, "dash", "post b", 11);
    for (post, seed) in [(POST_A, 1u64), (POST_B, 2)] {
        let like = build_like(&contract, "dash", post, OWNER_1, seed);
        insert_like(&drive, &contract, &like, true).expect("insert like");
    }

    let page_1 = chained_posts_i_liked(&contract, OWNER_1, None, Some(1));
    let outcome_1 = drive
        .query_chained_documents(&page_1, None, None, pv)
        .expect("page 1 executes");
    assert_eq!(outcome_1.result.outer_documents.len(), 1);
    assert_eq!(outcome_1.result.outer_documents[0].id().to_buffer(), POST_A);

    // The cursor is the page's last join value, read off the inner
    // projections — exactly what a client would do.
    let cursor: [u8; 32] = outcome_1.result.inner_documents[0]
        .properties()
        .get("postId")
        .expect("join value present")
        .to_identifier()
        .expect("identifier")
        .to_buffer();

    let page_2 = chained_posts_i_liked(&contract, OWNER_1, Some(cursor), Some(1));
    let (bundle, _) = drive
        .query_chained_documents_with_proofs(&page_2, None, pv)
        .expect("page 2 proofs generate");
    let (_root, verified) = page_2
        .verify_chained_documents_proof(
            bundle.inner_proof.as_slice(),
            bundle.outer_proof.as_deref(),
            pv,
        )
        .expect("page 2 verifies");
    assert_eq!(verified.outer_documents.len(), 1);
    assert_eq!(verified.outer_documents[0].id().to_buffer(), POST_B);
}

/// The validation rejections: shapes that could not compose soundly are
/// refused identically on the server and the verifier (both call
/// `validate`).
#[test]
fn chained_query_validation_rejections() {
    let (drive, contract) = setup_likes();
    let pv = platform_version();

    // Missing inner limit — the bound on the outer fan-out.
    let mut no_limit = chained_posts_i_liked(&contract, OWNER_1, None, None);
    assert!(
        matches!(
            drive.query_chained_documents(&no_limit, None, None, pv),
            Err(Error::Query(_))
        ),
        "an inner limit is required"
    );
    no_limit.inner.limit = Some(10);

    // Join property without a refersTo declaration.
    let mut bad_join = chained_posts_i_liked(&contract, OWNER_1, None, Some(10));
    bad_join.join_property = "hashtag".to_string();
    assert!(
        matches!(
            drive.query_chained_documents(&bad_join, None, None, pv),
            Err(Error::Query(_))
        ),
        "the join property must carry refersTo: permanentDocument"
    );

    // Outer type that is not the refersTo target (and is itself
    // indexOnly, which is refused in its own right).
    let mut bad_outer = chained_posts_i_liked(&contract, OWNER_1, None, Some(10));
    bad_outer.outer_document_type = contract
        .document_type_for_name("tip")
        .expect("tip doctype exists");
    assert!(
        matches!(
            drive.query_chained_documents(&bad_outer, None, None, pv),
            Err(Error::Query(_))
        ),
        "the outer type must be the refersTo target"
    );
}

/// A like whose referenced post is missing is corrupted state at the
/// drive level (consensus validates references on write): the chained
/// execution refuses to return a partial join.
#[test]
fn chained_query_refuses_dangling_reference() {
    let (drive, contract) = setup_likes();
    let pv = platform_version();
    // A like referencing POST_A — which was never inserted.
    let like = build_like(&contract, "dash", POST_A, OWNER_1, 1);
    insert_like(&drive, &contract, &like, true).expect("insert like");

    let chained = chained_posts_i_liked(&contract, OWNER_1, None, Some(10));
    let refused = drive.query_chained_documents(&chained, None, None, pv);
    assert!(
        matches!(refused, Err(Error::Proof(_))),
        "a dangling reference must refuse the join, got {refused:?}"
    );
}

/// Two individually valid proofs that straddle a state change describe
/// two different states — the verifier's root-equality check refuses the
/// composition.
#[test]
fn chained_query_rejects_proofs_of_different_roots() {
    let (drive, contract) = setup_likes();
    let pv = platform_version();
    insert_post(&drive, &contract, POST_A, "dash", "post a", 10);
    let like = build_like(&contract, "dash", POST_A, OWNER_1, 1);
    insert_like(&drive, &contract, &like, true).expect("insert like");

    let chained = chained_posts_i_liked(&contract, OWNER_1, None, Some(10));
    let (bundle, result) = drive
        .query_chained_documents_with_proofs(&chained, None, pv)
        .expect("chained proofs generate");

    // Mutate state, then regenerate ONLY the outer proof — it now
    // proves the same documents against a different root.
    insert_post(&drive, &contract, POST_B, "dash", "post b", 11);
    let join_values = chained
        .join_values(&result.inner_documents)
        .expect("join values extract");
    let (stale_outer_proof, _) = chained
        .derive_outer_query(&join_values)
        .execute_with_proof(&drive, None, None, pv)
        .expect("outer proof at the new root");

    let refused = chained.verify_chained_documents_proof(
        bundle.inner_proof.as_slice(),
        Some(stale_outer_proof.as_slice()),
        pv,
    );
    assert!(
        matches!(refused, Err(Error::Proof(_))),
        "proofs of different roots must be refused, got {refused:?}"
    );
}
