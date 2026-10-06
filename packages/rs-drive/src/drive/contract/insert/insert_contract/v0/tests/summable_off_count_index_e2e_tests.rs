//! End-to-end coverage for **`summableOffCountIndex` indexes** (meta schema
//! v3 / PV14): an indexOnly index that keeps, per group, one `SumItem`
//! counting its source index's entries in that group, in place of a value
//! tree, a `0` bucket and one entry per document.
//!
//! Runs against the `yappr-likes-summable-off-count-index` fixture: a `like`
//! whose `byPost` keeps one entry per like, and whose `byAuthorPost` and
//! `byHashtagPost` keep one counter per post, reached through the post
//! reference's `where` (the post's owner and hashtag). In the count-and-sum
//! trees above them each counter counts one post and adds its likes, so the
//! author and hashtag levels rank by likes (sum) and by likes per post
//! (average), and each author's or hashtag's posts rank by likes.

use super::index_only_e2e_tests::{
    assert_grovedb_is_consistent, assert_live_root_hash, delete_like, doctype_path, insert_like,
    platform_version, read_grove_element, sum_top_k,
};
use super::index_only_scalar_terminal_e2e_tests::equal;
use super::ranked_index_e2e_tests::avg_top_k;
use crate::config::{DriveConfig, DEFAULT_QUERY_LIMIT};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::query::drive_document_average_query::{
    AverageMode, DocumentAverageRequest, DocumentAverageResponse,
};
use crate::query::drive_document_count_query::{
    CountMode, DocumentCountRequest, DocumentCountResponse,
};
use crate::query::drive_document_having_query::mode_detection::detect_having_mode;
use crate::query::drive_document_having_query::{
    resolve_having_query_for_mode, DocumentHavingRequest, DocumentHavingResponse,
};
use crate::query::drive_document_ranked_query::index_picker::resolve_ranked_query_for_mode;
use crate::query::drive_document_ranked_query::mode_detection::detect_ranked_mode;
use crate::query::drive_document_ranked_query::{
    DocumentRankedRequest, DocumentRankedResponse, RankedEntryValue, RankedPaginationInputs,
    RANKED_COUNT_ORDER_KEY,
};
use crate::query::drive_document_sum_query::index_picker::{
    find_range_summable_index_for_where_clauses, find_summable_index_for_where_clauses,
};
use crate::query::drive_document_sum_query::{
    DocumentSumRequest, DocumentSumResponse, DriveDocumentSumQuery, SumMode,
};
use crate::query::having::{
    HavingAggregate, HavingAggregateFunction, HavingClause, HavingOperator, HavingRightOperand,
};
use crate::query::projection::SelectProjection;
use crate::query::{DriveDocumentCountQuery, OrderClause, WhereClause, WhereOperator};
use crate::util::batch::{DocumentOperationType, DriveOperation};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{
    DataContractInfo, DocumentAndContractInfo, DocumentTypeInfo, OwnedDocumentInfo,
};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_json_value;
use grovedb::element::indexed::compute_avg_fixed_point;
use grovedb::element::IndexAxis;
use grovedb::Element;
use std::cmp::Reverse;

const FIXTURE: &str =
    "tests/supporting_files/contract/yappr-likes/yappr-likes-summable-off-count-index-contract.json";

const AUTHOR_A: [u8; 32] = [0xAA; 32];
const AUTHOR_B: [u8; 32] = [0xBB; 32];
const LIKER_1: [u8; 32] = [0x11; 32];
const LIKER_2: [u8; 32] = [0x22; 32];
const LIKER_3: [u8; 32] = [0x33; 32];
const LIKER_4: [u8; 32] = [0x44; 32];
const LIKER_5: [u8; 32] = [0x55; 32];

/// The fixture, preallocated as written or with the counters created by the
/// first like of a post and removed with its last.
fn setup(preallocated: bool) -> (Drive, DataContract) {
    setup_with(|index| {
        if !preallocated {
            index.remove("preallocated");
        }
    })
}

/// The fixture with every like index edited by `edit`.
fn setup_with(
    edit: impl Fn(&mut serde_json::Map<String, serde_json::Value>),
) -> (Drive, DataContract) {
    setup_with_indices(|indices| {
        for index in indices.iter_mut() {
            edit(index.as_object_mut().expect("an index"));
        }
    })
}

/// The fixture with the like type's index list edited by `edit`.
fn setup_with_indices(edit: impl Fn(&mut Vec<serde_json::Value>)) -> (Drive, DataContract) {
    setup_with_schema(|schema| {
        edit(
            schema["documentSchemas"]["like"]["indices"]
                .as_array_mut()
                .expect("like indices"),
        )
    })
}

/// The fixture with its contract serialization format edited by `edit`.
fn setup_with_schema(edit: impl FnOnce(&mut serde_json::Value)) -> (Drive, DataContract) {
    let pv = platform_version();
    let mut schema = json_document_to_json_value(FIXTURE).expect("read contract fixture");
    edit(&mut schema);
    let contract = DataContract::try_from_platform_versioned(
        serde_json::from_value(schema).expect("contract serialization format"),
        true,
        &mut vec![],
        pv,
    )
    .expect("parse the like contract");
    let drive = setup_drive_with_initial_state_structure(None);
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("apply the contract");
    (drive, contract)
}

/// A post by `author` under `hashtag`, inserted.
fn insert_post(
    drive: &Drive,
    contract: &DataContract,
    author: [u8; 32],
    hashtag: &str,
    seed: u64,
) -> [u8; 32] {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("post")
        .expect("post doctype exists");
    let post = build_post(contract, author, hashtag, seed);
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((&post, None)),
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
        .expect("insert the post");
    post.id().to_buffer()
}

/// A post by `author` under `hashtag`.
fn build_post(contract: &DataContract, author: [u8; 32], hashtag: &str, seed: u64) -> Document {
    let mut post = contract
        .document_type_for_name("post")
        .expect("post doctype exists")
        .random_document(Some(seed), platform_version())
        .expect("random post");
    post.set_properties(
        [("hashtag".to_string(), Value::Text(hashtag.to_string()))]
            .into_iter()
            .collect(),
    );
    post.set_owner_id(Identifier::from(author));
    post
}

/// A like by `liker` on `post`, carrying the post's author and hashtag as
/// its reference's `where` requires.
fn build_like(
    contract: &DataContract,
    post: [u8; 32],
    author: [u8; 32],
    hashtag: &str,
    liker: [u8; 32],
    seed: u64,
) -> Document {
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let mut like = document_type
        .random_document(Some(seed), platform_version())
        .expect("random like");
    like.set_properties(
        [
            ("postId".to_string(), Value::Identifier(post)),
            ("postAuthor".to_string(), Value::Identifier(author)),
            ("hashtag".to_string(), Value::Text(hashtag.to_string())),
        ]
        .into_iter()
        .collect(),
    );
    like.set_owner_id(Identifier::from(liker));
    like
}

fn like_post(
    drive: &Drive,
    contract: &DataContract,
    post: [u8; 32],
    author: [u8; 32],
    hashtag: &str,
    likers: &[[u8; 32]],
) -> Vec<Document> {
    likers
        .iter()
        .enumerate()
        .map(|(position, liker)| {
            let like = build_like(contract, post, author, hashtag, *liker, position as u64);
            insert_like(drive, contract, &like, true).expect("insert the like");
            like
        })
        .collect()
}

fn level(contract: &DataContract, segments: &[&[u8]]) -> Vec<Vec<u8>> {
    let mut path = doctype_path(contract);
    path.extend(segments.iter().map(|segment| segment.to_vec()));
    path
}

/// Two posts by author A (3 likes and 1 like) and one by author B (5
/// likes), all under one hashtag.
fn liked_posts(
    drive: &Drive,
    contract: &DataContract,
) -> ([u8; 32], [u8; 32], [u8; 32], Vec<Document>) {
    let a1 = insert_post(drive, contract, AUTHOR_A, "dash", 1);
    let a2 = insert_post(drive, contract, AUTHOR_A, "dash", 2);
    let b1 = insert_post(drive, contract, AUTHOR_B, "dash", 3);
    let mut likes = like_post(
        drive,
        contract,
        a1,
        AUTHOR_A,
        "dash",
        &[LIKER_1, LIKER_2, LIKER_3],
    );
    likes.extend(like_post(drive, contract, a2, AUTHOR_A, "dash", &[LIKER_1]));
    likes.extend(like_post(
        drive,
        contract,
        b1,
        AUTHOR_B,
        "dash",
        &[LIKER_1, LIKER_2, LIKER_3, LIKER_4, LIKER_5],
    ));
    (a1, a2, b1, likes)
}

/// The ranked page and bound read limit used by the proved reads below.
const PAGE: RankedPaginationInputs = RankedPaginationInputs {
    limit: Some(10),
    offset: None,
    has_start_at: false,
};

