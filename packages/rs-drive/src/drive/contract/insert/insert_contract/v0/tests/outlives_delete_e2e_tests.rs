//! End-to-end coverage for **`outlivesDelete`** indexes (meta-schema v3 /
//! PV14): a like type whose `byTrendPost` time window keeps its entries when
//! a like is deleted, leaving them to expire with their window, and keeps an
//! entry already there when the same liker likes the same post again.
//!
//! Only the window involves `$createdAt`, so the row commits to no timestamp
//! and a delete carries none: the like is removed from `byPost`, `byLiker`
//! and `byHashtagPost` by its values alone.

use super::index_only_e2e_tests::{
    assert_grovedb_is_consistent, assert_live_root_hash, build_like, delete_like, insert_like,
    platform_version, read_grove_element,
};
use crate::drive::document::layout::{document_type_layout, LayoutNode, LayoutNote};
use crate::drive::Drive;
use crate::query::{DriveDocumentCountQuery, RangeCountOptions, ResolvedTimeRange};
use crate::query::{WhereClause, WhereOperator};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{
    index_only_row_commits_created_at, Index, TimeRangeTransform,
};
use dpp::document::{Document, DocumentV0Setters};
use dpp::platform_value::Value;
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_json_value;
use grovedb::Element;
use serde_json::json;

const POST_A: [u8; 32] = [0xA1; 32];
const POST_B: [u8; 32] = [0xB1; 32];
const OWNER_1: [u8; 32] = [0x11; 32];
/// A like's time, and one a minute later in the same daily window.
const T_MS: u64 = 1_750_000_000_000;
const T_LATER_MS: u64 = T_MS + 60_000;

/// The yappr-likes `like` type with a daily `byTrendPost` window over
/// `$createdAt`, kept for seven days, which outlives deletes when `outlives`.
fn setup_trending_likes(outlives: bool) -> (Drive, DataContract) {
    setup_trending_likes_with(outlives, false)
}

/// [`setup_trending_likes`], with a `byPostTime` index a delete clears keyed
/// by `$createdAt` too when `timed`: the rows then commit to their timestamp.
fn setup_trending_likes_with(outlives: bool, timed: bool) -> (Drive, DataContract) {
    let pv = platform_version();
    let mut schema = json_document_to_json_value(
        "tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json",
    )
    .expect("read contract fixture");
    let document_schemas = schema["documentSchemas"]
        .as_object_mut()
        .expect("document schemas");
    document_schemas.retain(|name, _| name == "like" || name == "post");
    let like = &mut document_schemas["like"];
    like["required"] = json!(["postId", "$createdAt"]);
    like["indices"]
        .as_array_mut()
        .expect("like indices")
        .push(json!({
            "name": "byTrendPost",
            "properties": [{ "$createdAt": "asc" }, { "postId": "asc" }],
            "terminal": "$ownerId",
            "countable": "countable",
            "timeRange": { "on": "$createdAt", "range": 86400, "step": 86400, "ttl": 604800 },
            "outlivesDelete": outlives,
        }));
    if timed {
        like["indices"]
            .as_array_mut()
            .expect("like indices")
            .push(json!({
                "name": "byPostTime",
                "properties": [{ "postId": "asc" }, { "$createdAt": "asc" }],
                "terminal": "$ownerId",
            }));
    }
    let contract = DataContract::try_from_platform_versioned(
        serde_json::from_value(schema).expect("contract serialization format"),
        true,
        &mut vec![],
        pv,
    )
    .expect("parse the trending likes contract");
    drive_with(contract)
}

fn drive_with(contract: DataContract) -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version(),
        )
        .expect("expected to apply the contract");
    (drive, contract)
}

fn like_at(contract: &DataContract, created_at: u64, seed: u64) -> Document {
    let mut like = build_like(contract, "dash", POST_A, OWNER_1, seed);
    like.set_created_at(Some(created_at));
    like
}

fn index<'a>(contract: &'a DataContract, name: &str) -> &'a Index {
    contract
        .document_types()
        .get("like")
        .expect("like doctype exists")
        .indexes()
        .get(name)
        .expect("the index exists")
}

