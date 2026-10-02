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
    assert_grovedb_is_consistent, delete_like, doctype_path, insert_like, platform_version,
    read_grove_element, sum_top_k,
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
use crate::query::drive_document_sum_query::index_picker::find_summable_index_for_where_clauses;
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
    let pv = platform_version();
    let mut schema = json_document_to_json_value(FIXTURE).expect("read contract fixture");
    for index in schema["documentSchemas"]["like"]["indices"]
        .as_array_mut()
        .expect("like indices")
    {
        edit(index.as_object_mut().expect("an index"));
    }
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
    let mut post = document_type
        .random_document(Some(seed), pv)
        .expect("random post");
    post.set_properties(
        [("hashtag".to_string(), Value::Text(hashtag.to_string()))]
            .into_iter()
            .collect(),
    );
    post.set_owner_id(Identifier::from(author));
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

/// A verified proof's root hash must be the live grovedb root.
fn assert_live_root_hash(drive: &Drive, root_hash: [u8; 32]) {
    assert_eq!(
        root_hash,
        drive
            .grove
            .root_hash(None, &platform_version().drive.grove_version)
            .unwrap()
            .expect("root hash must be readable"),
        "the proof must reconstruct the live grovedb root hash"
    );
}

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
    DriveOperation::DocumentOperation(DocumentOperationType::AddDocument {
        owned_document_info: OwnedDocumentInfo {
            document_info: DocumentRefInfo((like, None)),
            owner_id: None,
        },
        contract_info: DataContractInfo::BorrowedDataContract(contract),
        document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr("like"),
        override_document: false,
    })
}

/// A Drive batch writing two likes is refused, applied, dry-run or converted:
/// each like's conversion reads the counter the other moves, so the counter
/// would move once. One like a batch goes through.
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