/// A ranked read over likes, its proof verified as the SDK verifies it
/// (through the shared resolver, against the live root): the unproven page,
/// which the verified proof reproduces.
fn proved_ranked_page(
    drive: &Drive,
    contract: &DataContract,
    select: &SelectProjection,
    group_by: &[String],
    order_by: &[OrderClause],
    where_clauses: &[WhereClause],
) -> Vec<(Vec<u8>, RankedEntryValue)> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let request = |prove: bool| DocumentRankedRequest {
        contract,
        document_type,
        group_by,
        select: select.clone(),
        having: &[],
        order_by,
        where_clauses,
        limit: PAGE.limit,
        offset: PAGE.offset,
        has_start_at: PAGE.has_start_at,
        prove,
        resolved_time_ranges: &[],
    };
    let page = match drive
        .execute_document_ranked_request(request(false), None, pv)
        .expect("the ranked read succeeds")
    {
        DocumentRankedResponse::Entries(page) => page,
        DocumentRankedResponse::Proof(_) => panic!("expected entries"),
    };
    let proof = match drive
        .execute_document_ranked_request(request(true), None, pv)
        .expect("the ranked prove succeeds")
    {
        DocumentRankedResponse::Proof(proof) => proof,
        DocumentRankedResponse::Entries(_) => panic!("expected a proof"),
    };
    let mode = detect_ranked_mode(select, group_by, &[], order_by, where_clauses, PAGE, pv)
        .expect("the ranked request is well formed");
    let (root_hash, verified) = resolve_ranked_query_for_mode(
        contract.id_ref().to_buffer(),
        document_type,
        "like".to_string(),
        document_type.indexes(),
        &mode,
        &[],
        pv,
    )
    .expect("a ranking answers")
    .verify_ranked_top_k_proof(&proof, pv)
    .expect("the ranked proof verifies");
    assert_live_root_hash(drive, root_hash);
    assert_eq!(verified.entries, page.entries, "{select:?}");
    page.entries
        .iter()
        .map(|entry| (entry.key.clone(), entry.value))
        .collect()
}

/// A having-range `count(*)` read over likes, proved and verified like
/// [`proved_ranked_page`].
fn proved_having_count_entries(
    drive: &Drive,
    contract: &DataContract,
    group_by: &[String],
    having: &[HavingClause],
    where_clauses: &[WhereClause],
) -> Vec<(Vec<u8>, RankedEntryValue)> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let request = |prove: bool| DocumentHavingRequest {
        contract,
        document_type,
        group_by,
        select: SelectProjection::count_star(),
        having,
        order_by: &[],
        where_clauses,
        resolved_time_ranges: &[],
        limit: PAGE.limit,
        offset: PAGE.offset,
        has_start_at: PAGE.has_start_at,
        prove,
    };
    let entries = match drive
        .execute_document_having_request(request(false), None, pv)
        .expect("the bounded count succeeds")
    {
        DocumentHavingResponse::Entries(entries) => entries,
        DocumentHavingResponse::Proof(_) => panic!("expected entries"),
    };
    let proof = match drive
        .execute_document_having_request(request(true), None, pv)
        .expect("the bounded count proves")
    {
        DocumentHavingResponse::Proof(proof) => proof,
        DocumentHavingResponse::Entries(_) => panic!("expected a proof"),
    };
    let mode = detect_having_mode(
        &SelectProjection::count_star(),
        group_by,
        having,
        &[],
        where_clauses,
        PAGE,
        pv,
    )
    .expect("the bounded count is well formed");
    let (root_hash, verified) = resolve_having_query_for_mode(
        contract.id_ref().to_buffer(),
        document_type,
        "like".to_string(),
        document_type.indexes(),
        &mode,
        &[],
        pv,
    )
    .expect("a ranking bounds the count")
    .verify_having_range_proof(&proof, pv)
    .expect("the bounded count proof verifies");
    assert_live_root_hash(drive, root_hash);
    assert_eq!(verified, entries);
    entries
        .iter()
        .map(|entry| (entry.key.clone(), entry.value))
        .collect()
}

/// A `count(*)` request over likes, executed.
fn count_likes(
    drive: &Drive,
    contract: &DataContract,
    where_clauses: Vec<WhereClause>,
    mode: CountMode,
    limit: Option<u32>,
    prove: bool,
) -> Result<DocumentCountResponse, Error> {
    let drive_config = DriveConfig::default();
    drive.execute_document_count_request(
        DocumentCountRequest {
            contract,
            document_type: contract
                .document_type_for_name("like")
                .expect("like doctype exists"),
            where_clauses,
            resolved_time_ranges: vec![],
            order_clauses: vec![],
            mode,
            limit,
            prove,
            drive_config: &drive_config,
        },
        None,
        platform_version(),
    )
}

/// The count query a verifier rebuilds for a like count over
/// `where_clauses`: through the range picker for a range, the point picker
/// otherwise.
fn like_count_query<'a>(
    contract: &'a DataContract,
    where_clauses: &[WhereClause],
) -> DriveDocumentCountQuery<'a> {
    let document_type = contract
        .document_types()
        .get("like")
        .expect("like doctype exists");
    let has_range = where_clauses
        .iter()
        .any(|clause| DriveDocumentCountQuery::is_range_operator(clause.operator));
    let index = if has_range {
        DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            where_clauses,
            &[],
        )
    } else {
        DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            where_clauses,
            &[],
        )
    }
    .expect("an index answers the count");
    DriveDocumentCountQuery {
        document_type: document_type.as_ref(),
        contract_id: contract.id().to_buffer(),
        document_type_name: "like".to_string(),
        index,
        where_clauses: where_clauses.to_vec(),
    }
}

/// A `sum(byPost)` request over likes, executed.
fn sum_likes(
    drive: &Drive,
    contract: &DataContract,
    where_clauses: Vec<WhereClause>,
    mode: SumMode,
    limit: Option<u32>,
    prove: bool,
) -> Result<DocumentSumResponse, Error> {
    let drive_config = DriveConfig::default();
    drive.execute_document_sum_request(
        DocumentSumRequest {
            contract,
            document_type: contract
                .document_type_for_name("like")
                .expect("like doctype exists"),
            sum_property: "byPost".to_string(),
            where_clauses,
            resolved_time_ranges: vec![],
            order_clauses: vec![],
            mode,
            limit,
            prove,
            drive_config: &drive_config,
        },
        None,
        platform_version(),
    )
}

/// The sum query a verifier rebuilds for a `sum(byPost)` over
/// `where_clauses`: through the range picker for a range, the point picker
/// otherwise.
fn like_sum_query<'a>(
    contract: &'a DataContract,
    where_clauses: &[WhereClause],
) -> DriveDocumentSumQuery<'a> {
    let document_type = contract
        .document_types()
        .get("like")
        .expect("like doctype exists");
    let has_range = where_clauses
        .iter()
        .any(|clause| DriveDocumentCountQuery::is_range_operator(clause.operator));
    let index = if has_range {
        find_range_summable_index_for_where_clauses(
            document_type.indexes(),
            where_clauses,
            "byPost",
            &[],
        )
    } else {
        find_summable_index_for_where_clauses(document_type.indexes(), where_clauses, "byPost", &[])
    }
    .expect("a counter index answers the sum");
    DriveDocumentSumQuery {
        document_type: document_type.as_ref(),
        contract_id: contract.id().to_buffer(),
        document_type_name: "like".to_string(),
        index,
        where_clauses: where_clauses.to_vec(),
        sum_property: "byPost".to_string(),
    }
}

/// `postId <operator> post`.
fn post_id(operator: WhereOperator, post: [u8; 32]) -> WhereClause {
    WhereClause {
        field: "postId".to_string(),
        operator,
        value: Value::Identifier(post),
    }
}

/// `postId > 0`: every post.
fn every_post() -> WhereClause {
    post_id(WhereOperator::GreaterThan, [0; 32])
}

/// `postAuthor IN [A, B]`.
fn both_authors() -> WhereClause {
    WhereClause {
        field: "postAuthor".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![
            Value::Identifier(AUTHOR_A),
            Value::Identifier(AUTHOR_B),
        ]),
    }
}

/// `count(*) > threshold`.
fn count_above(threshold: u64) -> Vec<HavingClause> {
    vec![HavingClause {
        aggregate: HavingAggregate {
            function: HavingAggregateFunction::Count,
            field: String::new(),
        },
        operator: HavingOperator::GreaterThan,
        right: HavingRightOperand::Value(Value::U64(threshold)),
    }]
}