/// The entry `like` wrote under `index_name`, one per window path, or `None`
/// where there is none.
fn entries(
    drive: &Drive,
    contract: &DataContract,
    index_name: &str,
    like: &Document,
) -> Vec<(Vec<Vec<u8>>, Option<Element>)> {
    let (paths, member_key) = Drive::index_only_entry_paths_and_key(
        contract.id(),
        contract
            .document_type_for_name("like")
            .expect("like doctype exists"),
        index(contract, index_name),
        like,
        platform_version(),
    )
    .expect("entry paths");
    assert!(!paths.is_empty(), "{index_name} keys the like somewhere");
    paths
        .into_iter()
        .map(|path| {
            let element = read_grove_element(drive, &path, &member_key);
            (path, element)
        })
        .collect()
}

/// The number of entries the member tree at `path` counts.
fn member_count(drive: &Drive, path: &[Vec<u8>]) -> u64 {
    let (member_key, parent) = path.split_last().expect("a member tree path");
    match read_grove_element(drive, parent, member_key) {
        Some(Element::CountTree(_, count, _)) => count,
        other => panic!("expected a count tree, got {other:?}"),
    }
}

#[test]
fn should_leave_outliving_entries_on_delete_and_keep_them_on_a_like_again() {
    let (drive, contract) = setup_trending_likes(true);
    assert!(!index_only_row_commits_created_at(
        contract
            .document_type_for_name("like")
            .expect("like doctype exists")
            .required_fields(),
        contract
            .document_type_for_name("like")
            .expect("like doctype exists")
            .index_structure(),
    ));

    let like = like_at(&contract, T_MS, 1);
    insert_like(&drive, &contract, &like, true).expect("insert like");
    let trend_before = entries(&drive, &contract, "byTrendPost", &like);
    assert!(trend_before.iter().all(|(_, element)| element.is_some()));
    assert!(entries(&drive, &contract, "byPost", &like)
        .iter()
        .all(|(_, element)| element.is_some()));

    // The delete carries no `$createdAt`: the row does not commit to it, and
    // a delete carrying one is refused
    assert!(delete_like(&drive, &contract, like.clone(), true).is_err());
    let mut unlike = like.clone();
    unlike.set_created_at(None);
    let estimated = delete_like(&drive, &contract, unlike.clone(), false).expect("estimate");
    let actual = delete_like(&drive, &contract, unlike, true).expect("unlike");
    assert!(
        estimated.storage_fee >= actual.storage_fee
            && estimated.processing_fee >= actual.processing_fee,
        "the estimate {estimated:?} must upper-bound the delete {actual:?}"
    );
    for index_name in ["byPost", "byLiker", "byHashtagPost"] {
        assert!(
            entries(&drive, &contract, index_name, &like)
                .iter()
                .all(|(_, element)| element.is_none()),
            "the delete removes the like from {index_name}"
        );
    }
    let trend_after_delete = entries(&drive, &contract, "byTrendPost", &like);
    assert_eq!(
        trend_after_delete, trend_before,
        "the window keeps the entry the delete left"
    );

    // The same liker likes the same post again in the same window: the
    // window's entry stands for them already and is kept as it is
    let like_again = like_at(&contract, T_LATER_MS, 2);
    let estimated = insert_like(&drive, &contract, &like_again, false).expect("estimate");
    let actual = insert_like(&drive, &contract, &like_again, true).expect("like again");
    assert!(
        estimated.storage_fee >= actual.storage_fee
            && estimated.processing_fee >= actual.processing_fee,
        "the estimate {estimated:?} must upper-bound the create {actual:?}"
    );
    assert!(entries(&drive, &contract, "byPost", &like_again)
        .iter()
        .all(|(_, element)| element.is_some()));
    let trend_after_like_again = entries(&drive, &contract, "byTrendPost", &like_again);
    assert_eq!(
        trend_after_like_again, trend_before,
        "the kept entry is not written again"
    );
    for (path, _) in &trend_after_like_again {
        assert_eq!(
            member_count(&drive, path),
            1,
            "the liker counts once in the window"
        );
    }

    // And that like can be removed too, leaving the window as it is
    let mut unlike_again = like_again.clone();
    unlike_again.set_created_at(None);
    delete_like(&drive, &contract, unlike_again, true).expect("unlike again");
    assert!(entries(&drive, &contract, "byPost", &like_again)
        .iter()
        .all(|(_, element)| element.is_none()));
    assert_eq!(
        entries(&drive, &contract, "byTrendPost", &like_again),
        trend_before
    );

    assert_grovedb_is_consistent(&drive);
}

