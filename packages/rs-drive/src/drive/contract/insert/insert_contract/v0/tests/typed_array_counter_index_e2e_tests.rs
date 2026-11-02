//! End-to-end coverage for **`preallocated` and `summableOffCountIndex`
//! indexes keyed through a typed-array equality** (protocol version 14), on
//! the `yappr-likes-tags-counters` fixture: Yappr's `like` binds its `tags`
//! to its post's through `where: { "tags": "tags" }`, so every like of a post
//! holds the post's whole list, and `byTagPost` keeps one counter per tag of
//! each post, counting every like of it.
//!
//! A post creates one zero counter per distinct tag (none without tags), each
//! like moves every one of its post's counters, a delete moves them back, and
//! the tag levels add up likes per tag and posts per tag. The dry runs
//! upper-bound the applied fees for 1 and 4 tags, and per-tag counts, sums
//! and rankings prove.

use super::index_only_array_index_e2e_tests::counted;
use super::index_only_e2e_tests::{
    assert_grovedb_is_consistent, assert_live_root_hash, doctype_path, platform_version,
    read_grove_element, sum_top_k,
};
use super::summable_off_count_index_e2e_tests::{like_sum_query, proved_ranked_page, sum_likes};
use crate::drive::Drive;
use crate::query::drive_document_count_query::CountMode;
use crate::query::drive_document_ranked_query::{RankedEntryValue, RANKED_COUNT_ORDER_KEY};
use crate::query::drive_document_sum_query::{DocumentSumResponse, SumMode};
use crate::query::projection::SelectProjection;
use crate::query::{key_array_element_clauses, OrderClause, WhereClause, WhereOperator};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_json_value;
use grovedb::Element;
use std::collections::BTreeMap;

const FIXTURE: &str =
    "tests/supporting_files/contract/yappr-likes/yappr-likes-tags-counters-contract.json";

const AUTHOR: [u8; 32] = [0xAA; 32];
const LIKER_1: [u8; 32] = [0x11; 32];
const LIKER_2: [u8; 32] = [0x22; 32];
const LIKER_3: [u8; 32] = [0x33; 32];
/// A like's time.
const T_MS: u64 = 1_750_000_000_000;

/// The fixture, with the like's indexes preallocated as written or with the
/// trees and counters created by the first like of a post instead.
fn setup(preallocated: bool) -> (Drive, DataContract) {
    setup_with(preallocated, |_| {})
}

/// [`setup`] with the like's index list edited by `edit` first.
fn setup_with(
    preallocated: bool,
    edit: impl FnOnce(&mut Vec<serde_json::Value>),
) -> (Drive, DataContract) {
    let pv = platform_version();
    let mut schema = json_document_to_json_value(FIXTURE).expect("read contract fixture");
    let indices = schema["documentSchemas"]["like"]["indices"]
        .as_array_mut()
        .expect("like indices");
    edit(indices);
    if !preallocated {
        for index in indices.iter_mut() {
            index
                .as_object_mut()
                .expect("an index")
                .remove("preallocated");
        }
    }
    let contract = DataContract::try_from_platform_versioned(
        serde_json::from_value(schema).expect("contract serialization format"),
        true,
        &mut vec![],
        pv,
    )
    .expect("parse the tag likes contract");
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

fn document_type<'a>(contract: &'a DataContract, name: &str) -> DocumentTypeRef<'a> {
    contract
        .document_type_for_name(name)
        .expect("the document type")
}

/// A post by `AUTHOR` with id `[id; 32]`, carrying `tags` as given: a list of
/// them, or no `tags` at all.
fn post(id: u8, tags: Option<&[&str]>) -> Document {
    let mut properties =
        BTreeMap::from([("content".to_string(), Value::Text("a post".to_string()))]);
    if let Some(tags) = tags {
        properties.insert("tags".to_string(), Value::from(tags));
    }
    Document::V0(DocumentV0 {
        id: Identifier::from([id; 32]),
        owner_id: Identifier::from(AUTHOR),
        properties,
        created_at: Some(T_MS),
        updated_at: Some(T_MS),
        ..Default::default()
    })
}