/// A document count over a counter index reads its sums, which never exceed
/// `i64::MAX`: a `HAVING count(*)` lower bound above that matches no group and
/// is refused, while `>= i64::MAX` is still a bound the sums can meet. An
/// ordinary Count axis (`byPost`, ranking posts by their entries) keeps the
/// higher bound.
#[test]
fn should_refuse_a_count_bound_above_what_the_counters_sums_reach() {
    let (_drive, contract) = setup(true);
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let resolve = |group_by: &str, operator: HavingOperator| {
        let group_by = vec![group_by.to_string()];
        let having = vec![HavingClause {
            aggregate: HavingAggregate {
                function: HavingAggregateFunction::Count,
                field: String::new(),
            },
            operator,
            right: HavingRightOperand::Value(Value::U64(i64::MAX as u64)),
        }];
        let mode = detect_having_mode(
            &SelectProjection::count_star(),
            &group_by,
            &having,
            &[],
            &[],
            PAGE,
            pv,
        )
        .expect("the bounded count is well formed");
        resolve_having_query_for_mode(
            contract.id_ref().to_buffer(),
            document_type,
            "like".to_string(),
            document_type.indexes(),
            &mode,
            &[],
            pv,
        )
        .map(|query| query.index.name.clone())
    };

    let refused = resolve("postAuthor", HavingOperator::GreaterThan);
    assert!(
        matches!(
            refused,
            Err(Error::Query(QuerySyntaxError::InvalidParameter(_)))
        ),
        "got {refused:?}"
    );
    assert_eq!(
        resolve("postAuthor", HavingOperator::GreaterThanOrEquals)
            .expect("i64::MAX is a bound the sums can meet"),
        "byAuthorPost"
    );
    assert_eq!(
        resolve("postId", HavingOperator::GreaterThan)
            .expect("an ordinary Count axis keeps the higher bound"),
        "byPost"
    );
}

/// The ranked tree of each author or hashtag with its axes: the levels rank
/// by likes and likes per post, the counter levels by likes.
#[test]
fn should_register_the_author_and_hashtag_levels_as_sum_and_average_rankings() {
    let (drive, contract) = setup(true);
    for first in [b"postAuthor".as_slice(), b"hashtag"] {
        let element = read_grove_element(&drive, &doctype_path(&contract), first)
            .expect("the first level is created with the contract");
        let Element::ProvableCountProvableSumIndexedTree(.., axes, _) = &element else {
            panic!("expected a count-and-sum indexed tree, got {element:?}");
        };
        let tags: Vec<u8> = axes.iter().map(|(tag, _)| *tag).collect();
        assert_eq!(
            tags,
            vec![IndexAxis::Sum.tag(), IndexAxis::Avg.tag()],
            "{}",
            String::from_utf8_lossy(first)
        );
    }
}

/// A post creates, under its author and its hashtag, one counter at zero:
/// the post counts as a group with no likes, and no entry tree is built.
#[test]
fn should_preallocate_one_zero_counter_per_post() {
    let (drive, contract) = setup(true);
    let post = insert_post(&drive, &contract, AUTHOR_A, "dash", 1);

    for (first, value) in [
        (b"postAuthor".as_slice(), AUTHOR_A.as_slice()),
        (b"hashtag", b"dash"),
    ] {
        let group = read_grove_element(&drive, &level(&contract, &[first]), value)
            .expect("the group's value tree");
        assert!(matches!(group, Element::CountSumTree(..)), "got {group:?}");
        assert_eq!(group.count_sum_value_or_default(), (1, 0));
        let counter =
            read_grove_element(&drive, &level(&contract, &[first, value, b"postId"]), &post)
                .expect("the post's counter");
        assert!(matches!(counter, Element::SumItem(0, _)), "got {counter:?}");
    }
    assert_grovedb_is_consistent(&drive);
}

/// Two preallocated counter indexes sharing their first property reach one
/// value tree from a post: it is queued once, so the post's batch passes
/// grovedb's consistency check (on in these tests), and each index gets its
/// zero counter.
#[test]
fn should_preallocate_a_value_tree_two_counter_indexes_share_once() {
    let (drive, contract) = setup_with_indices(|indices| {
        for index in indices.iter_mut() {
            let index = index.as_object_mut().expect("an index");
            index.remove("rankedSummable");
            index.remove("rankedAverageable");
        }
        indices.push(serde_json::json!({
            "name": "byAuthorHashtagPost",
            "properties": [{ "postAuthor": "asc" }, { "hashtag": "asc" }, { "postId": "asc" }],
            "summableOffCountIndex": "byPost",
            "rangeCountable": true,
            "rangeSummable": true,
            "skipIfAbsent": true,
            "preallocated": true,
        }));
    });
    let post = insert_post(&drive, &contract, AUTHOR_A, "dash", 1);

    for segments in [
        [b"postAuthor".as_slice(), AUTHOR_A.as_slice(), b"postId"].as_slice(),
        &[
            b"postAuthor",
            AUTHOR_A.as_slice(),
            b"hashtag",
            b"dash",
            b"postId",
        ],
    ] {
        let counter = read_grove_element(&drive, &level(&contract, segments), &post)
            .expect("the post's counter");
        assert!(matches!(counter, Element::SumItem(0, _)), "got {counter:?}");
    }
    assert_grovedb_is_consistent(&drive);
}

/// Two bindings of one counter index resolving the same counter: `postA` and
/// `postB` each refer to a post whose `$id` the other holds, so a post binds
/// the index `[postB, postA]` through either reference, at `[post, post]`. Its
/// zero counter is queued once, so the post's batch passes grovedb's
/// consistency check (on in these tests).
#[test]
fn should_preallocate_a_counter_two_bindings_of_one_index_resolve_once() {
    let post_reference = |other: &str, position: u64| {
        serde_json::json!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": position,
            "refersTo": {
                "type": "permanentDocument",
                "documentType": "post",
                "where": { "$id": other },
            },
        })
    };
    let (drive, contract) = setup_with_schema(|schema| {
        schema["documentSchemas"]["like"] = serde_json::json!({
            "type": "object",
            "indexOnly": true,
            "documentsMutable": false,
            "canBeDeleted": true,
            "properties": {
                "postA": post_reference("postB", 0),
                "postB": post_reference("postA", 1),
            },
            "indices": [
                {
                    "name": "byPair",
                    "properties": [{ "postA": "asc" }, { "postB": "asc" }],
                    "terminal": "$ownerId",
                },
                {
                    "name": "byPairCounter",
                    "properties": [{ "postB": "asc" }, { "postA": "asc" }],
                    "summableOffCountIndex": "byPair",
                    "rangeSummable": true,
                    "preallocated": true,
                },
            ],
            "required": ["postA", "postB"],
            "additionalProperties": false,
        });
    });

    let post = insert_post(&drive, &contract, AUTHOR_A, "dash", 1);
    let counter = read_grove_element(
        &drive,
        &level(&contract, &[b"postB", post.as_slice(), b"postA"]),
        &post,
    )
    .expect("the post's counter");
    assert!(matches!(counter, Element::SumItem(0, _)), "got {counter:?}");
    assert_grovedb_is_consistent(&drive);
}

/// Each like moves its post's counters by one, which the author and hashtag
/// levels add up: author A has 2 posts and 4 likes, author B 1 post and 5.
#[test]
fn should_count_each_post_once_and_sum_its_likes() {
    for preallocated in [true, false] {
        let (drive, contract) = setup(preallocated);
        let (a1, a2, b1, _) = liked_posts(&drive, &contract);

        let authors = level(&contract, &[b"postAuthor"]);
        let author_a = read_grove_element(&drive, &authors, &AUTHOR_A).expect("author A");
        assert_eq!(author_a.count_sum_value_or_default(), (2, 4));
        let author_b = read_grove_element(&drive, &authors, &AUTHOR_B).expect("author B");
        assert_eq!(author_b.count_sum_value_or_default(), (1, 5));
        let hashtag = read_grove_element(&drive, &level(&contract, &[b"hashtag"]), b"dash")
            .expect("the hashtag");
        assert_eq!(hashtag.count_sum_value_or_default(), (3, 9));

        let a_posts = level(&contract, &[b"postAuthor", &AUTHOR_A, b"postId"]);
        for (post, likes) in [(a1, 3), (a2, 1)] {
            let counter = read_grove_element(&drive, &a_posts, &post).expect("the counter");
            assert!(
                matches!(counter, Element::SumItem(n, _) if n == likes),
                "got {counter:?}"
            );
        }
        let b_counter = read_grove_element(
            &drive,
            &level(&contract, &[b"postAuthor", &AUTHOR_B, b"postId"]),
            &b1,
        )
        .expect("B's counter");
        assert!(
            matches!(b_counter, Element::SumItem(5, _)),
            "got {b_counter:?}"
        );

        // Most likes: B (5) over A (4); most likes per post: B (5) over A (2)
        assert_eq!(
            sum_top_k(&drive, &authors, 10, true),
            vec![(5, AUTHOR_B.to_vec()), (4, AUTHOR_A.to_vec())]
        );
        assert_eq!(
            avg_top_k(&drive, &authors, 10, true),
            vec![
                (compute_avg_fixed_point(5, 1), AUTHOR_B.to_vec()),
                (compute_avg_fixed_point(4, 2), AUTHOR_A.to_vec()),
            ]
        );
        // A's posts by likes
        assert_eq!(
            sum_top_k(&drive, &a_posts, 10, true),
            vec![(3, a1.to_vec()), (1, a2.to_vec())]
        );
        assert_grovedb_is_consistent(&drive);
    }
}

