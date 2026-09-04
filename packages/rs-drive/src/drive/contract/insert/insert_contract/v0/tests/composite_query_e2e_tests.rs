//! End-to-end coverage for **composite document queries**: a page of
//! posts plus everything a feed card renders for it — like and repost
//! counts, the quoted posts, the reposts, the authors' profiles (in
//! another contract), the quoted authors' profiles (derived from a
//! sub-query rather than the page), and the viewer's own likes — as ONE
//! merged proof against the `yappr-feed` fixture.
//!
//! Pinned here: no-proof/proof parity (the verifier's composed result
//! equals the server's materialized result), the empty-page shape, the
//! validation rejections, the fail-closed behaviour on a page-only proof
//! (what a node ignoring the sub-queries would serve), the dangling
//! reference refusal, and by-id routing when the page and a join share
//! the primary tree.

use crate::error::Error;
use crate::query::drive_composite_document_query::{
    BindingSource, DriveCompositeDocumentQuery, DriveSubQuery, SubQueryBinding, SubQueryKind,
    SubQueryResult, MAX_SUB_QUERIES,
};
use crate::query::{DriveDocumentQuery, InternalClauses, OrderClause, WhereClause, WhereOperator};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

const FEED_CONTRACT: &str = "tests/supporting_files/contract/yappr-feed/yappr-feed-contract.json";
const DASHPAY_CONTRACT: &str = "tests/supporting_files/contract/dashpay/dashpay-contract.json";

const POST_A: [u8; 32] = [0xA1; 32];
const POST_B: [u8; 32] = [0xB2; 32];
const POST_C: [u8; 32] = [0xC3; 32];
const POST_D: [u8; 32] = [0xD4; 32];
const MISSING_POST: [u8; 32] = [0xE5; 32];
const OWNER_1: [u8; 32] = [0x11; 32];
const OWNER_2: [u8; 32] = [0x22; 32];
const OWNER_3: [u8; 32] = [0x33; 32];

fn platform_version() -> &'static PlatformVersion {
    PlatformVersion::latest()
}

/// A drive with the feed contract and the dashpay contract (whose
/// `profile` type, keyed by `$ownerId`, plays the cross-contract lookup).
fn setup() -> (crate::drive::Drive, DataContract, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let mut contracts = Vec::new();
    for path in [FEED_CONTRACT, DASHPAY_CONTRACT] {
        let contract =
            json_document_to_contract(path, false, pv).expect("expected to parse the contract");
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
        contracts.push(contract);
    }
    let dashpay = contracts.pop().expect("dashpay");
    let feed = contracts.pop().expect("feed");
    (drive, feed, dashpay)
}

fn insert(drive: &crate::drive::Drive, contract: &DataContract, type_name: &str, doc: &Document) {
    let document_type = contract
        .document_type_for_name(type_name)
        .expect("doctype exists");
    drive
        .add_document_for_contract(
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
            true,
            None,
            platform_version(),
            None,
        )
        .expect("insert document");
}

fn build(contract: &DataContract, type_name: &str, seed: u64) -> Document {
    contract
        .document_type_for_name(type_name)
        .expect("doctype exists")
        .random_document(Some(seed), platform_version())
        .expect("random document")
}

fn insert_post(
    drive: &crate::drive::Drive,
    contract: &DataContract,
    id: [u8; 32],
    owner: [u8; 32],
    hashtag: &str,
    quoted: Option<[u8; 32]>,
    seed: u64,
) {
    let mut doc = build(contract, "post", seed);
    let mut props = BTreeMap::new();
    props.insert("hashtag".to_string(), Value::Text(hashtag.to_string()));
    props.insert("message".to_string(), Value::Text(format!("post {seed}")));
    if let Some(quoted) = quoted {
        props.insert("quotedPostId".to_string(), Value::Identifier(quoted));
    }
    doc.set_properties(props);
    doc.set_id(Identifier::from(id));
    doc.set_owner_id(Identifier::from(owner));
    insert(drive, contract, "post", &doc);
}