/// `liker`'s like of `post`, carrying the post's tags and author as its
/// reference's `where` requires.
fn like(post: &Document, liker: [u8; 32]) -> Document {
    let mut properties = BTreeMap::from([
        (
            "postId".to_string(),
            Value::Identifier(post.id().to_buffer()),
        ),
        ("postAuthor".to_string(), Value::Identifier(AUTHOR)),
    ]);
    if let Some(tags) = post.properties().get("tags") {
        properties.insert("tags".to_string(), tags.clone());
    }
    let mut id = post.id().to_buffer();
    id[..16].copy_from_slice(&liker[..16]);
    Document::V0(DocumentV0 {
        id: Identifier::from(id),
        owner_id: Identifier::from(liker),
        properties,
        created_at: Some(T_MS),
        ..Default::default()
    })
}

fn insert(
    drive: &Drive,
    contract: &DataContract,
    type_name: &str,
    document: &Document,
    apply: bool,
) -> FeeResult {
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: None,
                },
                contract,
                document_type: document_type(contract, type_name),
            },
            false,
            BlockInfo {
                time_ms: T_MS,
                ..Default::default()
            },
            apply,
            None,
            platform_version(),
            None,
        )
        .unwrap_or_else(|error| panic!("the {type_name} inserts: {error:?}"))
}

/// Deletes `like`, carrying its values but its `$createdAt`: only the
/// trending windows, whose entries outlive a delete, read it, so its rows do
/// not commit to it.
fn delete_like(drive: &Drive, contract: &DataContract, like: &Document, apply: bool) -> FeeResult {
    let mut like = like.clone();
    like.set_created_at(None);
    drive
        .delete_index_only_document_for_contract(
            like,
            contract,
            document_type(contract, "like"),
            BlockInfo {
                time_ms: T_MS,
                ..Default::default()
            },
            apply,
            None,
            platform_version(),
            None,
        )
        .expect("the like deletes")
}

/// The like type's tree followed by `segments`.
fn like_path(contract: &DataContract, segments: &[&[u8]]) -> Vec<Vec<u8>> {
    let mut path = doctype_path(contract);
    path.extend(segments.iter().map(|segment| segment.to_vec()));
    path
}

/// `byTagPost`'s counter of `post` under `tag`, if any.
fn tag_counter(drive: &Drive, contract: &DataContract, tag: &str, post: &Document) -> Option<i64> {
    match read_grove_element(
        drive,
        &like_path(contract, &[b"tags", tag.as_bytes(), b"postId"]),
        post.id().as_slice(),
    ) {
        None => None,
        Some(Element::SumItem(count, _)) => Some(count),
        Some(other) => panic!("expected a counter under {tag}, got {other:?}"),
    }
}

/// The likes a tag's value tree of `byTagPost` adds up, the sum of its
/// posts' counters.
fn tag_likes(drive: &Drive, contract: &DataContract, tag: &str) -> Option<i64> {
    read_grove_element(drive, &like_path(contract, &[b"tags"]), tag.as_bytes())
        .map(|element| element.count_sum_value_or_default().1)
}

/// Two posts: A tagged `dash` and `news` with 3 likes, B tagged `dash` with
/// 2, and an untagged post C with 1.
fn liked_posts(drive: &Drive, contract: &DataContract) -> [Document; 3] {
    let posts = [
        post(0xA1, Some(&["dash", "news"])),
        post(0xB2, Some(&["dash"])),
        post(0xC3, None),
    ];
    for post in &posts {
        insert(drive, contract, "post", post, true);
    }
    for (post, likers) in posts.iter().zip([
        &[LIKER_1, LIKER_2, LIKER_3][..],
        &[LIKER_1, LIKER_2][..],
        &[LIKER_3][..],
    ]) {
        for liker in likers {
            insert(drive, contract, "like", &like(post, *liker), true);
        }
    }
    posts
}

// ---------------------------------------------------------------------------
// Counters
// ---------------------------------------------------------------------------