/// An unlike takes the like back; the last unlike of a post leaves a
/// preallocated counter at zero and removes any other with the trees it
/// leaves empty. A re-like moves it again.
#[test]
fn should_take_a_like_back_on_delete() {
    for preallocated in [true, false] {
        let (drive, contract) = setup(preallocated);
        let post = insert_post(&drive, &contract, AUTHOR_A, "dash", 1);
        let likes = like_post(
            &drive,
            &contract,
            post,
            AUTHOR_A,
            "dash",
            &[LIKER_1, LIKER_2],
        );
        let authors = level(&contract, &[b"postAuthor"]);
        let a_posts = level(&contract, &[b"postAuthor", &AUTHOR_A, b"postId"]);

        delete_like(&drive, &contract, likes[0].clone(), true).expect("unlike once");
        let counter = read_grove_element(&drive, &a_posts, &post).expect("the counter");
        assert!(matches!(counter, Element::SumItem(1, _)), "got {counter:?}");

        delete_like(&drive, &contract, likes[1].clone(), true).expect("unlike again");
        let author = read_grove_element(&drive, &authors, &AUTHOR_A);
        if preallocated {
            let counter = read_grove_element(&drive, &a_posts, &post).expect("the counter stays");
            assert!(matches!(counter, Element::SumItem(0, _)), "got {counter:?}");
            assert_eq!(
                author
                    .expect("the author stays")
                    .count_sum_value_or_default(),
                (1, 0)
            );
        } else {
            assert!(
                author.is_none(),
                "the drained author is pruned, got {author:?}"
            );
        }

        let relike = build_like(&contract, post, AUTHOR_A, "dash", LIKER_3, 7);
        insert_like(&drive, &contract, &relike, true).expect("re-like");
        let counter = read_grove_element(&drive, &a_posts, &post).expect("the counter");
        assert!(matches!(counter, Element::SumItem(1, _)), "got {counter:?}");
        assert_eq!(
            sum_top_k(&drive, &authors, 10, true),
            vec![(1, AUTHOR_A.to_vec())]
        );
        assert_grovedb_is_consistent(&drive);
    }
}

/// A dry run of a like prices at least what applying it charges, in storage
/// and in processing, for the post's first like and for a later one that
/// only rewrites the counter, and leaves the counter as it was.
#[test]
fn should_upper_bound_a_like_with_its_dry_run() {
    for preallocated in [true, false] {
        let (drive, contract) = setup(preallocated);
        let post = insert_post(&drive, &contract, AUTHOR_A, "dash", 1);
        let a_posts = level(&contract, &[b"postAuthor", &AUTHOR_A, b"postId"]);
        for (seed, liker) in [LIKER_1, LIKER_2].into_iter().enumerate() {
            let like = build_like(&contract, post, AUTHOR_A, "dash", liker, seed as u64);
            let before = read_grove_element(&drive, &a_posts, &post);
            let estimated = insert_like(&drive, &contract, &like, false).expect("dry run");
            assert_eq!(
                read_grove_element(&drive, &a_posts, &post),
                before,
                "a dry run moves no counter"
            );
            let applied = insert_like(&drive, &contract, &like, true).expect("apply");
            assert!(
                estimated.storage_fee >= applied.storage_fee
                    && estimated.processing_fee >= applied.processing_fee,
                "estimated {estimated:?} below applied {applied:?} (preallocated: \
                 {preallocated}, like {seed})"
            );
        }
    }
}

/// A dry run of an unlike prices at least what applying it charges, also
/// when the post keeps other likes and the counter is rewritten rather than
/// removed.
#[test]
fn should_upper_bound_an_unlike_with_its_dry_run() {
    for preallocated in [true, false] {
        for liker_count in [1, 2] {
            let (drive, contract) = setup(preallocated);
            let post = insert_post(&drive, &contract, AUTHOR_A, "dash", 1);
            let likes = like_post(
                &drive,
                &contract,
                post,
                AUTHOR_A,
                "dash",
                &[LIKER_1, LIKER_2][..liker_count],
            );
            let a_posts = level(&contract, &[b"postAuthor", &AUTHOR_A, b"postId"]);
            let before = read_grove_element(&drive, &a_posts, &post);
            let estimated =
                delete_like(&drive, &contract, likes[0].clone(), false).expect("dry run");
            assert_eq!(
                read_grove_element(&drive, &a_posts, &post),
                before,
                "a dry run moves no counter"
            );
            let applied = delete_like(&drive, &contract, likes[0].clone(), true).expect("apply");
            assert!(
                estimated.processing_fee >= applied.processing_fee,
                "estimated {} below applied {} (preallocated: {preallocated}, likes: {liker_count})",
                estimated.processing_fee,
                applied.processing_fee
            );
        }
    }
}

/// A Drive operation adding `like`.
fn add_like<'a>(contract: &'a DataContract, like: &'a Document) -> DriveOperation<'a> {
    add_document(contract, "like", like)
}

/// A Drive operation adding `document` of the type named `document_type`.
fn add_document<'a>(
    contract: &'a DataContract,
    document_type: &'static str,
    document: &'a Document,
) -> DriveOperation<'a> {
    DriveOperation::DocumentOperation(DocumentOperationType::AddDocument {
        owned_document_info: OwnedDocumentInfo {
            document_info: DocumentRefInfo((document, None)),
            owner_id: None,
        },
        contract_info: DataContractInfo::BorrowedDataContract(contract),
        document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr(document_type),
        override_document: false,
    })
}

/// A Drive batch writing two likes is refused, applied, dry-run or converted:
/// each like's conversion reads the counter the other moves, so the counter
/// would move once. So is a batch writing a post and a like of it, in either
/// order: the post's preallocation writes the counter the like moves. Only an
/// insert preallocates, so deleting posts moves no counter. One like a batch
/// goes through.
#[test]
fn should_refuse_a_batch_writing_two_likes_of_a_type_keeping_counters() {
    let (drive, contract) = setup(true);
    let post = insert_post(&drive, &contract, AUTHOR_A, "dash", 1);
    let likes = [
        build_like(&contract, post, AUTHOR_A, "dash", LIKER_1, 1),
        build_like(&contract, post, AUTHOR_A, "dash", LIKER_2, 2),
    ];
    let add = |like| add_like(&contract, like);
    let pv = platform_version();
    let refused = |result: Result<(), Error>| {
        matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
        )
    };
    for apply in [false, true] {
        let result = drive
            .apply_drive_operations(
                likes.iter().map(add).collect(),
                apply,
                &BlockInfo::default(),
                None,
                pv,
                None,
            )
            .map(|_| ());
        assert!(refused(result), "apply: {apply}");
    }
    let result = drive
        .convert_drive_operations_to_grove_operations(
            likes.iter().map(add).collect(),
            &BlockInfo::default(),
            None,
            pv,
        )
        .map(|_| ());
    assert!(refused(result), "converted");

    let new_post = build_post(&contract, AUTHOR_B, "dash", 9);
    let like_of_new_post = build_like(
        &contract,
        new_post.id().to_buffer(),
        AUTHOR_B,
        "dash",
        LIKER_3,
        9,
    );
    for post_first in [true, false] {
        let mut operations = vec![
            add_document(&contract, "post", &new_post),
            add_like(&contract, &like_of_new_post),
        ];
        if !post_first {
            operations.reverse();
        }
        let result = drive
            .apply_drive_operations(operations, true, &BlockInfo::default(), None, pv, None)
            .map(|_| ());
        assert!(
            refused(result),
            "a post and its like (post first: {post_first})"
        );
    }
    let delete_post = |post: [u8; 32]| {
        DriveOperation::DocumentOperation(DocumentOperationType::ForceDeleteDocument {
            document_id: Identifier::new(post),
            contract_info: DataContractInfo::BorrowedDataContract(&contract),
            document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("post"),
        })
    };
    let other_post = insert_post(&drive, &contract, AUTHOR_B, "dash", 2);
    for operations in [
        vec![delete_post(post), delete_post(other_post)],
        vec![
            delete_post(post),
            add_document(&contract, "post", &new_post),
        ],
    ] {
        drive
            .refuse_repeated_counter_moves(&operations, &BlockInfo::default(), None, pv)
            .expect("a post delete moves no counter");
    }

    drive
        .apply_drive_operations(
            vec![add(&likes[0])],
            true,
            &BlockInfo::default(),
            None,
            pv,
            None,
        )
        .expect("one like a batch applies");
    let a_posts = level(&contract, &[b"postAuthor", &AUTHOR_A, b"postId"]);
    let counter = read_grove_element(&drive, &a_posts, &post).expect("the counter");
    assert!(matches!(counter, Element::SumItem(1, _)), "got {counter:?}");
}