/// Without `outlivesDelete` the same window is cleared with the like, and a
/// delete must carry the like's `$createdAt` to find it.
#[test]
fn should_clear_a_window_that_does_not_outlive_deletes() {
    let (drive, contract) = setup_trending_likes(false);

    let like = like_at(&contract, T_MS, 1);
    insert_like(&drive, &contract, &like, true).expect("insert like");
    let mut without_created_at = like.clone();
    without_created_at.set_created_at(None);
    assert!(
        delete_like(&drive, &contract, without_created_at, true).is_err(),
        "the row commits to its $createdAt, which the delete must carry"
    );
    delete_like(&drive, &contract, like.clone(), true).expect("unlike");
    assert!(entries(&drive, &contract, "byTrendPost", &like)
        .iter()
        .all(|(_, element)| element.is_none()));
    assert_grovedb_is_consistent(&drive);
}

/// The layout describes the window's members as outliving deletes, and only
/// them.
#[test]
fn should_note_the_outliving_window_in_the_layout() {
    fn noted<'a>(node: &'a LayoutNode, found: &mut Vec<&'a [String]>) {
        if node.notes.contains(&LayoutNote::OutlivesDelete) {
            found.push(&node.indexes);
        }
        for child in &node.children {
            noted(child, found);
        }
    }
    let (_, contract) = setup_trending_likes(true);
    let layout = document_type_layout(
        contract
            .document_type_for_name("like")
            .expect("like doctype exists"),
        platform_version(),
    )
    .expect("the layout");
    let mut found = Vec::new();
    noted(&layout.root, &mut found);
    assert_eq!(found, vec![&["byTrendPost".to_string()][..]]);
}

/// When the rows commit to their timestamp (an index a delete clears is keyed
/// by it), a like again writes its own commitment over the entry the deleted
/// like left, and the liker still counts once.
#[test]
fn should_write_the_live_row_over_an_entry_a_deleted_one_left() {
    let (drive, contract) = setup_trending_likes_with(true, true);
    let like = like_at(&contract, T_MS, 1);
    insert_like(&drive, &contract, &like, true).expect("insert like");
    let trend_before = entries(&drive, &contract, "byTrendPost", &like);
    // The row commits to its timestamp, so the delete carries it
    delete_like(&drive, &contract, like.clone(), true).expect("unlike");
    assert_eq!(
        entries(&drive, &contract, "byTrendPost", &like),
        trend_before
    );

    let like_again = like_at(&contract, T_LATER_MS, 2);
    let estimated = insert_like(&drive, &contract, &like_again, false).expect("estimate");
    let actual = insert_like(&drive, &contract, &like_again, true).expect("like again");
    assert!(
        estimated.storage_fee >= actual.storage_fee
            && estimated.processing_fee >= actual.processing_fee,
        "the estimate {estimated:?} must upper-bound the create {actual:?}"
    );
    let trend_after = entries(&drive, &contract, "byTrendPost", &like_again);
    assert_ne!(
        trend_after, trend_before,
        "the entry now carries the live like's commitment"
    );
    for (path, element) in &trend_after {
        assert!(element.is_some());
        assert_eq!(member_count(&drive, path), 1, "the liker counts once");
    }
    assert_grovedb_is_consistent(&drive);
}