/// A post creates one counter at zero per distinct tag, under the tag's value
/// tree, next to the preallocated `byPost` and `byAuthorPost` trees; a post
/// without tags (absent or empty) creates none under the tags.
#[test]
fn should_preallocate_a_zero_counter_per_tag_of_a_post() {
    let (drive, contract) = setup(true);
    let tagged = post(0xA1, Some(&["news", "dash", "evo", "solo"]));
    insert(&drive, &contract, "post", &tagged, true);
    for tag in ["dash", "evo", "news", "solo"] {
        assert_eq!(
            tag_counter(&drive, &contract, tag, &tagged),
            Some(0),
            "{tag}"
        );
        assert_eq!(tag_likes(&drive, &contract, tag), Some(0), "{tag}");
    }
    assert!(read_grove_element(
        &drive,
        &like_path(&contract, &[b"postId", tagged.id().as_slice()]),
        &[0]
    )
    .is_some());
    assert!(matches!(
        read_grove_element(
            &drive,
            &like_path(&contract, &[b"postAuthor", &AUTHOR, b"postId"]),
            tagged.id().as_slice()
        ),
        Some(Element::SumItem(0, _))
    ));

    for (id, tags) in [(0xB2, None), (0xC3, Some(&[][..]))] {
        let untagged = post(id, tags);
        insert(&drive, &contract, "post", &untagged, true);
        assert!(read_grove_element(
            &drive,
            &like_path(&contract, &[b"postId", untagged.id().as_slice()]),
            &[0]
        )
        .is_some());
    }
    // Only the tagged post's tags are there
    assert_eq!(
        sum_top_k(&drive, &like_path(&contract, &[b"tags"]), 10, true).len(),
        4
    );
    assert_grovedb_is_consistent(&drive);
}

/// A post whose list no like can hold preallocates nothing under the tags: one
/// longer than the like's `maxItems` (4), or one repeating a tag where the
/// like declares `uniqueItems`. A like must hold the post's list exactly.
#[test]
fn should_preallocate_no_tag_counter_for_a_list_no_like_can_hold() {
    let (drive, contract) = setup(true);
    for (id, tags) in [
        (0xA1, &["a", "b", "c", "d", "e"][..]),
        (0xB2, &["dash", "dash"][..]),
    ] {
        let unlikeable = post(id, Some(tags));
        insert(&drive, &contract, "post", &unlikeable, true);
        for tag in tags {
            assert_eq!(
                tag_counter(&drive, &contract, tag, &unlikeable),
                None,
                "{tag}"
            );
        }
        // The trees keyed by the post alone are still preallocated
        assert!(read_grove_element(
            &drive,
            &like_path(&contract, &[b"postId", unlikeable.id().as_slice()]),
            &[0]
        )
        .is_some());
    }
    assert_grovedb_is_consistent(&drive);
}

/// Each like moves its post's counter under every tag the post holds, which
/// the tag levels add up: `dash` has 5 likes over 2 posts, `news` 3 over 1,
/// and the untagged post is in neither. The count ranking at the tag and post
/// levels, a counter index's sum ranking, ranks by likes.
#[test]
fn should_count_every_like_of_a_post_under_each_of_its_tags() {
    for preallocated in [true, false] {
        let (drive, contract) = setup(preallocated);
        let [a, b, _] = liked_posts(&drive, &contract);
        assert_eq!(tag_counter(&drive, &contract, "dash", &a), Some(3));
        assert_eq!(tag_counter(&drive, &contract, "news", &a), Some(3));
        assert_eq!(tag_counter(&drive, &contract, "dash", &b), Some(2));
        assert_eq!(tag_counter(&drive, &contract, "news", &b), None);
        assert_eq!(tag_likes(&drive, &contract, "dash"), Some(5));
        assert_eq!(tag_likes(&drive, &contract, "news"), Some(3));

        assert_eq!(
            sum_top_k(&drive, &like_path(&contract, &[b"tags"]), 10, true),
            vec![(5, b"dash".to_vec()), (3, b"news".to_vec())]
        );
        assert_eq!(
            sum_top_k(
                &drive,
                &like_path(&contract, &[b"tags", b"dash", b"postId"]),
                10,
                true
            ),
            vec![(3, a.id().to_vec()), (2, b.id().to_vec())]
        );
        assert_grovedb_is_consistent(&drive);
    }
}