/// Both generations of the batch conversion through its dispatcher: v1
/// (protocol version 14) refuses a batch writing two likes, while v0, every
/// earlier version's, converts it as it always has.
#[test]
fn should_refuse_two_likes_only_from_the_convert_generation_that_checks_counter_moves() {
    let (drive, contract) = setup(true);
    let post = insert_post(&drive, &contract, AUTHOR_A, "dash", 1);
    let likes = [
        build_like(&contract, post, AUTHOR_A, "dash", LIKER_1, 1),
        build_like(&contract, post, AUTHOR_A, "dash", LIKER_2, 2),
    ];
    let operations = || {
        likes
            .iter()
            .map(|like| add_like(&contract, like))
            .collect::<Vec<_>>()
    };
    let latest = platform_version();
    let refused = drive.convert_drive_operations_to_grove_operations(
        operations(),
        &BlockInfo::default(),
        None,
        latest,
    );
    assert!(
        matches!(
            refused,
            Err(Error::Drive(DriveError::CorruptedCodeExecution(_)))
        ),
        "v1 refuses the batch"
    );
    let mut first_generation = latest.clone();
    first_generation
        .drive
        .methods
        .batch_operations
        .convert_drive_operations_to_grove_operations = 0;
    let converted = drive
        .convert_drive_operations_to_grove_operations(
            operations(),
            &BlockInfo::default(),
            None,
            &first_generation,
        )
        .expect("v0 converts the batch unrefused");
    // Each like's conversion writes both of its post's counters: two writes
    // per counter, which would apply as one, so v1 refuses the batch
    let counter_writes = |segments: &[&[u8]]| {
        let counter_path = level(&contract, segments);
        converted
            .operations
            .iter()
            .filter(|operation| {
                operation.path == counter_path
                    && operation
                        .key
                        .as_ref()
                        .is_some_and(|key| *key == post.to_vec())
            })
            .count()
    };
    assert_eq!(
        counter_writes(&[b"postAuthor", &AUTHOR_A, b"postId"]),
        2,
        "v0 writes the author counter once per like"
    );
    assert_eq!(
        counter_writes(&[b"hashtag", b"dash", b"postId"]),
        2,
        "v0 writes the hashtag counter once per like"
    );
}

/// The source index's name addresses the summed likes: one post's, one
/// author's (read off the author's value tree, which sums its posts), with
/// a proof that verifies to the same total.
#[test]
fn should_sum_likes_by_the_source_index_name() {
    let (drive, contract) = setup(true);
    let (a1, _, _, _) = liked_posts(&drive, &contract);

    for (clauses, total) in [
        (
            vec![
                equal("postAuthor", Value::Identifier(AUTHOR_A)),
                equal("postId", Value::Identifier(a1)),
            ],
            3,
        ),
        (vec![equal("postAuthor", Value::Identifier(AUTHOR_A))], 4),
        (vec![equal("hashtag", Value::Text("dash".to_string()))], 9),
    ] {
        match sum_likes(
            &drive,
            &contract,
            clauses.clone(),
            SumMode::Aggregate,
            None,
            false,
        )
        .expect("the sum must execute")
        {
            DocumentSumResponse::Aggregate(sum) => assert_eq!(sum, total, "{clauses:?}"),
            other => panic!("expected an aggregate sum, got {other:?}"),
        }
        let proof = match sum_likes(
            &drive,
            &contract,
            clauses.clone(),
            SumMode::Aggregate,
            None,
            true,
        )
        .expect("the proved sum must execute")
        {
            DocumentSumResponse::Proof(proof) => proof,
            other => panic!("expected a proof, got {other:?}"),
        };
        let query = like_sum_query(&contract, &clauses);
        assert!(query.index.is_summable_off_count_index());
        let (root_hash, entries) = query
            .verify_point_lookup_sum_proof(&proof, platform_version())
            .expect("the sum proof verifies");
        assert_live_root_hash(&drive, root_hash);
        let proved: i64 = entries.iter().filter_map(|entry| entry.sum).sum();
        assert_eq!(proved, total, "{clauses:?}");
    }
}

/// A count query counts documents, and a counter index answers it from its
/// sums: an author's or a hashtag's likes, one post's likes, zero for a
/// preallocated post nobody liked, proved like any count. The average reads
/// the posts and the likes together.
#[test]
fn should_count_likes_from_the_counters_sums_and_read_posts_through_the_average() {
    let (drive, contract) = setup(true);
    let (a1, a2, _, _) = liked_posts(&drive, &contract);
    let unliked = insert_post(&drive, &contract, AUTHOR_B, "dash", 4);
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let drive_config = DriveConfig::default();
    let count = |where_clauses: Vec<WhereClause>, prove: bool| {
        count_likes(
            &drive,
            &contract,
            where_clauses,
            CountMode::Aggregate,
            None,
            prove,
        )
    };

    for (clauses, likes) in [
        (vec![equal("postId", Value::Identifier(a1))], 3),
        (vec![equal("postAuthor", Value::Identifier(AUTHOR_A))], 4),
        (
            vec![
                equal("postAuthor", Value::Identifier(AUTHOR_A)),
                equal("postId", Value::Identifier(a1)),
            ],
            3,
        ),
        (vec![equal("hashtag", Value::Text("dash".to_string()))], 9),
        (
            vec![
                equal("hashtag", Value::Text("dash".to_string())),
                equal("postId", Value::Identifier(a2)),
            ],
            1,
        ),
        (
            vec![
                equal("postAuthor", Value::Identifier(AUTHOR_B)),
                equal("postId", Value::Identifier(unliked)),
            ],
            0,
        ),
    ] {
        match count(clauses.clone(), false).expect("the count executes") {
            DocumentCountResponse::Aggregate(counted) => {
                assert_eq!(counted, likes, "{clauses:?}")
            }
            other => panic!("expected an aggregate count, got {other:?}"),
        }
        let proof = match count(clauses.clone(), true).expect("the proved count executes") {
            DocumentCountResponse::Proof(proof) => proof,
            other => panic!("expected a proof, got {other:?}"),
        };
        let (root_hash, entries) = like_count_query(&contract, &clauses)
            .verify_point_lookup_count_proof(&proof, pv)
            .expect("the count proof verifies");
        assert_live_root_hash(&drive, root_hash);
        let proved: u64 = entries.iter().filter_map(|entry| entry.count).sum();
        assert_eq!(proved, likes, "proved {clauses:?}");
    }
    let clauses = vec![equal("postAuthor", Value::Identifier(AUTHOR_A))];

    match drive
        .execute_document_average_request(
            DocumentAverageRequest {
                contract: &contract,
                document_type,
                sum_property: "byPost".to_string(),
                where_clauses: clauses,
                resolved_time_ranges: vec![],
                order_clauses: vec![],
                mode: AverageMode::Aggregate,
                limit: None,
                prove: false,
                drive_config: &drive_config,
            },
            None,
            platform_version(),
        )
        .expect("the average must execute")
    {
        DocumentAverageResponse::Aggregate { count, sum } => {
            assert_eq!((count, sum), (2, 4), "A's posts and likes")
        }
        other => panic!("expected an aggregate average, got {other:?}"),
    }
}

/// A range count over a counter index reads its range sums, so it counts
/// likes: per post over a range of one author's posts (a preallocated post
/// nobody liked counted at zero, so a page of one ends only at its limit) and
/// per author and post across an `IN`, each proved.
#[test]
fn should_count_likes_over_a_range_of_posts_from_the_counters_sums() {
    let (drive, contract) = setup(true);
    let (a1, a2, b1, _) = liked_posts(&drive, &contract);
    let unliked = insert_post(&drive, &contract, AUTHOR_A, "dash", 4);
    let pv = platform_version();
    let mut a_posts = [(a1.to_vec(), 3), (a2.to_vec(), 1), (unliked.to_vec(), 0)];
    a_posts.sort();
    let mut both = [
        (AUTHOR_A.to_vec(), a1.to_vec(), 3),
        (AUTHOR_A.to_vec(), a2.to_vec(), 1),
        (AUTHOR_A.to_vec(), unliked.to_vec(), 0),
        (AUTHOR_B.to_vec(), b1.to_vec(), 5),
    ];
    both.sort();

    // A page of one at a time walks every post, the unliked one included.
    let mut paged = Vec::new();
    let mut after = [0u8; 32];
    loop {
        let page = match count_likes(
            &drive,
            &contract,
            vec![
                equal("postAuthor", Value::Identifier(AUTHOR_A)),
                post_id(WhereOperator::GreaterThan, after),
            ],
            CountMode::GroupByRange,
            Some(1),
            false,
        )
        .expect("a page of one executes")
        {
            DocumentCountResponse::Entries(entries) => entries,
            other => panic!("expected entries, got {other:?}"),
        };
        let Some(entry) = page.first() else {
            break;
        };
        assert_eq!(page.len(), 1);
        after = entry.key.clone().try_into().expect("a post id");
        paged.push((
            entry.key.clone(),
            entry
                .count
                .expect("a range walk counts each entry it returns"),
        ));
    }
    assert_eq!(paged, a_posts.to_vec(), "every page of one");

    for (clauses, mode, expected) in [
        (
            vec![
                equal("postAuthor", Value::Identifier(AUTHOR_A)),
                every_post(),
            ],
            CountMode::GroupByRange,
            a_posts
                .iter()
                .map(|(post, likes)| (None, post.clone(), *likes))
                .collect::<Vec<_>>(),
        ),
        (
            vec![both_authors(), every_post()],
            CountMode::GroupByCompound,
            both.iter()
                .map(|(author, post, likes)| (Some(author.clone()), post.clone(), *likes))
                .collect(),
        ),
    ] {
        let entries = match count_likes(&drive, &contract, clauses.clone(), mode, None, false)
            .expect("the range count executes")
        {
            DocumentCountResponse::Entries(entries) => entries,
            other => panic!("expected entries, got {other:?}"),
        };
        assert_eq!(
            entries
                .iter()
                .map(|entry| (
                    entry.in_key.clone(),
                    entry.key.clone(),
                    entry
                        .count
                        .expect("a range walk counts each entry it returns")
                ))
                .collect::<Vec<_>>(),
            expected,
            "{mode:?}"
        );
        let proof = match count_likes(&drive, &contract, clauses.clone(), mode, None, true)
            .expect("the range count proves")
        {
            DocumentCountResponse::Proof(proof) => proof,
            other => panic!("expected a proof, got {other:?}"),
        };
        let query = like_count_query(&contract, &clauses);
        assert!(query.index.is_summable_off_count_index());
        let (root_hash, verified) = query
            .verify_distinct_count_proof(&proof, DEFAULT_QUERY_LIMIT, true, pv)
            .expect("the range count proof verifies");
        assert_live_root_hash(&drive, root_hash);
        assert_eq!(verified, entries, "{mode:?}");
    }
}