fn insert_like(
    drive: &crate::drive::Drive,
    contract: &DataContract,
    owner: [u8; 32],
    post: [u8; 32],
    hashtag: &str,
    seed: u64,
) {
    let mut doc = build(contract, "like", seed);
    let mut props = BTreeMap::new();
    props.insert("hashtag".to_string(), Value::Text(hashtag.to_string()));
    props.insert("postId".to_string(), Value::Identifier(post));
    doc.set_properties(props);
    doc.set_owner_id(Identifier::from(owner));
    insert(drive, contract, "like", &doc);
}

fn insert_repost(
    drive: &crate::drive::Drive,
    contract: &DataContract,
    owner: [u8; 32],
    post: [u8; 32],
    seed: u64,
) {
    let mut doc = build(contract, "repost", seed);
    let mut props = BTreeMap::new();
    props.insert("postId".to_string(), Value::Identifier(post));
    doc.set_properties(props);
    doc.set_owner_id(Identifier::from(owner));
    insert(drive, contract, "repost", &doc);
}

fn insert_profile(
    drive: &crate::drive::Drive,
    dashpay: &DataContract,
    owner: [u8; 32],
    display_name: &str,
    seed: u64,
) {
    let mut doc = build(dashpay, "profile", seed);
    let mut props = BTreeMap::new();
    props.insert(
        "displayName".to_string(),
        Value::Text(display_name.to_string()),
    );
    doc.set_properties(props);
    doc.set_owner_id(Identifier::from(owner));
    insert(drive, dashpay, "profile", &doc);
}

/// The feed fixture: three `dash` posts (the page), one `btc` post two
/// of them quote, likes, reposts and two profiles.
///
/// | post | owner | tag  | quotes | likes by      | reposts by     |
/// |------|-------|------|--------|---------------|----------------|
/// | A    | 1     | dash | D      | 1, 2          | 2              |
/// | B    | 2     | dash | —      | 1             | 1, 3           |
/// | C    | 3     | dash | D      | —             | —              |
/// | D    | 3     | btc  | —      | 3             | —              |
///
/// Profiles exist for owners 1 and 3 only.
fn seed_feed(drive: &crate::drive::Drive, feed: &DataContract, dashpay: &DataContract) {
    insert_post(drive, feed, POST_D, OWNER_3, "btc", None, 4);
    insert_post(drive, feed, POST_A, OWNER_1, "dash", Some(POST_D), 1);
    insert_post(drive, feed, POST_B, OWNER_2, "dash", None, 2);
    insert_post(drive, feed, POST_C, OWNER_3, "dash", Some(POST_D), 3);
    insert_like(drive, feed, OWNER_1, POST_A, "dash", 10);
    insert_like(drive, feed, OWNER_2, POST_A, "dash", 11);
    insert_like(drive, feed, OWNER_1, POST_B, "dash", 12);
    insert_like(drive, feed, OWNER_3, POST_D, "btc", 13);
    insert_repost(drive, feed, OWNER_2, POST_A, 20);
    insert_repost(drive, feed, OWNER_1, POST_B, 21);
    insert_repost(drive, feed, OWNER_3, POST_B, 22);
    insert_profile(drive, dashpay, OWNER_1, "one", 30);
    insert_profile(drive, dashpay, OWNER_3, "three", 31);
}

fn page_by_hashtag<'a>(
    contract: &'a DataContract,
    hashtag: &str,
    limit: Option<u16>,
) -> DriveDocumentQuery<'a> {
    DriveDocumentQuery {
        contract,
        document_type: contract.document_type_for_name("post").expect("post"),
        internal_clauses: InternalClauses::extract_from_clauses(
            vec![WhereClause {
                field: "hashtag".to_string(),
                operator: WhereOperator::Equal,
                value: Value::Text(hashtag.to_string()),
            }],
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

fn bound<'a>(
    contract: &'a DataContract,
    type_name: &str,
    kind: SubQueryKind,
    source: BindingSource,
    source_property: &str,
    field: &str,
    limit: Option<u16>,
) -> DriveSubQuery<'a> {
    DriveSubQuery {
        contract,
        document_type: contract.document_type_for_name(type_name).expect("doctype"),
        kind,
        where_clauses: vec![],
        order_by: vec![],
        limit,
        binding: Some(SubQueryBinding {
            source,
            source_property: source_property.to_string(),
            field: field.to_string(),
        }),
    }
}