/// An unlike takes the like back from every tag of its post; the last one
/// leaves preallocated counters at zero and removes the others with the trees
/// they leave empty. A re-like moves them again.
#[test]
fn should_take_a_like_back_from_every_tag_on_delete() {
    for preallocated in [true, false] {
        let (drive, contract) = setup(preallocated);
        let tagged = post(0xA1, Some(&["dash", "news"]));
        insert(&drive, &contract, "post", &tagged, true);
        let likes = [like(&tagged, LIKER_1), like(&tagged, LIKER_2)];
        for like in &likes {
            insert(&drive, &contract, "like", like, true);
        }

        delete_like(&drive, &contract, &likes[0], true);
        for tag in ["dash", "news"] {
            assert_eq!(
                tag_counter(&drive, &contract, tag, &tagged),
                Some(1),
                "{tag}"
            );
        }
        delete_like(&drive, &contract, &likes[1], true);
        for tag in ["dash", "news"] {
            if preallocated {
                assert_eq!(tag_counter(&drive, &contract, tag, &tagged), Some(0));
                assert_eq!(tag_likes(&drive, &contract, tag), Some(0));
            } else {
                assert_eq!(tag_likes(&drive, &contract, tag), None, "{tag} is pruned");
            }
        }

        insert(&drive, &contract, "like", &like(&tagged, LIKER_3), true);
        for tag in ["dash", "news"] {
            assert_eq!(
                tag_counter(&drive, &contract, tag, &tagged),
                Some(1),
                "{tag}"
            );
        }
        assert_grovedb_is_consistent(&drive);
    }
}

/// The like with its per-tag counters under each post, `[postId, tags]`, in
/// place of `byTagPost`: the counters of one post are siblings under its
/// `tags` tree, each tag's created at zero with the post, moved together by
/// every like and back by every unlike, the last unlike of an index that is
/// not preallocated removing them and the trees they leave empty.
fn setup_counters_under_each_post(preallocated: bool) -> (Drive, DataContract) {
    setup_with(preallocated, |indices| {
        indices.retain(|index| index["name"] != "byTagPost");
        indices.push(serde_json::json!({
            "name": "byPostTag",
            "properties": [{ "postId": "asc" }, { "tags": "asc" }],
            "summableOffCountIndex": "byPost",
            "rangeSummable": true,
            "preallocated": true,
        }));
    })
}

/// `byPostTag`'s counter of `post` under `tag`, if any.
fn post_tag_counter(
    drive: &Drive,
    contract: &DataContract,
    post: &Document,
    tag: &str,
) -> Option<i64> {
    match read_grove_element(
        drive,
        &like_path(contract, &[b"postId", post.id().as_slice(), b"tags"]),
        tag.as_bytes(),
    ) {
        None => None,
        Some(Element::SumItem(count, _)) => Some(count),
        Some(other) => panic!("expected a counter under {tag}, got {other:?}"),
    }
}

#[test]
fn should_keep_sibling_counters_per_tag_under_each_post() {
    for preallocated in [true, false] {
        let (drive, contract) = setup_counters_under_each_post(preallocated);
        let tags = ["dash", "evo", "news", "solo"];
        let tagged = post(0xA1, Some(&tags));
        insert(&drive, &contract, "post", &tagged, true);
        let counters =
            |drive: &Drive| tags.map(|tag| post_tag_counter(drive, &contract, &tagged, tag));
        let expected = |count: i64| {
            if preallocated || count > 0 {
                [Some(count); 4]
            } else {
                [None; 4]
            }
        };
        assert_eq!(
            counters(&drive),
            expected(0),
            "preallocated: {preallocated}"
        );

        let likes = [like(&tagged, LIKER_1), like(&tagged, LIKER_2)];
        for liked in &likes {
            let estimated = insert(&drive, &contract, "like", liked, false);
            let applied = insert(&drive, &contract, "like", liked, true);
            assert!(
                estimated.storage_fee >= applied.storage_fee
                    && estimated.processing_fee >= applied.processing_fee,
                "like estimated {estimated:?} under applied {applied:?}"
            );
        }
        assert_eq!(counters(&drive), expected(2));
        for liked in &likes {
            let estimated = delete_like(&drive, &contract, liked, false);
            let applied = delete_like(&drive, &contract, liked, true);
            assert!(
                estimated.processing_fee >= applied.processing_fee,
                "unlike estimated {} under applied {}",
                estimated.processing_fee,
                applied.processing_fee
            );
        }
        assert_eq!(counters(&drive), expected(0));
        if !preallocated {
            assert!(
                read_grove_element(
                    &drive,
                    &like_path(&contract, &[b"postId", tagged.id().as_slice()]),
                    b"tags"
                )
                .is_none(),
                "the drained counters' tree is pruned"
            );
        }
        assert_grovedb_is_consistent(&drive);
    }
}

// ---------------------------------------------------------------------------
// Fees
// ---------------------------------------------------------------------------