/// A grouped `sum(byPost)` over a counter index keeps a preallocated post
/// nobody liked as a sum of zero, unproved and proved alike, so a page of
/// one starting at it returns it and paging one post at a time walks every
/// post: through one author and through `IN` both authors.
#[test]
fn should_keep_an_unliked_post_on_a_page_of_grouped_likes_sums() {
    let (drive, contract) = setup(true);
    let (a1, a2, _, _) = liked_posts(&drive, &contract);
    let unliked = insert_post(&drive, &contract, AUTHOR_A, "dash", 4);
    let unproved_page = |where_clauses: Vec<WhereClause>, mode: SumMode| match sum_likes(
        &drive,
        &contract,
        where_clauses,
        mode,
        Some(1),
        false,
    )
    .expect("a page of one executes")
    {
        DocumentSumResponse::Entries(entries) => entries,
        other => panic!("expected entries, got {other:?}"),
    };
    let proved_page = |where_clauses: Vec<WhereClause>, mode: SumMode| {
        let proof = match sum_likes(
            &drive,
            &contract,
            where_clauses.clone(),
            mode,
            Some(1),
            true,
        )
        .expect("a page of one proves")
        {
            DocumentSumResponse::Proof(proof) => proof,
            other => panic!("expected a proof, got {other:?}"),
        };
        let query = like_sum_query(&contract, &where_clauses);
        assert!(query.index.is_summable_off_count_index());
        let (root_hash, entries) = query
            .verify_distinct_sum_proof(&proof, 1, true, platform_version())
            .expect("the page's proof verifies");
        assert_live_root_hash(&drive, root_hash);
        entries
    };

    // A page of one starting at the unliked post returns it at zero.
    for (clauses, mode, in_key) in [
        (
            vec![
                equal("postAuthor", Value::Identifier(AUTHOR_A)),
                post_id(WhereOperator::GreaterThanOrEquals, unliked),
            ],
            SumMode::GroupByRange,
            None,
        ),
        (
            vec![
                both_authors(),
                post_id(WhereOperator::GreaterThanOrEquals, unliked),
            ],
            SumMode::GroupByCompound,
            Some(AUTHOR_A.to_vec()),
        ),
    ] {
        let unproved = unproved_page(clauses.clone(), mode);
        assert_eq!(
            unproved
                .iter()
                .map(|entry| (entry.in_key.clone(), entry.key.clone(), entry.sum))
                .collect::<Vec<_>>(),
            vec![(in_key, unliked.to_vec(), Some(0))],
            "{mode:?}"
        );
        assert_eq!(proved_page(clauses, mode), unproved, "proved {mode:?}");
    }

    // Paging one post at a time walks every one of A's posts.
    let mut expected = [(a1.to_vec(), 3), (a2.to_vec(), 1), (unliked.to_vec(), 0)];
    expected.sort();
    let mut paged = Vec::new();
    let mut after = [0u8; 32];
    loop {
        let clauses = vec![
            equal("postAuthor", Value::Identifier(AUTHOR_A)),
            post_id(WhereOperator::GreaterThan, after),
        ];
        let entries = unproved_page(clauses.clone(), SumMode::GroupByRange);
        assert_eq!(
            proved_page(clauses, SumMode::GroupByRange),
            entries,
            "proved page after {after:?}"
        );
        let Some(entry) = entries.first() else {
            break;
        };
        after = entry.key.clone().try_into().expect("a post id");
        paged.push((entry.key.clone(), entry.sum.unwrap_or_default()));
    }
    assert_eq!(paged, expected.to_vec(), "every page of one");
}

/// Over a counter index that ranks no level, a range count's total is the
/// range's likes: one total, and one per author across an `IN`, each proved.
#[test]
fn should_total_the_likes_of_a_range_of_posts_from_the_counters_sums() {
    let (drive, contract) = setup_with(|index| {
        index.remove("rankedSummable");
        index.remove("rankedAverageable");
    });
    liked_posts(&drive, &contract);
    let pv = platform_version();

    let a_range = vec![
        equal("postAuthor", Value::Identifier(AUTHOR_A)),
        every_post(),
    ];
    match count_likes(
        &drive,
        &contract,
        a_range.clone(),
        CountMode::Aggregate,
        None,
        false,
    )
    .expect("the range total executes")
    {
        DocumentCountResponse::Aggregate(total) => assert_eq!(total, 4, "A's likes"),
        other => panic!("expected an aggregate count, got {other:?}"),
    }
    let proof = match count_likes(
        &drive,
        &contract,
        a_range.clone(),
        CountMode::Aggregate,
        None,
        true,
    )
    .expect("the range total proves")
    {
        DocumentCountResponse::Proof(proof) => proof,
        other => panic!("expected a proof, got {other:?}"),
    };
    let (root_hash, total) = like_count_query(&contract, &a_range)
        .verify_aggregate_count_proof(&proof, pv)
        .expect("the range total proof verifies");
    assert_live_root_hash(&drive, root_hash);
    assert_eq!(total, 4, "A's likes, proved");

    let both_ranges = vec![both_authors(), every_post()];
    let proof = match count_likes(
        &drive,
        &contract,
        both_ranges.clone(),
        CountMode::GroupByIn,
        None,
        true,
    )
    .expect("the per-author range totals prove")
    {
        DocumentCountResponse::Proof(proof) => proof,
        other => panic!("expected a proof, got {other:?}"),
    };
    let (root_hash, per_author) = like_count_query(&contract, &both_ranges)
        .verify_carrier_aggregate_count_proof(&proof, None, true, pv)
        .expect("the per-author range totals verify");
    assert_live_root_hash(&drive, root_hash);
    assert_eq!(
        per_author,
        vec![(AUTHOR_A.to_vec(), 4), (AUTHOR_B.to_vec(), 5)],
        "each author's likes"
    );
}