/// Sub-query positions in [`feed_query`].
const LIKE_COUNTS: usize = 0;
const QUOTED_POSTS: usize = 1;
const REPOSTS: usize = 2;
const AUTHOR_PROFILES: usize = 3;
const QUOTED_AUTHOR_PROFILES: usize = 4;
const VIEWER_LIKES: usize = 5;

/// The whole feed composition: like counts, the quoted posts, the
/// reposts themselves (their count is a client-side length; a count on
/// the same `byPost` index would read the value trees the documents
/// lookup descends past), the authors' profiles, the quoted authors'
/// profiles. `viewer` adds the "which of these did I like" lookup on
/// the indexOnly `like` type — proof-path only, since its `byLiker`
/// projection does not cover every property and so cannot be
/// materialized into a non-proof response.
fn feed_query<'a>(
    feed: &'a DataContract,
    dashpay: &'a DataContract,
    viewer: Option<[u8; 32]>,
) -> DriveCompositeDocumentQuery<'a> {
    let mut sub_queries = vec![
        bound(
            feed,
            "like",
            SubQueryKind::Count,
            BindingSource::Page,
            "$id",
            "postId",
            None,
        ),
        bound(
            feed,
            "post",
            SubQueryKind::Documents,
            BindingSource::Page,
            "quotedPostId",
            "$id",
            None,
        ),
        bound(
            feed,
            "repost",
            SubQueryKind::Documents,
            BindingSource::Page,
            "$id",
            "postId",
            Some(50),
        ),
        // Profiles are unique per owner: value-bounded, so no limit.
        bound(
            dashpay,
            "profile",
            SubQueryKind::Documents,
            BindingSource::Page,
            "$ownerId",
            "$ownerId",
            None,
        ),
        bound(
            dashpay,
            "profile",
            SubQueryKind::Documents,
            BindingSource::SubQuery(QUOTED_POSTS),
            "$ownerId",
            "$ownerId",
            None,
        ),
    ];
    if let Some(viewer) = viewer {
        // `byLiker` is `[$ownerId] → postId`: with the owner fixed, the
        // terminal postId is unique per value — value-bounded, no limit.
        let mut marks = bound(
            feed,
            "like",
            SubQueryKind::Documents,
            BindingSource::Page,
            "$id",
            "postId",
            None,
        );
        marks.where_clauses = vec![WhereClause {
            field: "$ownerId".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(viewer),
        }];
        sub_queries.push(marks);
    }
    DriveCompositeDocumentQuery {
        page: page_by_hashtag(feed, "dash", Some(10)),
        sub_queries,
    }
}

fn ids(documents: &[Document]) -> Vec<[u8; 32]> {
    documents.iter().map(|d| d.id().to_buffer()).collect()
}

fn counts(result: &SubQueryResult) -> BTreeMap<[u8; 32], u64> {
    result
        .counts()
        .iter()
        .map(|entry| {
            let key: [u8; 32] = entry.key.as_slice().try_into().expect("identifier key");
            (key, entry.count.expect("present count"))
        })
        .collect()
}

fn post_ids_of(result: &SubQueryResult, property: &str) -> Vec<[u8; 32]> {
    result
        .documents()
        .iter()
        .map(|d| {
            d.properties()
                .get(property)
                .expect("property present")
                .to_identifier()
                .expect("identifier")
                .to_buffer()
        })
        .collect()
}

fn owner_ids(documents: &[Document]) -> Vec<[u8; 32]> {
    documents.iter().map(|d| d.owner_id().to_buffer()).collect()
}