/// The dry runs upper-bound the applied fees with a counter per tag: a post's
/// insert (its preallocation), a like's insert and a like's delete
/// (processing; its storage is refunded), for one tag and for four,
/// preallocated or not.
#[test]
fn should_upper_bound_post_and_like_fees_with_their_dry_runs() {
    for preallocated in [true, false] {
        for tags in [&["dash"][..], &["dash", "evo", "news", "solo"][..]] {
            let (drive, contract) = setup(preallocated);
            // Another post first, so the tag trees are not all new
            insert(
                &drive,
                &contract,
                "post",
                &post(0xB2, Some(&["dash"])),
                true,
            );
            let tagged = post(0xA1, Some(tags));
            let estimated = insert(&drive, &contract, "post", &tagged, false);
            let applied = insert(&drive, &contract, "post", &tagged, true);
            let context = format!("preallocated: {preallocated}, {} tags", tags.len());
            assert!(
                estimated.storage_fee >= applied.storage_fee
                    && estimated.processing_fee >= applied.processing_fee,
                "{context}: post estimated {estimated:?} under applied {applied:?}"
            );

            for liker in [LIKER_1, LIKER_2] {
                let liked = like(&tagged, liker);
                let estimated = insert(&drive, &contract, "like", &liked, false);
                let applied = insert(&drive, &contract, "like", &liked, true);
                assert!(
                    estimated.storage_fee >= applied.storage_fee
                        && estimated.processing_fee >= applied.processing_fee,
                    "{context}: like estimated {estimated:?} under applied {applied:?}"
                );
            }
            for liker in [LIKER_1, LIKER_2] {
                let liked = like(&tagged, liker);
                let estimated = delete_like(&drive, &contract, &liked, false);
                let applied = delete_like(&drive, &contract, &liked, true);
                assert!(
                    estimated.processing_fee >= applied.processing_fee,
                    "{context}: unlike estimated {} under applied {}",
                    estimated.processing_fee,
                    applied.processing_fee
                );
            }
            assert_grovedb_is_consistent(&drive);
        }
    }
}