/// An author without posts has no subtree under a counter index, and a range
/// total counts it as zero: across an `IN` the author's branch adds no likes,
/// posts or sums (the carrier proof drops it), and a range of that author
/// alone totals zero, its count and sum proofs verifying the author absent.
#[test]
fn should_total_an_author_without_posts_as_zero() {
    const AUTHOR_Z: [u8; 32] = [0xCC; 32];
    let (drive, contract) = setup_with(|index| {
        index.remove("rankedSummable");
        index.remove("rankedAverageable");
    });
    liked_posts(&drive, &contract);
    let pv = platform_version();
    let a_and_z = WhereClause {
        field: "postAuthor".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![
            Value::Identifier(AUTHOR_A),
            Value::Identifier(AUTHOR_Z),
        ]),
    };
    let drive_config = DriveConfig::default();
    let average = |where_clauses: Vec<WhereClause>| match drive
        .execute_document_average_request(
            DocumentAverageRequest {
                contract: &contract,
                document_type: contract
                    .document_type_for_name("like")
                    .expect("like doctype exists"),
                sum_property: "byPost".to_string(),
                where_clauses,
                resolved_time_ranges: vec![],
                order_clauses: vec![],
                mode: AverageMode::Aggregate,
                limit: None,
                prove: false,
                drive_config: &drive_config,
            },
            None,
            pv,
        )
        .expect("the range average executes")
    {
        DocumentAverageResponse::Aggregate { count, sum } => (count, sum),
        other => panic!("expected an aggregate average, got {other:?}"),
    };

    for (clauses, posts, likes) in [
        (vec![a_and_z.clone(), every_post()], 2u64, 4u64),
        (
            vec![
                equal("postAuthor", Value::Identifier(AUTHOR_Z)),
                every_post(),
            ],
            0,
            0,
        ),
    ] {
        match count_likes(
            &drive,
            &contract,
            clauses.clone(),
            CountMode::Aggregate,
            None,
            false,
        )
        .expect("the range count executes")
        {
            DocumentCountResponse::Aggregate(total) => assert_eq!(total, likes, "{clauses:?}"),
            other => panic!("expected an aggregate count, got {other:?}"),
        }
        match sum_likes(
            &drive,
            &contract,
            clauses.clone(),
            SumMode::Aggregate,
            None,
            false,
        )
        .expect("the range sum executes")
        {
            DocumentSumResponse::Aggregate(total) => {
                assert_eq!(total, likes as i64, "{clauses:?}")
            }
            other => panic!("expected an aggregate sum, got {other:?}"),
        }
        assert_eq!(
            average(clauses.clone()),
            (posts, likes as i64),
            "{clauses:?}"
        );
    }

    let z_range = vec![
        equal("postAuthor", Value::Identifier(AUTHOR_Z)),
        every_post(),
    ];
    let proof = match count_likes(
        &drive,
        &contract,
        z_range.clone(),
        CountMode::Aggregate,
        None,
        true,
    )
    .expect("the range count proves")
    {
        DocumentCountResponse::Proof(proof) => proof,
        other => panic!("expected a proof, got {other:?}"),
    };
    let (root_hash, total) = like_count_query(&contract, &z_range)
        .verify_aggregate_count_proof(&proof, pv)
        .expect("the count proof of an author without posts verifies");
    assert_live_root_hash(&drive, root_hash);
    assert_eq!(total, 0, "Z's likes, proved");
    let proof = match sum_likes(
        &drive,
        &contract,
        z_range.clone(),
        SumMode::Aggregate,
        None,
        true,
    )
    .expect("the range sum proves")
    {
        DocumentSumResponse::Proof(proof) => proof,
        other => panic!("expected a proof, got {other:?}"),
    };
    let (root_hash, total) = like_sum_query(&contract, &z_range)
        .verify_aggregate_sum_proof(&proof, pv)
        .expect("the sum proof of an author without posts verifies");
    assert_live_root_hash(&drive, root_hash);
    assert_eq!(total, 0, "Z's likes summed, proved");

    let a_and_z_range = vec![a_and_z, every_post()];
    let proof = match count_likes(
        &drive,
        &contract,
        a_and_z_range.clone(),
        CountMode::GroupByIn,
        None,
        true,
    )
    .expect("the per-author range totals prove")
    {
        DocumentCountResponse::Proof(proof) => proof,
        other => panic!("expected a proof, got {other:?}"),
    };
    let (root_hash, per_author) = like_count_query(&contract, &a_and_z_range)
        .verify_carrier_aggregate_count_proof(&proof, None, true, pv)
        .expect("the per-author range totals verify");
    assert_live_root_hash(&drive, root_hash);
    assert_eq!(
        per_author,
        vec![(AUTHOR_A.to_vec(), 4)],
        "the proof drops the author without posts"
    );
}

/// A range total through an index that ranks any level is refused cleanly,
/// with and without a proof: grovedb neither totals a range over an indexed
/// tree nor proves one through it, and the unproven read refuses a ranked
/// ancestor too, so the read, the proof and its verification agree. That
/// covers `byPost`, ranked at its last
/// property, the counter indexes as the fixture ranks them, an average over
/// them, the per-author totals of a count, a sum and an average across an
/// `IN` (the carrier proofs), and a counter index ranked only above the
/// range.
#[test]
fn should_refuse_a_range_total_through_a_ranked_index() {
    let refused = |result: Result<(), Error>| {
        matches!(
            result,
            Err(Error::Query(QuerySyntaxError::Unsupported(message)))
                if message.starts_with("a range total")
        )
    };
    let (drive, contract) = setup(true);
    liked_posts(&drive, &contract);
    let a_range = vec![
        equal("postAuthor", Value::Identifier(AUTHOR_A)),
        every_post(),
    ];
    let both_ranges = vec![both_authors(), every_post()];
    let drive_config = DriveConfig::default();
    let average = |where_clauses: Vec<WhereClause>, mode: AverageMode, prove: bool| {
        drive
            .execute_document_average_request(
                DocumentAverageRequest {
                    contract: &contract,
                    document_type: contract
                        .document_type_for_name("like")
                        .expect("like doctype exists"),
                    sum_property: "byPost".to_string(),
                    where_clauses,
                    resolved_time_ranges: vec![],
                    order_clauses: vec![],
                    mode,
                    limit: None,
                    prove,
                    drive_config: &drive_config,
                },
                None,
                platform_version(),
            )
            .map(|_| ())
    };
    for prove in [false, true] {
        for clauses in [vec![every_post()], a_range.clone()] {
            let result = count_likes(
                &drive,
                &contract,
                clauses.clone(),
                CountMode::Aggregate,
                None,
                prove,
            )
            .map(|_| ());
            assert!(refused(result), "{clauses:?} (prove: {prove})");
        }
        assert!(
            refused(average(a_range.clone(), AverageMode::Aggregate, prove)),
            "a range average (prove: {prove})"
        );
        let per_author_counts = count_likes(
            &drive,
            &contract,
            both_ranges.clone(),
            CountMode::GroupByIn,
            None,
            prove,
        )
        .map(|_| ());
        assert!(
            refused(per_author_counts),
            "per-author range counts (prove: {prove})"
        );
        let per_author_sums = sum_likes(
            &drive,
            &contract,
            both_ranges.clone(),
            SumMode::GroupByIn,
            None,
            prove,
        )
        .map(|_| ());
        assert!(
            refused(per_author_sums),
            "per-author range sums (prove: {prove})"
        );
        assert!(
            refused(average(both_ranges.clone(), AverageMode::GroupByIn, prove)),
            "per-author range averages (prove: {prove})"
        );
    }

    // Ranked only at `postAuthor`, above the range on `postId`.
    let (drive, contract) = setup_with(|index| {
        index.remove("rankedSummable");
    });
    liked_posts(&drive, &contract);
    for prove in [false, true] {
        let result = count_likes(
            &drive,
            &contract,
            a_range.clone(),
            CountMode::Aggregate,
            None,
            prove,
        )
        .map(|_| ());
        assert!(refused(result), "ranked above the range (prove: {prove})");
    }
}

/// "Top creators by likes" and "most engaging creators" through the ranked
/// query surface, proved and verified.
#[test]
fn should_rank_authors_by_likes_and_likes_per_post_with_proofs() {
    let (drive, contract) = setup(true);
    liked_posts(&drive, &contract);
    let group_by = vec!["postAuthor".to_string()];
    let order_by = vec![OrderClause {
        field: "byPost".to_string(),
        ascending: false,
    }];

    for (select, [b, a]) in [
        (
            SelectProjection::sum("byPost"),
            [RankedEntryValue::Sum(5), RankedEntryValue::Sum(4)],
        ),
        (
            SelectProjection::avg("byPost"),
            [
                RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(5, 1)),
                RankedEntryValue::AvgFixedPoint(compute_avg_fixed_point(4, 2)),
            ],
        ),
    ] {
        assert_eq!(
            proved_ranked_page(&drive, &contract, &select, &group_by, &order_by, &[]),
            vec![(AUTHOR_B.to_vec(), b), (AUTHOR_A.to_vec(), a)],
            "B has 5 likes on 1 post, A 4 on 2 ({select:?})"
        );
    }
}

/// On a counter index `rankedCountable` declares the ranking `count(*)`
/// reads, the sum ranking: spelled so, the counter indexes rank authors and
/// posts by likes, for a ranked `count(*)` and a ranked `sum(byPost)` alike.
#[test]
fn should_rank_by_likes_through_a_count_ranking_of_a_counter_index() {
    let (drive, contract) = setup_with(|index| {
        if let Some(ranking) = index.remove("rankedSummable") {
            index.insert("rankedCountable".to_string(), ranking);
        }
    });
    let (a1, a2, b1, _) = liked_posts(&drive, &contract);
    let by_author = vec!["postAuthor".to_string()];
    for (select, order_field, [b, a]) in [
        (
            SelectProjection::count_star(),
            RANKED_COUNT_ORDER_KEY,
            [RankedEntryValue::Count(5), RankedEntryValue::Count(4)],
        ),
        (
            SelectProjection::sum("byPost"),
            "byPost",
            [RankedEntryValue::Sum(5), RankedEntryValue::Sum(4)],
        ),
    ] {
        let order_by = vec![OrderClause {
            field: order_field.to_string(),
            ascending: false,
        }];
        assert_eq!(
            proved_ranked_page(&drive, &contract, &select, &by_author, &order_by, &[]),
            vec![(AUTHOR_B.to_vec(), b), (AUTHOR_A.to_vec(), a)],
            "{select:?}"
        );
    }
    let mut posts = [(a1.to_vec(), 3), (a2.to_vec(), 1), (b1.to_vec(), 5)];
    posts.sort_by_key(|(_, likes)| Reverse(*likes));
    let a_posts = vec![equal("postAuthor", Value::Identifier(AUTHOR_A))];
    assert_eq!(
        proved_ranked_page(
            &drive,
            &contract,
            &SelectProjection::count_star(),
            &["postId".to_string()],
            &[OrderClause {
                field: RANKED_COUNT_ORDER_KEY.to_string(),
                ascending: false,
            }],
            &a_posts,
        ),
        posts
            .iter()
            .filter(|(post, _)| post != &b1.to_vec())
            .map(|(post, likes)| (post.clone(), RankedEntryValue::Count(*likes)))
            .collect::<Vec<_>>(),
        "A's posts by likes"
    );
}