/// The full round trip: the server's materialized result and the
/// verifier's composed result agree component for component.
#[test]
fn should_answer_the_feed_composition_with_proof_parity() {
    let (drive, feed, dashpay) = setup();
    seed_feed(&drive, &feed, &dashpay);
    let pv = platform_version();
    let query = feed_query(&feed, &dashpay, None);

    let materialized = drive
        .query_composite_documents(&query, None, None, pv)
        .expect("no-proof composite executes")
        .result;
    let (proof, page) = drive
        .query_composite_documents_with_proof(&query, pv)
        .expect("composite proves");
    let (_root, verified) = query
        .verify_composite_documents_proof(&proof, pv)
        .expect("the merged proof verifies");

    // The page: the three `dash` posts, in index order.
    assert_eq!(
        ids(&materialized.page_documents),
        vec![POST_A, POST_B, POST_C]
    );
    assert_eq!(ids(&page), vec![POST_A, POST_B, POST_C]);
    assert_eq!(verified.page_documents, materialized.page_documents);

    for result in [&materialized, &verified] {
        assert_eq!(
            counts(&result.sub_results[LIKE_COUNTS]),
            BTreeMap::from([(POST_A, 2), (POST_B, 1)]),
            "C has no like tree and D is off the page"
        );
        assert_eq!(
            ids(result.sub_results[QUOTED_POSTS].documents()),
            vec![POST_D],
            "A and C both quote D: one derived id, one document"
        );
        assert_eq!(
            post_ids_of(&result.sub_results[REPOSTS], "postId"),
            vec![POST_A, POST_B, POST_B]
        );
        assert_eq!(
            owner_ids(result.sub_results[AUTHOR_PROFILES].documents()),
            vec![OWNER_1, OWNER_3],
            "owner 2 has no profile: a proven absence, not an error"
        );
        assert_eq!(
            owner_ids(result.sub_results[QUOTED_AUTHOR_PROFILES].documents()),
            vec![OWNER_3],
            "derived from the quoted-posts sub-query, not the page"
        );
    }
    assert_eq!(verified.sub_results, materialized.sub_results);
}

/// The viewer's own likes ride the same proof as an indexOnly lookup
/// pinned on `$ownerId`: the synthesized projections carry the post ids
/// the viewer liked among the page.
#[test]
fn should_prove_the_viewers_marks_as_an_index_only_lookup() {
    let (drive, feed, dashpay) = setup();
    seed_feed(&drive, &feed, &dashpay);
    let pv = platform_version();
    let query = feed_query(&feed, &dashpay, Some(OWNER_1));

    let (proof, _) = drive
        .query_composite_documents_with_proof(&query, pv)
        .expect("composite proves");
    let (_root, verified) = query
        .verify_composite_documents_proof(&proof, pv)
        .expect("the merged proof verifies");

    assert_eq!(
        post_ids_of(&verified.sub_results[VIEWER_LIKES], "postId"),
        vec![POST_A, POST_B]
    );
    assert!(verified.sub_results[VIEWER_LIKES]
        .documents()
        .iter()
        .all(|like| like.owner_id().to_buffer() == OWNER_1));

    let query = feed_query(&feed, &dashpay, Some(OWNER_2));
    let (proof, _) = drive
        .query_composite_documents_with_proof(&query, pv)
        .expect("composite proves");
    let (_root, verified) = query
        .verify_composite_documents_proof(&proof, pv)
        .expect("verifies");
    assert_eq!(
        post_ids_of(&verified.sub_results[VIEWER_LIKES], "postId"),
        vec![POST_A]
    );
}

/// An empty page derives nothing: every sub-query is empty and the proof
/// is the page's alone.
#[test]
fn should_prove_an_empty_page_alone() {
    let (drive, feed, dashpay) = setup();
    seed_feed(&drive, &feed, &dashpay);
    let pv = platform_version();
    let mut query = feed_query(&feed, &dashpay, Some(OWNER_1));
    query.page = page_by_hashtag(&feed, "nothing", Some(10));

    let materialized = drive
        .query_composite_documents(&query, None, None, pv)
        .expect("executes")
        .result;
    assert!(materialized.page_documents.is_empty());
    assert!(materialized
        .sub_results
        .iter()
        .all(|result| result.documents().is_empty() && result.counts().is_empty()));

    let (proof, page) = drive
        .query_composite_documents_with_proof(&query, pv)
        .expect("proves");
    assert!(page.is_empty());
    let (_root, verified) = query
        .verify_composite_documents_proof(&proof, pv)
        .expect("verifies");
    assert!(verified.page_documents.is_empty());
    assert_eq!(verified.sub_results.len(), query.sub_queries.len());
}

