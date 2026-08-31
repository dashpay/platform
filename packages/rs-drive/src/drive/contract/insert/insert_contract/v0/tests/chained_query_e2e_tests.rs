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
fn should_return_liked_posts_with_proof_parity() {
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

    // Proof round trip: ONE merged proof, verified against the query
    // re-derived from the server's join-value hint. The with-proof path
    // materializes only the inner projections — the outer half rides
    // the proof.
    let (proof, proved_inner) = drive
        .query_chained_documents_with_proof(&chained, pv)
        .expect("chained proof generates");
    let hint = chained
        .join_values(&proved_inner)
        .expect("join values extract");
    let (_root_hash, verified) = chained
        .verify_chained_documents_proof(proof.as_slice(), &hint, pv)
        .expect("chained proof verifies");
    assert_eq!(
        verified
            .outer_documents
            .iter()
            .map(|d| d.id())
            .collect::<Vec<_>>(),
        outcome
            .result
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
        proved_inner.iter().map(|d| d.id()).collect::<Vec<_>>(),
        "verifier and server agree on the inner half"
    );
}

/// An empty inner page proves alone (the merged query degenerates to
/// the inner component), and a hint claiming otherwise is refused: the
/// derived outer branch demands documents the proof cannot cover.
#[test]
fn should_prove_an_empty_inner_page_alone() {
    let (drive, contract) = setup_likes();
    let pv = platform_version();
    insert_post(&drive, &contract, POST_A, "dash", "post a", 10);

    let chained = chained_posts_i_liked(&contract, OWNER_3, None, Some(10));
    let (proof, proved_inner) = drive
        .query_chained_documents_with_proof(&chained, pv)
        .expect("chained proof generates");
    assert!(proved_inner.is_empty());

    let (_root, verified) = chained
        .verify_chained_documents_proof(proof.as_slice(), &[], pv)
        .expect("empty chained proof verifies");
    assert!(verified.outer_documents.is_empty());

    // A fabricated hint over an empty page: the merged query gains an
    // outer branch this proof never covered.
    let fake_hint = vec![dpp::identifier::Identifier::from(POST_A)];
    let refused = chained.verify_chained_documents_proof(proof.as_slice(), &fake_hint, pv);
    assert!(
        refused.is_err(),
        "a non-empty hint over an empty proven page must be refused, got {refused:?}"
    );
}

/// Pagination lives on the INNER query alone: each page re-derives its
/// own outer half from that page's proven join values.
#[test]
fn should_paginate_through_the_inner_cursor() {
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
    let (proof, proved_inner_2) = drive
        .query_chained_documents_with_proof(&page_2, pv)
        .expect("page 2 proof generates");
    let hint = page_2
        .join_values(&proved_inner_2)
        .expect("join values extract");
    let (_root, verified) = page_2
        .verify_chained_documents_proof(proof.as_slice(), &hint, pv)
        .expect("page 2 verifies");
    assert_eq!(verified.outer_documents.len(), 1);
    assert_eq!(verified.outer_documents[0].id().to_buffer(), POST_B);
}

/// The validation rejections: shapes that could not compose soundly are
/// refused identically on the server and the verifier (both call
/// `validate`).
#[test]
fn should_reject_invalid_chained_shapes() {
    let (drive, contract) = setup_likes();
    let pv = platform_version();

    // Missing inner limit — the bound on the outer fan-out.
    let no_limit = chained_posts_i_liked(&contract, OWNER_1, None, None);
    assert!(
        matches!(
            drive.query_chained_documents(&no_limit, None, None, pv),
            Err(Error::Query(_))
        ),
        "an inner limit is required"
    );

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

    // Inner limit above the outer `$id IN` clause's 100-value cap.
    let over_cap = chained_posts_i_liked(&contract, OWNER_1, None, Some(101));
    assert!(
        matches!(
            drive.query_chained_documents(&over_cap, None, None, pv),
            Err(Error::Query(_))
        ),
        "an inner limit above MAX_CHAINED_JOIN_VALUES must be refused"
    );

    // An oversized (necessarily lying) verifier-side hint is refused
    // before the outer derivation runs.
    let capped = chained_posts_i_liked(&contract, OWNER_1, None, Some(10));
    let oversized_hint: Vec<dpp::identifier::Identifier> = (0..101u8)
        .map(|i| dpp::identifier::Identifier::from([i; 32]))
        .collect();
    assert!(
        matches!(
            capped.verify_chained_documents_proof(&[], &oversized_hint, pv),
            Err(Error::Query(_))
        ),
        "an oversized join-value hint must be refused"
    );
}

/// A like whose referenced post is missing is corrupted state at the
/// drive level (consensus validates references on write): the chained
/// execution refuses to return a partial join.
#[test]
fn should_refuse_a_dangling_reference() {
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

/// The hint is untrusted: any lie — a dropped id, an extra id, a
/// substituted id — produces a merged query the proof cannot satisfy,
/// and verification fails rather than returning a steered join.
#[test]
fn should_reject_tampered_hints() {
    use dpp::identifier::Identifier;

    let (drive, contract) = setup_likes();
    let pv = platform_version();
    insert_post(&drive, &contract, POST_A, "dash", "post a", 10);
    insert_post(&drive, &contract, POST_B, "dash", "post b", 11);
    insert_post(&drive, &contract, POST_C, "btc", "post c", 12);
    for (post, hashtag, seed) in [(POST_A, "dash", 1u64), (POST_B, "dash", 2)] {
        let like = build_like(&contract, hashtag, post, OWNER_1, seed);
        insert_like(&drive, &contract, &like, true).expect("insert like");
    }

    let chained = chained_posts_i_liked(&contract, OWNER_1, None, Some(10));
    let (proof, proved_inner) = drive
        .query_chained_documents_with_proof(&chained, pv)
        .expect("chained proof generates");
    let honest_hint = chained
        .join_values(&proved_inner)
        .expect("join values extract");
    assert_eq!(honest_hint.len(), 2);

    // Dropped id: the merged query's outer branch misses a proven
    // reference.
    let dropped: Vec<Identifier> = honest_hint[..1].to_vec();
    assert!(
        chained
            .verify_chained_documents_proof(proof.as_slice(), &dropped, pv)
            .is_err(),
        "a hint missing a proven join value must be refused"
    );

    // Extra id: the merged query demands a document no proven join
    // value references (POST_C exists on chain, making this the
    // interesting injection case).
    let mut extra = honest_hint.clone();
    extra.push(Identifier::from(POST_C));
    assert!(
        chained
            .verify_chained_documents_proof(proof.as_slice(), &extra, pv)
            .is_err(),
        "a hint with an injected id must be refused"
    );

    // Substituted id.
    let mut substituted = honest_hint.clone();
    substituted[0] = Identifier::from(POST_C);
    assert!(
        chained
            .verify_chained_documents_proof(proof.as_slice(), &substituted, pv)
            .is_err(),
        "a hint with a substituted id must be refused"
    );

    // And the honest hint still verifies after all that.
    chained
        .verify_chained_documents_proof(proof.as_slice(), &honest_hint, pv)
        .expect("the honest hint verifies");
}