/// The like type counting a day's likes per hashtag over a range
/// (`byDayHashtag`), with `byDayHashtagPost` continuing below it in the same
/// daily window, kept for seven days, outliving deletes.
fn setup_daily_hashtags() -> (Drive, DataContract) {
    let mut schema = json_document_to_json_value(
        "tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json",
    )
    .expect("read contract fixture");
    let document_schemas = schema["documentSchemas"]
        .as_object_mut()
        .expect("document schemas");
    document_schemas.retain(|name, _| name == "like" || name == "post");
    let like = &mut document_schemas["like"];
    like["required"] = json!(["hashtag", "postId", "$createdAt"]);
    let window = json!({ "on": "$createdAt", "range": 86400, "step": 86400, "ttl": 604800 });
    like["indices"] = json!([
        { "name": "byPost", "properties": [{ "postId": "asc" }] },
        {
            "name": "byDayHashtag",
            "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }],
            "countable": "countable",
            "rangeCountable": true,
            "timeRange": window,
        },
        {
            "name": "byDayHashtagPost",
            "properties": [{ "$createdAt": "asc" }, { "hashtag": "asc" }, { "postId": "asc" }],
            "timeRange": window,
            "outlivesDelete": true,
        },
    ]);
    let contract = DataContract::try_from_platform_versioned(
        serde_json::from_value(schema).expect("contract serialization format"),
        true,
        &mut vec![],
        platform_version(),
    )
    .expect("parse the daily hashtags contract");
    drive_with(contract)
}

/// A day's hashtag whose last like is deleted stands at zero, since the
/// outliving index below it keeps its entry, and a range count over the
/// day's hashtags keeps it, unproved and proved alike: it took a place in
/// the page's limit, so leaving it out would end the page before the hashtag
/// that follows.
#[test]
fn should_keep_a_hashtag_an_outliving_window_holds_at_zero_on_a_page() {
    let (drive, contract) = setup_daily_hashtags();
    let pv = platform_version();
    let mut dash = build_like(&contract, "dash", POST_A, OWNER_1, 1);
    dash.set_created_at(Some(T_MS));
    let mut eth = build_like(&contract, "eth", POST_B, OWNER_1, 2);
    eth.set_created_at(Some(T_MS));
    insert_like(&drive, &contract, &dash, true).expect("insert the dash like");
    insert_like(&drive, &contract, &eth, true).expect("insert the eth like");
    delete_like(&drive, &contract, dash, true).expect("unlike dash");

    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let resolved = vec![ResolvedTimeRange {
        transform: TimeRangeTransform {
            source: "$createdAt".to_string(),
            range_seconds: 86400,
            step_seconds: 86400,
            phase_seconds: 0,
            ttl_seconds: Some(604800),
        }
        .into(),
    }];
    let where_clauses = vec![
        WhereClause {
            field: "$createdAt".to_string(),
            operator: WhereOperator::Equal,
            value: Value::U64(T_MS - T_MS % 86_400_000),
        },
        WhereClause {
            field: "hashtag".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::Text("a".to_string()),
        },
    ];
    let index = DriveDocumentCountQuery::find_range_countable_index_for_where_clauses(
        document_type.indexes(),
        &where_clauses,
        &resolved,
    )
    .expect("byDayHashtag answers the range");
    assert_eq!(index.name, "byDayHashtag");
    let query = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "like".to_string(),
        index,
        where_clauses,
    };

    let groups = [(b"dash".to_vec(), Some(0)), (b"eth".to_vec(), Some(1))];
    for limit in [1u16, 2] {
        let expected = groups[..limit as usize].to_vec();
        let unproved = query
            .execute_range_count_no_proof(
                &drive,
                &RangeCountOptions {
                    distinct: true,
                    limit: Some(limit.into()),
                    order_by_ascending: true,
                },
                None,
                pv,
            )
            .expect("the range count executes");
        assert_eq!(
            unproved
                .into_iter()
                .map(|entry| (entry.key, entry.count))
                .collect::<Vec<_>>(),
            expected,
            "unproved, limit {limit}"
        );
        let proof = query
            .execute_distinct_count_with_proof(&drive, limit, true, None, pv)
            .expect("the range count proves");
        let (root_hash, proved) = query
            .verify_distinct_count_proof(&proof, limit, true, pv)
            .expect("the range count proof verifies");
        assert_live_root_hash(&drive, root_hash);
        assert_eq!(
            proved
                .into_iter()
                .map(|entry| (entry.key, entry.count))
                .collect::<Vec<_>>(),
            expected,
            "proved, limit {limit}"
        );
    }
    assert_grovedb_is_consistent(&drive);
}