/// A proof covering only the page — what a node that ignores the
/// sub-queries would serve — cannot satisfy the merged query.
#[test]
fn should_refuse_a_page_only_proof() {
    let (drive, feed, dashpay) = setup();
    seed_feed(&drive, &feed, &dashpay);
    let pv = platform_version();
    let query = feed_query(&feed, &dashpay, None);

    let (page_only_proof, _cost) = query
        .page
        .clone()
        .execute_with_proof(&drive, None, None, pv)
        .expect("the page alone proves");
    assert!(
        query
            .verify_composite_documents_proof(&page_only_proof, pv)
            .is_err(),
        "a page-only proof must fail the composite verification"
    );
}

/// A by-id join whose derived id has no document is an invalid proof
/// (and corrupted state on the server): a permanentDocument reference
/// cannot dangle.
#[test]
fn should_refuse_a_dangling_reference() {
    let (drive, feed, _dashpay) = setup();
    insert_post(
        &drive,
        &feed,
        POST_A,
        OWNER_1,
        "dash",
        Some(MISSING_POST),
        1,
    );
    let pv = platform_version();
    let query = DriveCompositeDocumentQuery {
        page: page_by_hashtag(&feed, "dash", Some(10)),
        sub_queries: vec![bound(
            &feed,
            "post",
            SubQueryKind::Documents,
            BindingSource::Page,
            "quotedPostId",
            "$id",
            None,
        )],
    };

    let refused = drive.query_composite_documents(&query, None, None, pv);
    assert!(
        matches!(refused, Err(Error::Proof(_))),
        "expected the missing-document refusal, got {refused:?}"
    );
    let (proof, _) = drive
        .query_composite_documents_with_proof(&query, pv)
        .expect("the proof itself generates");
    assert!(
        query.verify_composite_documents_proof(&proof, pv).is_err(),
        "the verifier must refuse a dangling reference"
    );
}

/// When the page is itself a by-ids fetch and a join targets the same
/// type, both land in the primary tree: the page keeps its own ids, the
/// join keeps the derived ones.
#[test]
fn should_tell_a_by_ids_page_from_a_join_on_the_same_type() {
    let (drive, feed, dashpay) = setup();
    seed_feed(&drive, &feed, &dashpay);
    let pv = platform_version();
    let post_type = feed.document_type_for_name("post").expect("post");
    let page = DriveDocumentQuery {
        contract: &feed,
        document_type: post_type,
        internal_clauses: InternalClauses {
            primary_key_in_clause: Some(WhereClause {
                field: "$id".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(vec![Value::Identifier(POST_A), Value::Identifier(POST_B)]),
            }),
            primary_key_equal_clause: None,
            in_clauses: vec![],
            range_clause: None,
            equal_clauses: Default::default(),
        },
        offset: None,
        limit: Some(2),
        order_by: Default::default(),
        start_at: None,
        start_at_included: false,
        block_time_ms: None,
        resolved_time_ranges: vec![],
    };
    let query = DriveCompositeDocumentQuery {
        page,
        sub_queries: vec![bound(
            &feed,
            "post",
            SubQueryKind::Documents,
            BindingSource::Page,
            "quotedPostId",
            "$id",
            None,
        )],
    };

    let materialized = drive
        .query_composite_documents(&query, None, None, pv)
        .expect("executes")
        .result;
    let (proof, _) = drive
        .query_composite_documents_with_proof(&query, pv)
        .expect("proves");
    let (_root, verified) = query
        .verify_composite_documents_proof(&proof, pv)
        .expect("verifies");
    for result in [&materialized, &verified] {
        assert_eq!(ids(&result.page_documents), vec![POST_A, POST_B]);
        assert_eq!(ids(result.sub_results[0].documents()), vec![POST_D]);
    }
}