/// The source index's name addresses the summed likes: one post's, one
/// author's (read off the author's value tree, which sums its posts), with
/// a proof that verifies to the same total.
#[test]
fn should_sum_likes_by_the_source_index_name() {
    let (drive, contract) = setup(true);
    let (a1, _, _, _) = liked_posts(&drive, &contract);
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let drive_config = DriveConfig::default();

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
        let request = |prove: bool| DocumentSumRequest {
            contract: &contract,
            document_type,
            sum_property: "byPost".to_string(),
            where_clauses: clauses.clone(),
            resolved_time_ranges: vec![],
            order_clauses: vec![],
            mode: SumMode::Aggregate,
            limit: None,
            prove,
            drive_config: &drive_config,
        };
        match drive
            .execute_document_sum_request(request(false), None, platform_version())
            .expect("the sum must execute")
        {
            DocumentSumResponse::Aggregate(sum) => assert_eq!(sum, total, "{clauses:?}"),
            other => panic!("expected an aggregate sum, got {other:?}"),
        }
        let proof = match drive
            .execute_document_sum_request(request(true), None, platform_version())
            .expect("the proved sum must execute")
        {
            DocumentSumResponse::Proof(proof) => proof,
            other => panic!("expected a proof, got {other:?}"),
        };
        let index =
            find_summable_index_for_where_clauses(document_type.indexes(), &clauses, "byPost", &[])
                .expect("a summableOffCountIndex index answers");
        assert!(index.is_summable_off_count_index());
        let (root_hash, entries) = DriveDocumentSumQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "like".to_string(),
            index,
            where_clauses: clauses.clone(),
            sum_property: "byPost".to_string(),
        }
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
        drive.execute_document_count_request(
            DocumentCountRequest {
                contract: &contract,
                document_type,
                where_clauses,
                resolved_time_ranges: vec![],
                order_clauses: vec![],
                mode: CountMode::Aggregate,
                limit: None,
                prove,
                drive_config: &drive_config,
            },
            None,
            pv,
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
        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &clauses,
            &[],
        )
        .expect("an index answers the count");
        let (root_hash, entries) = DriveDocumentCountQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "like".to_string(),
            index,
            where_clauses: clauses.clone(),
        }
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
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let drive_config = DriveConfig::default();
    let count_page =
        |where_clauses: Vec<WhereClause>, mode: CountMode, limit: Option<u32>, prove: bool| {
            drive.execute_document_count_request(
                DocumentCountRequest {
                    contract: &contract,
                    document_type,
                    where_clauses,
                    resolved_time_ranges: vec![],
                    order_clauses: vec![],
                    mode,
                    limit,
                    prove,
                    drive_config: &drive_config,
                },
                None,
                pv,
            )
        };
    let count = |where_clauses: Vec<WhereClause>, mode: CountMode, prove: bool| {
        count_page(where_clauses, mode, None, prove)
    };
    let every_post = WhereClause {
        field: "postId".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Identifier([0; 32]),
    };
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
        let page = match count_page(
            vec![
                equal("postAuthor", Value::Identifier(AUTHOR_A)),
                WhereClause {
                    field: "postId".to_string(),
                    operator: WhereOperator::GreaterThan,
                    value: Value::Identifier(after),
                },
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
        paged.push((entry.key.clone(), entry.count.unwrap_or(0)));
    }
    assert_eq!(paged, a_posts.to_vec(), "every page of one");

    for (clauses, mode, expected) in [
        (
            vec![
                equal("postAuthor", Value::Identifier(AUTHOR_A)),
                every_post.clone(),
            ],
            CountMode::GroupByRange,
            a_posts
                .iter()
                .map(|(post, likes)| (None, post.clone(), *likes))
                .collect::<Vec<_>>(),
        ),
        (
            vec![
                WhereClause {
                    field: "postAuthor".to_string(),
                    operator: WhereOperator::In,
                    value: Value::Array(vec![
                        Value::Identifier(AUTHOR_A),
                        Value::Identifier(AUTHOR_B),
                    ]),
                },
                every_post.clone(),
            ],
            CountMode::GroupByCompound,
            both.iter()
                .map(|(author, post, likes)| (Some(author.clone()), post.clone(), *likes))
                .collect(),
        ),
    ] {
        let entries = match count(clauses.clone(), mode, false).expect("the range count executes") {
            DocumentCountResponse::Entries(entries) => entries,
            other => panic!("expected entries, got {other:?}"),
        };
        assert_eq!(
            entries
                .iter()
                .map(|entry| (
                    entry.in_key.clone(),
                    entry.key.clone(),
                    entry.count.unwrap_or(0)
                ))
                .collect::<Vec<_>>(),
            expected,
            "{mode:?}"
        );
        let proof = match count(clauses.clone(), mode, true).expect("the range count proves") {
            DocumentCountResponse::Proof(proof) => proof,
            other => panic!("expected a proof, got {other:?}"),
        };
        let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            &clauses,
            &[],
        )
        .expect("the counter index answers the range count");
        assert!(index.is_summable_off_count_index());
        let (root_hash, verified) = DriveDocumentCountQuery {
            document_type,
            contract_id: contract.id().to_buffer(),
            document_type_name: "like".to_string(),
            index,
            where_clauses: clauses,
        }
        .verify_distinct_count_proof(&proof, DEFAULT_QUERY_LIMIT, true, pv)
        .expect("the range count proof verifies");
        assert_live_root_hash(&drive, root_hash);
        assert_eq!(verified, entries, "{mode:?}");
    }

    // The fixture ranks `postId`, an indexed tree grovedb sums no range over.
    let a_range = vec![equal("postAuthor", Value::Identifier(AUTHOR_A)), every_post];
    let refused = count(a_range, CountMode::Aggregate, false);
    assert!(
        matches!(refused, Err(Error::Query(QuerySyntaxError::Unsupported(_)))),
        "got {refused:?}"
    );
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
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let drive_config = DriveConfig::default();
    let count = |where_clauses: Vec<WhereClause>, mode: CountMode, prove: bool| {
        drive.execute_document_count_request(
            DocumentCountRequest {
                contract: &contract,
                document_type,
                where_clauses,
                resolved_time_ranges: vec![],
                order_clauses: vec![],
                mode,
                limit: None,
                prove,
                drive_config: &drive_config,
            },
            None,
            pv,
        )
    };
    let counter_query = |where_clauses: &[WhereClause]| DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "like".to_string(),
        index: DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
            document_type.indexes(),
            where_clauses,
            &[],
        )
        .expect("the counter index answers the range count"),
        where_clauses: where_clauses.to_vec(),
    };
    let every_post = WhereClause {
        field: "postId".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::Identifier([0; 32]),
    };

    let a_range = vec![
        equal("postAuthor", Value::Identifier(AUTHOR_A)),
        every_post.clone(),
    ];
    match count(a_range.clone(), CountMode::Aggregate, false).expect("the range total executes") {
        DocumentCountResponse::Aggregate(total) => assert_eq!(total, 4, "A's likes"),
        other => panic!("expected an aggregate count, got {other:?}"),
    }
    let proof =
        match count(a_range.clone(), CountMode::Aggregate, true).expect("the range total proves") {
            DocumentCountResponse::Proof(proof) => proof,
            other => panic!("expected a proof, got {other:?}"),
        };
    let (root_hash, total) = counter_query(&a_range)
        .verify_aggregate_count_proof(&proof, pv)
        .expect("the range total proof verifies");
    assert_live_root_hash(&drive, root_hash);
    assert_eq!(total, 4, "A's likes, proved");

    let both_ranges = vec![
        WhereClause {
            field: "postAuthor".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Identifier(AUTHOR_A),
                Value::Identifier(AUTHOR_B),
            ]),
        },
        every_post,
    ];
    let proof = match count(both_ranges.clone(), CountMode::GroupByIn, true)
        .expect("the per-author range totals prove")
    {
        DocumentCountResponse::Proof(proof) => proof,
        other => panic!("expected a proof, got {other:?}"),
    };
    let (root_hash, per_author) = counter_query(&both_ranges)
        .verify_carrier_aggregate_count_proof(&proof, None, true, pv)
        .expect("the per-author range totals verify");
    assert_live_root_hash(&drive, root_hash);
    assert_eq!(
        per_author,
        vec![(AUTHOR_A.to_vec(), 4), (AUTHOR_B.to_vec(), 5)],
        "each author's likes"
    );
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

/// A ranked or bounded document count over a counter index reads its sums,
/// its likes, where its count secondaries would count posts: `count(*)`
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