/// Four tags cost a post more preallocation than one: each tag's trees are
/// priced. The post's own writes (its `tagAndTime` entries, its larger
/// serialization) grow with its tags too, so the preallocation is measured
/// as what a post pays beyond the same post when the like indexes are not
/// preallocated.
#[test]
fn should_price_each_tag_a_post_preallocates() {
    let preallocation = |tags: &[&str]| {
        let tagged = post(0xA1, Some(tags));
        let (drive, contract) = setup(true);
        let with = insert(&drive, &contract, "post", &tagged, true);
        let (drive, contract) = setup(false);
        let without = insert(&drive, &contract, "post", &tagged, true);
        (
            with.storage_fee - without.storage_fee,
            with.processing_fee - without.processing_fee,
        )
    };
    let (one_storage, one_processing) = preallocation(&["dash"]);
    let (four_storage, four_processing) = preallocation(&["dash", "evo", "news", "solo"]);
    assert!(
        four_storage > one_storage,
        "{four_storage} <= {one_storage}"
    );
    assert!(
        four_processing > one_processing,
        "{four_processing} <= {one_processing}"
    );
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

fn clause(field: &str, operator: WhereOperator, value: Value) -> WhereClause {
    WhereClause {
        field: field.to_string(),
        operator,
        value,
    }
}

fn contains(tag: &str) -> WhereClause {
    clause(
        "tags",
        WhereOperator::Contains,
        Value::Text(tag.to_string()),
    )
}

fn post_is(post: &Document) -> WhereClause {
    clause(
        "postId",
        WhereOperator::Equal,
        Value::Identifier(post.id().to_buffer()),
    )
}

/// The clauses as the server keys them, `contains` as an element's key.
fn keyed(
    contract: &DataContract,
    clauses: &[WhereClause],
    grouped_by_in: bool,
) -> Vec<WhereClause> {
    key_array_element_clauses(
        clauses.to_vec(),
        document_type(contract, "like"),
        grouped_by_in,
        platform_version(),
    )
    .expect("the clauses key")
}

/// A `sum(byPost)` over likes, unproved and proved through the point lookup
/// verifier with the clauses keyed as the server keys them: one total.
fn summed(drive: &Drive, contract: &DataContract, clauses: Vec<WhereClause>) -> i64 {
    let unproved = match sum_likes(
        drive,
        contract,
        clauses.clone(),
        SumMode::Aggregate,
        None,
        false,
    )
    .expect("the sum executes")
    {
        DocumentSumResponse::Aggregate(sum) => sum,
        other => panic!("expected an aggregate sum, got {other:?}"),
    };
    let proof = match sum_likes(
        drive,
        contract,
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
    let (root_hash, entries) = like_sum_query(contract, &keyed(contract, &clauses, false))
        .verify_point_lookup_sum_proof(&proof, platform_version())
        .expect("the sum proof verifies");
    assert_live_root_hash(drive, root_hash);
    let proved: i64 = entries.iter().filter_map(|entry| entry.sum).sum();
    assert_eq!(unproved, proved, "unproved and proved sums agree");
    proved
}

/// A tag's likes, a post's likes under a tag (zero for a post nobody liked),
/// and one total per tag of several are read from the counters, as counts
/// and as sums, unproved and proved alike.
#[test]
fn should_count_and_sum_likes_per_tag_with_proofs() {
    let (drive, contract) = setup(true);
    let [a, b, _] = liked_posts(&drive, &contract);
    let unliked = post(0xD4, Some(&["news"]));
    insert(&drive, &contract, "post", &unliked, true);
    let total = |total: u64| BTreeMap::from([(String::new(), total)]);

    for (clauses, likes) in [
        (vec![contains("dash")], 5),
        (vec![contains("news")], 3),
        (vec![contains("dash"), post_is(&a)], 3),
        (vec![contains("dash"), post_is(&b)], 2),
        (vec![contains("news"), post_is(&unliked)], 0),
    ] {
        assert_eq!(
            counted(
                &drive,
                &contract,
                clauses.clone(),
                vec![],
                CountMode::Aggregate
            ),
            total(likes),
            "{clauses:?}"
        );
        assert_eq!(
            summed(&drive, &contract, clauses.clone()),
            likes as i64,
            "{clauses:?}"
        );
    }
    assert_eq!(
        counted(
            &drive,
            &contract,
            vec![clause(
                "tags",
                WhereOperator::ContainsAny,
                platform_value!(["dash", "news"]),
            )],
            vec![],
            CountMode::GroupByIn
        ),
        BTreeMap::from([("dash".to_string(), 5), ("news".to_string(), 3)])
    );
    assert_grovedb_is_consistent(&drive);
}

/// Tags rank by likes (`dash` 5 over `news` 3), and a tag's posts by likes
/// (A 3 over B 2 under `dash`), through the counter index's count ranking,
/// proved.
#[test]
fn should_rank_tags_and_their_posts_by_likes_with_proofs() {
    let (drive, contract) = setup(true);
    let [a, b, _] = liked_posts(&drive, &contract);
    let select = SelectProjection::count_star();
    let by_count = [OrderClause {
        field: RANKED_COUNT_ORDER_KEY.to_string(),
        ascending: false,
    }];
    assert_eq!(
        proved_ranked_page(
            &drive,
            &contract,
            &select,
            &["tags".to_string()],
            &by_count,
            &[]
        ),
        vec![
            (b"dash".to_vec(), RankedEntryValue::Count(5)),
            (b"news".to_vec(), RankedEntryValue::Count(3)),
        ]
    );
    assert_eq!(
        proved_ranked_page(
            &drive,
            &contract,
            &select,
            &["postId".to_string()],
            &by_count,
            &[contains("dash")]
        ),
        vec![
            (a.id().to_vec(), RankedEntryValue::Count(3)),
            (b.id().to_vec(), RankedEntryValue::Count(2)),
        ]
    );
}

/// A like whose tags differ from its post's would land in groups of their
/// own: Drive writes what it is given (the reference check, before it,
/// refuses such a like), so a like of B under `news` creates a group B never
/// preallocated, and the `dash` counter of B does not move.
#[test]
fn should_move_only_the_groups_a_like_names() {
    let (drive, contract) = setup(true);
    let tagged = post(0xB2, Some(&["dash"]));
    insert(&drive, &contract, "post", &tagged, true);
    let mut stray = like(&tagged, LIKER_1);
    stray.set("tags", Value::from(&["news"][..]));
    insert(&drive, &contract, "like", &stray, true);
    assert_eq!(tag_counter(&drive, &contract, "dash", &tagged), Some(0));
    assert_eq!(tag_counter(&drive, &contract, "news", &tagged), Some(1));
    assert_grovedb_is_consistent(&drive);
}