#[test]
fn should_reject_invalid_composite_shapes() {
    let (drive, feed, dashpay) = setup();
    let pv = platform_version();
    let base = feed_query(&feed, &dashpay, Some(OWNER_1));
    let expect_unsupported = |query: DriveCompositeDocumentQuery, what: &str| {
        let result = drive.query_composite_documents(&query, None, None, pv);
        assert!(
            matches!(result, Err(Error::Query(_))),
            "{what}: expected a query rejection, got {result:?}"
        );
    };

    let mut no_limit = base.clone();
    no_limit.page.limit = None;
    expect_unsupported(no_limit, "page without a limit");

    let mut oversized = base.clone();
    oversized.page.limit = Some(101);
    expect_unsupported(oversized, "page limit above the bound-value cap");

    let mut none = base.clone();
    none.sub_queries.clear();
    expect_unsupported(none, "no sub-queries");

    let mut too_many = base.clone();
    let extra = too_many.sub_queries[LIKE_COUNTS].clone();
    while too_many.sub_queries.len() <= MAX_SUB_QUERIES {
        too_many.sub_queries.push(extra.clone());
    }
    expect_unsupported(too_many, "more sub-queries than the cap");

    let mut counted_with_limit = base.clone();
    counted_with_limit.sub_queries[LIKE_COUNTS].limit = Some(5);
    expect_unsupported(counted_with_limit, "count with a limit");

    let mut unbound_count = base.clone();
    unbound_count.sub_queries[LIKE_COUNTS].binding = None;
    expect_unsupported(unbound_count, "unbound count");

    let mut join_without_reference = base.clone();
    join_without_reference.sub_queries[QUOTED_POSTS]
        .binding
        .as_mut()
        .expect("bound")
        .source_property = "$ownerId".to_string();
    expect_unsupported(
        join_without_reference,
        "by-id join from a non-refersTo source",
    );

    let mut join_with_limit = base.clone();
    join_with_limit.sub_queries[QUOTED_POSTS].limit = Some(5);
    expect_unsupported(join_with_limit, "by-id join with a limit");

    let mut lookup_without_limit = base.clone();
    lookup_without_limit.sub_queries[REPOSTS].limit = None;
    expect_unsupported(lookup_without_limit, "non-unique lookup without a limit");

    let mut bounded_lookup_with_limit = base.clone();
    bounded_lookup_with_limit.sub_queries[AUTHOR_PROFILES].limit = Some(20);
    expect_unsupported(
        bounded_lookup_with_limit,
        "value-bounded lookup with a limit",
    );

    let mut forward_binding = base.clone();
    forward_binding.sub_queries[LIKE_COUNTS]
        .binding
        .as_mut()
        .expect("bound")
        .source = BindingSource::SubQuery(QUOTED_POSTS);
    expect_unsupported(forward_binding, "binding to a later sub-query");

    let mut bound_to_a_count = base.clone();
    bound_to_a_count.sub_queries[QUOTED_AUTHOR_PROFILES]
        .binding
        .as_mut()
        .expect("bound")
        .source = BindingSource::SubQuery(LIKE_COUNTS);
    expect_unsupported(bound_to_a_count, "binding to a count sub-query");

    let mut fixed_on_bound_field = base.clone();
    fixed_on_bound_field.sub_queries[REPOSTS]
        .where_clauses
        .push(WhereClause {
            field: "postId".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Identifier(POST_A),
        });
    expect_unsupported(fixed_on_bound_field, "fixed clause on the bound field");

    let mut unknown_property = base.clone();
    unknown_property.sub_queries[LIKE_COUNTS]
        .binding
        .as_mut()
        .expect("bound")
        .source_property = "nope".to_string();
    expect_unsupported(unknown_property, "unknown source property");

    let mut non_identifier_property = base.clone();
    non_identifier_property.sub_queries[LIKE_COUNTS]
        .binding
        .as_mut()
        .expect("bound")
        .source_property = "hashtag".to_string();
    expect_unsupported(non_identifier_property, "non-identifier source property");

    let mut ordered_join = base.clone();
    ordered_join.sub_queries[QUOTED_POSTS].order_by = vec![OrderClause {
        field: "hashtag".to_string(),
        ascending: true,
    }];
    expect_unsupported(ordered_join, "ordered by-id join");

    let mut count_on_a_looked_up_index = base.clone();
    count_on_a_looked_up_index.sub_queries.push(bound(
        &feed,
        "repost",
        SubQueryKind::Count,
        BindingSource::Page,
        "$id",
        "postId",
        None,
    ));
    expect_unsupported(
        count_on_a_looked_up_index,
        "count on the index a documents lookup reads through",
    );
}