/// A ranked or bounded document count over a counter index reads its sum
/// rankings, its likes (the index keeps no count ranking): `count(*)`
/// ranks authors B (5 likes) over A (4), and `HAVING count(*) > 4` keeps
/// only B; pinned by `postAuthor IN [A, B]`, the posts rank and bound by
/// their likes across both branches. All proved.
#[test]
fn should_rank_and_bound_a_document_count_by_the_counters_sums() {
    let (drive, contract) = setup(true);
    let (a1, a2, b1, _) = liked_posts(&drive, &contract);
    let order_by = vec![OrderClause {
        field: RANKED_COUNT_ORDER_KEY.to_string(),
        ascending: false,
    }];

    let by_author = vec!["postAuthor".to_string()];
    assert_eq!(
        proved_ranked_page(
            &drive,
            &contract,
            &SelectProjection::count_star(),
            &by_author,
            &order_by,
            &[],
        ),
        vec![
            (AUTHOR_B.to_vec(), RankedEntryValue::Count(5)),
            (AUTHOR_A.to_vec(), RankedEntryValue::Count(4)),
        ]
    );
    assert_eq!(
        proved_having_count_entries(&drive, &contract, &by_author, &count_above(4), &[]),
        vec![(AUTHOR_B.to_vec(), RankedEntryValue::Count(5))],
        "only B has more than 4 likes"
    );

    // Pinned by an `IN`, one branch per author, merged on the sums.
    let by_post = vec!["postId".to_string()];
    let both_authors = vec![WhereClause {
        field: "postAuthor".to_string(),
        operator: WhereOperator::In,
        value: Value::Array(vec![
            Value::Identifier(AUTHOR_A),
            Value::Identifier(AUTHOR_B),
        ]),
    }];
    assert_eq!(
        proved_ranked_page(
            &drive,
            &contract,
            &SelectProjection::count_star(),
            &by_post,
            &order_by,
            &both_authors,
        ),
        vec![
            (b1.to_vec(), RankedEntryValue::Count(5)),
            (a1.to_vec(), RankedEntryValue::Count(3)),
            (a2.to_vec(), RankedEntryValue::Count(1)),
        ]
    );
    assert_eq!(
        proved_having_count_entries(&drive, &contract, &by_post, &count_above(2), &both_authors),
        vec![
            (a1.to_vec(), RankedEntryValue::Count(3)),
            (b1.to_vec(), RankedEntryValue::Count(5)),
        ],
        "the posts with more than 2 likes, ascending"
    );
}

/// A ranked `sum(byPost)` no index covers names the keyword that would: an
/// index with `summableOffCountIndex` over the source, ranked at the grouping
/// property through the level-addressed form, not a `summable` property.
#[test]
fn should_name_the_counter_keyword_when_no_index_ranks_a_sum_of_the_source() {
    let (_drive, contract) = setup(true);
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let group_by = vec!["postId".to_string()];
    let order_by = vec![OrderClause {
        field: "byPost".to_string(),
        ascending: false,
    }];
    let mode = detect_ranked_mode(
        &SelectProjection::sum("byPost"),
        &group_by,
        &[],
        &order_by,
        &[],
        PAGE,
        pv,
    )
    .expect("the ranked request is well formed");
    // Posts are ranked by likes only under an author or a hashtag pin
    let error = resolve_ranked_query_for_mode(
        contract.id_ref().to_buffer(),
        document_type,
        "like".to_string(),
        document_type.indexes(),
        &mode,
        &[],
        pv,
    )
    .expect_err("no index ranks every post by likes");
    let message = error.to_string();
    assert!(
        message.contains("`summableOffCountIndex: \"byPost\"`")
            && message.contains(r#"`rankedSummable: { "at": ["postId"] }`"#)
            && !message.contains("summable: \"byPost\""),
        "the hint must name the counter keyword: {message}"
    );
}

/// `sum(byPost)` and `avg(byPost)` pinned on every property but the last of
/// a counter index whose last property is unranked read the last property's
/// tree, as a `count(*)` with the same pins does: an author's or a hashtag's
/// likes, and with them its posts, unproved and proved alike, whether the
/// index ranks no level or ranks by sum alone at the first one, and when the
/// tree sits wrapped under another index's count trees.
#[test]
fn should_sum_and_average_likes_from_the_last_property_s_tree() {
    use crate::query::drive_document_sum_query::index_picker::find_summable_index_with_counts_for_where_clauses;

    let unranked = |index: &mut serde_json::Map<String, serde_json::Value>| {
        index.remove("rankedSummable");
        index.remove("rankedAverageable");
    };
    let setups = [
        setup_with(unranked),
        // Ranked by sum alone at the first property, whose value trees then
        // count no posts: the average reads the last property's tree
        setup_with(|index| {
            if index.contains_key("summableOffCountIndex") {
                let first = index["properties"][0]
                    .as_object()
                    .and_then(|property| property.keys().next().cloned())
                    .expect("a first property");
                index.remove("rankedAverageable");
                index.insert(
                    "rankedSummable".to_string(),
                    serde_json::json!({ "at": [first] }),
                );
            }
        }),
        // Beside a countable index ending at `postAuthor`, under whose count
        // trees the counter's `postId` tree sits wrapped to count nothing:
        // the proof still reads its posts
        setup_with_indices(|indices| {
            for index in indices.iter_mut() {
                unranked(index.as_object_mut().expect("an index"));
            }
            indices.push(serde_json::json!({
                "name": "byAuthor",
                "properties": [{ "postAuthor": "asc" }],
                "terminal": ["postId", "$ownerId"],
                "countable": "countable",
            }));
        }),
    ];
    for (drive, contract) in setups {
        liked_posts(&drive, &contract);
        let pv = platform_version();
        let document_type = contract
            .document_type_for_name("like")
            .expect("like doctype exists");
        let drive_config = DriveConfig::default();

        for (clauses, posts, likes) in [
            (vec![equal("postAuthor", Value::Identifier(AUTHOR_A))], 2, 4),
            (vec![equal("postAuthor", Value::Identifier(AUTHOR_B))], 1, 5),
            (
                vec![equal("hashtag", Value::Text("dash".to_string()))],
                3,
                9,
            ),
        ] {
            match sum_likes(
                &drive,
                &contract,
                clauses.clone(),
                SumMode::Aggregate,
                None,
                false,
            )
            .expect("the sum executes")
            {
                DocumentSumResponse::Aggregate(sum) => assert_eq!(sum, likes, "{clauses:?}"),
                other => panic!("expected an aggregate sum, got {other:?}"),
            }
            let proof = match sum_likes(
                &drive,
                &contract,
                clauses.clone(),
                SumMode::Aggregate,
                None,
                true,
            )
            .expect("the sum proves")
            {
                DocumentSumResponse::Proof(proof) => proof,
                other => panic!("expected a proof, got {other:?}"),
            };
            let query = like_sum_query(&contract, &clauses);
            assert!(query.index.is_summable_off_count_index());
            let (root_hash, entries) = query
                .verify_point_lookup_sum_proof(&proof, pv)
                .expect("the sum proof verifies");
            assert_live_root_hash(&drive, root_hash);
            let proved: i64 = entries.iter().filter_map(|entry| entry.sum).sum();
            assert_eq!(proved, likes, "proved {clauses:?}");

            let average = |prove| {
                drive
                    .execute_document_average_request(
                        DocumentAverageRequest {
                            contract: &contract,
                            document_type,
                            sum_property: "byPost".to_string(),
                            where_clauses: clauses.clone(),
                            resolved_time_ranges: vec![],
                            order_clauses: vec![],
                            mode: AverageMode::Aggregate,
                            limit: None,
                            prove,
                            drive_config: &drive_config,
                        },
                        None,
                        pv,
                    )
                    .expect("the average executes")
            };
            match average(false) {
                DocumentAverageResponse::Aggregate { count, sum } => {
                    assert_eq!((count, sum), (posts, likes), "{clauses:?}")
                }
                other => panic!("expected an aggregate average, got {other:?}"),
            }
            let DocumentAverageResponse::Proof(proof) = average(true) else {
                panic!("expected an average proof");
            };
            let index = find_summable_index_with_counts_for_where_clauses(
                document_type.indexes(),
                &clauses,
                "byPost",
                &[],
            )
            .expect("a counter index answers the average");
            let (root_hash, entries) = DriveDocumentSumQuery {
                document_type,
                contract_id: contract.id().to_buffer(),
                document_type_name: "like".to_string(),
                index,
                where_clauses: clauses.clone(),
                sum_property: "byPost".to_string(),
            }
            .verify_point_lookup_count_and_sum_proof(&proof, pv)
            .expect("the average proof verifies");
            assert_live_root_hash(&drive, root_hash);
            let proved = entries.iter().fold((0u64, 0i64), |(count, sum), entry| {
                (
                    count + entry.count.unwrap_or_default(),
                    sum + entry.sum.unwrap_or_default(),
                )
            });
            assert_eq!(proved, (posts, likes), "proved {clauses:?}");
        }
    }
}
