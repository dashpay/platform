//! End-to-end coverage for **`skipIfAbsent` below the first position and on
//! stored document types** (PV14): an index skips a document that omits a
//! property of its skip set, wherever that property sits, and writes nothing
//! for it — no entry, and no tree of its own.
//!
//! Three fixtures under `tests/supporting_files/contract/skip-if-absent/`:
//!
//! - `skip-likes-contract.json`, an indexOnly `like` whose optional `hashtag`
//!   sits below a day window (`byDayHashtagPost`, sharing the day grid with
//!   the non-skip `byDayPost`) and below a rolling 24h/6h window that only a
//!   skip index uses (`byRollingHashtagPost`).
//! - `skip-posts-contract.json`, a stored `post` with an optional `hashtag`
//!   skipped on by `byHashtagLanguageTime` (a partial skip array: an absent
//!   `language` keeps the null key) and by the windowed ranked
//!   `byDayHashtag`, an optional `slug` under the unique skip index
//!   `bySlug`, and a plain `byLanguage` keeping the null layout. Beside it a
//!   stored `note` whose skip indexes share trees in pairs (the owner value
//!   tree, and a day window of one ttl grid).
//! - `skip-memos-contract.json`, a stored `memo` with a `ttl` and three deep
//!   skip indexes.
//!
//! Three things are pinned: what an insert writes (and leaves unbuilt), that
//! a delete restores the exact prior tree, and that a replace moving a stored
//! document into or out of a skip index leaves the same tree a fresh insert
//! of the new version would — with an estimate that bounds the applied fee.

use super::index_only_e2e_tests::{assert_grovedb_is_consistent, platform_version};
use crate::drive::{Drive, RootTree};
use crate::error::Error;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType;
use grovedb::{PathQuery, Query, SizedQuery};
use std::collections::{BTreeMap, BTreeSet};

const LIKES: &str = "tests/supporting_files/contract/skip-if-absent/skip-likes-contract.json";
const POSTS: &str = "tests/supporting_files/contract/skip-if-absent/skip-posts-contract.json";
const MEMOS: &str = "tests/supporting_files/contract/skip-if-absent/skip-memos-contract.json";

/// A day, 20000 days after the epoch, plus five hours: inside one day window
/// and four overlapping 24h/6h windows.
const CREATED_AT: u64 = 20_000 * 86_400_000 + 5 * 3_600_000;

const POST_A: [u8; 32] = [0xA1; 32];
pub(super) const OWNER_1: [u8; 32] = [0x11; 32];
const OWNER_2: [u8; 32] = [0x22; 32];
const OWNER_3: [u8; 32] = [0x33; 32];

pub(super) fn setup(path: &str) -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let contract = json_document_to_contract(path, true, platform_version())
        .expect("expected to parse the fixture");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version(),
        )
        .expect("expected to apply the fixture");
    (drive, contract)
}

/// `[DataContractDocuments, contract_id, 1, <doctype>]`.
pub(super) fn doctype_path(contract: &DataContract, doctype: &str) -> Vec<Vec<u8>> {
    vec![
        vec![RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        doctype.as_bytes().to_vec(),
    ]
}

/// Every element at or below `path`, as (path, key, element kind, count).
/// Kinds and counts, not whole elements: a merk's root key depends on the
/// order its keys were inserted in, and storage flags on when.
type Snapshot = BTreeSet<(Vec<Vec<u8>>, Vec<u8>, String, u64)>;

pub(super) fn snapshot(drive: &Drive, path: Vec<Vec<u8>>, skip_primary_key: bool) -> Snapshot {
    fn walk(drive: &Drive, path: Vec<Vec<u8>>, skip: Option<usize>, out: &mut Snapshot) {
        let mut query = Query::new();
        query.insert_all();
        let path_query = PathQuery::new(path.clone(), SizedQuery::new(query, None, None));
        let (elements, _) = drive
            .grove_get_raw_path_query(
                &path_query,
                None,
                QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version().drive,
            )
            .expect("expected to read a layer");
        for (key, element) in elements.to_key_elements() {
            if skip == Some(path.len()) && key == [0] {
                continue;
            }
            out.insert((
                path.clone(),
                key.clone(),
                element.type_str().to_string(),
                element.count_value_or_default(),
            ));
            if element.is_any_tree() {
                let mut below = path.clone();
                below.push(key);
                walk(drive, below, skip, out);
            }
        }
    }
    let skip = skip_primary_key.then_some(path.len());
    let mut out = Snapshot::new();
    walk(drive, path, skip, &mut out);
    out
}

/// The keys directly under `path`.
pub(super) fn keys_under(drive: &Drive, path: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut query = Query::new();
    query.insert_all();
    let path_query = PathQuery::new(path.to_vec(), SizedQuery::new(query, None, None));
    let (elements, _) = drive
        .grove_get_raw_path_query(
            &path_query,
            None,
            QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version().drive,
        )
        .expect("expected to read a layer");
    elements
        .to_key_elements()
        .into_iter()
        .map(|(key, _)| key)
        .collect()
}

pub(super) fn with_key(path: &[Vec<u8>], key: &[u8]) -> Vec<Vec<u8>> {
    let mut path = path.to_vec();
    path.push(key.to_vec());
    path
}

// ── indexOnly: the windowed like ────────────────────────────────────────

fn like(
    contract: &DataContract,
    hashtag: Option<&str>,
    post: [u8; 32],
    owner: [u8; 32],
    seed: u64,
) -> Document {
    let document_type = contract
        .document_type_for_name("like")
        .expect("like doctype exists");
    let mut document = document_type
        .random_document(Some(seed), platform_version())
        .expect("random like");
    let mut properties = BTreeMap::new();
    properties.insert("postId".to_string(), Value::Identifier(post));
    if let Some(hashtag) = hashtag {
        properties.insert("hashtag".to_string(), Value::Text(hashtag.to_string()));
    }
    document.set_properties(properties);
    document.set_owner_id(Identifier::from(owner));
    document.set_created_at(Some(CREATED_AT));
    document
}

fn insert_like(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    apply: bool,
) -> FeeResult {
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((document, None)),
                    owner_id: None,
                },
                contract,
                document_type: contract
                    .document_type_for_name("like")
                    .expect("like doctype exists"),
            },
            false,
            BlockInfo::default(),
            apply,
            None,
            platform_version(),
            None,
        )
        .expect("expected to insert the like")
}

fn delete_like(
    drive: &Drive,
    contract: &DataContract,
    document: Document,
    apply: bool,
) -> FeeResult {
    drive
        .delete_index_only_document_for_contract(
            document,
            contract,
            contract
                .document_type_for_name("like")
                .expect("like doctype exists"),
            BlockInfo::default(),
            apply,
            None,
            platform_version(),
            None,
        )
        .expect("expected to delete the like")
}

/// The single day window an insert at `CREATED_AT` lands in.
fn day_window(drive: &Drive, contract: &DataContract) -> Vec<Vec<u8>> {
    let grid = with_key(&doctype_path(contract, "like"), b"$createdAt#86400#86400");
    let windows = keys_under(drive, &grid);
    assert_eq!(windows.len(), 1, "one day window holds every like");
    with_key(&grid, &windows[0])
}

#[test]
fn should_build_no_skip_branch_for_an_untagged_like() {
    let (drive, contract) = setup(LIKES);
    let path = doctype_path(&contract, "like");

    insert_like(
        &drive,
        &contract,
        &like(&contract, None, POST_A, OWNER_1, 1),
        true,
    );

    // The shared day window is built for byDayPost, with no hashtag branch.
    let window = day_window(&drive, &contract);
    let under_window = keys_under(&drive, &window);
    assert!(
        under_window.contains(&b"postId".to_vec()),
        "byDayPost writes under the day window"
    );
    assert!(
        !under_window.contains(&b"hashtag".to_vec()),
        "byDayHashtagPost skips an untagged like: no hashtag branch, got {under_window:?}"
    );
    // Only a skip index uses the rolling grid: no window at all.
    assert!(
        keys_under(&drive, &with_key(&path, b"$createdAt#86400#21600")).is_empty(),
        "byRollingHashtagPost builds no window for an untagged like"
    );
    // byHashtagPost's registration-time tree stays empty.
    assert!(keys_under(&drive, &with_key(&path, b"hashtag")).is_empty());
    assert_grovedb_is_consistent(&drive);
}

#[test]
fn should_build_every_skip_branch_for_a_tagged_like() {
    let (drive, contract) = setup(LIKES);
    let path = doctype_path(&contract, "like");

    insert_like(
        &drive,
        &contract,
        &like(&contract, Some("dash"), POST_A, OWNER_1, 1),
        true,
    );

    let window = day_window(&drive, &contract);
    let under_window = keys_under(&drive, &window);
    assert!(under_window.contains(&b"hashtag".to_vec()));
    assert!(under_window.contains(&b"postId".to_vec()));
    assert_eq!(
        keys_under(&drive, &with_key(&path, b"$createdAt#86400#21600")).len(),
        4,
        "a 24h window every 6h holds the like in four windows"
    );
    assert_eq!(
        keys_under(&drive, &with_key(&path, b"hashtag")),
        vec![b"dash".to_vec()]
    );
    assert_grovedb_is_consistent(&drive);
}

#[test]
fn should_count_only_tagged_likes_under_a_windowed_skip_index() {
    let (drive, contract) = setup(LIKES);

    insert_like(
        &drive,
        &contract,
        &like(&contract, Some("dash"), POST_A, OWNER_1, 1),
        true,
    );
    insert_like(
        &drive,
        &contract,
        &like(&contract, Some("dash"), POST_A, OWNER_2, 2),
        true,
    );
    insert_like(
        &drive,
        &contract,
        &like(&contract, None, POST_A, OWNER_3, 3),
        true,
    );

    let window = day_window(&drive, &contract);
    let count_at = |path: Vec<Vec<u8>>, key: &[u8]| {
        snapshot(&drive, path.clone(), false)
            .into_iter()
            .find(|(element_path, element_key, _, _)| *element_path == path && element_key == key)
            .map(|(_, _, _, count)| count)
            .expect("the value tree exists")
    };
    assert_eq!(
        count_at(with_key(&window, b"postId"), &POST_A),
        3,
        "byDayPost counts every like"
    );
    let tag_post = with_key(
        &with_key(&with_key(&window, b"hashtag"), b"dash"),
        b"postId",
    );
    assert_eq!(
        count_at(tag_post, &POST_A),
        2,
        "byDayHashtagPost counts the tagged likes only"
    );
}

#[test]
fn should_restore_the_exact_tree_after_deleting_likes() {
    let (drive, contract) = setup(LIKES);
    let path = doctype_path(&contract, "like");
    let before = snapshot(&drive, path.clone(), false);

    let tagged = like(&contract, Some("dash"), POST_A, OWNER_1, 1);
    let untagged = like(&contract, None, POST_A, OWNER_2, 2);
    insert_like(&drive, &contract, &tagged, true);
    insert_like(&drive, &contract, &untagged, true);
    assert_ne!(snapshot(&drive, path.clone(), false), before);

    delete_like(&drive, &contract, untagged, true);
    delete_like(&drive, &contract, tagged, true);
    assert_eq!(
        snapshot(&drive, path, false),
        before,
        "deleting every like leaves exactly the registration-time tree"
    );
    assert_grovedb_is_consistent(&drive);
}

#[test]
fn should_estimate_at_least_the_applied_fee_for_likes() {
    for hashtag in [None, Some("dash")] {
        let (drive, contract) = setup(LIKES);
        let document = like(&contract, hashtag, POST_A, OWNER_1, 1);
        let estimated = insert_like(&drive, &contract, &document, false);
        let applied = insert_like(&drive, &contract, &document, true);
        assert!(
            estimated.total_base_fee() >= applied.total_base_fee(),
            "insert, hashtag {hashtag:?}: estimated {estimated:?} applied {applied:?}"
        );
        let estimated = delete_like(&drive, &contract, document.clone(), false);
        let applied = delete_like(&drive, &contract, document, true);
        assert!(
            estimated.total_base_fee() >= applied.total_base_fee(),
            "delete, hashtag {hashtag:?}: estimated {estimated:?} applied {applied:?}"
        );
    }
}

// ── stored: the post ────────────────────────────────────────────────────

fn post(
    contract: &DataContract,
    id: [u8; 32],
    hashtag: Option<&str>,
    language: Option<&str>,
    slug: Option<&str>,
    text: &str,
) -> Document {
    let document_type = contract
        .document_type_for_name("post")
        .expect("post doctype exists");
    let mut document = document_type
        .random_document(Some(id[0] as u64), platform_version())
        .expect("random post");
    let mut properties = BTreeMap::new();
    properties.insert("text".to_string(), Value::Text(text.to_string()));
    for (name, value) in [("hashtag", hashtag), ("language", language), ("slug", slug)] {
        if let Some(value) = value {
            properties.insert(name.to_string(), Value::Text(value.to_string()));
        }
    }
    document.set_properties(properties);
    document.set_id(Identifier::from(id));
    document.set_owner_id(Identifier::from(OWNER_1));
    document.set_created_at(Some(CREATED_AT));
    document.set_updated_at(None);
    document.set_revision(Some(1));
    document
}

fn insert_post(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    apply: bool,
) -> Result<FeeResult, Error> {
    insert_document(drive, contract, "post", document, apply, platform_version())
}

fn replace_post(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    apply: bool,
) -> FeeResult {
    replace_document(drive, contract, "post", document, apply, platform_version())
        .expect("expected to replace the post")
}

pub(super) fn insert_document(
    drive: &Drive,
    contract: &DataContract,
    doctype: &str,
    document: &Document,
    apply: bool,
    platform_version: &PlatformVersion,
) -> Result<FeeResult, Error> {
    drive.add_document_for_contract(
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((document, StorageFlags::optional_default_as_cow())),
                owner_id: None,
            },
            contract,
            document_type: contract
                .document_type_for_name(doctype)
                .expect("doctype exists"),
        },
        false,
        BlockInfo::default(),
        apply,
        None,
        platform_version,
        None,
    )
}

pub(super) fn replace_document(
    drive: &Drive,
    contract: &DataContract,
    doctype: &str,
    document: &Document,
    apply: bool,
    platform_version: &PlatformVersion,
) -> Result<FeeResult, Error> {
    drive.update_document_for_contract(
        document,
        contract,
        contract
            .document_type_for_name(doctype)
            .expect("doctype exists"),
        Some(document.owner_id().to_buffer()),
        BlockInfo::default(),
        apply,
        StorageFlags::optional_default_as_cow(),
        None,
        platform_version,
        None,
    )
}

/// A document of the stored `doctype` carrying the text `properties`, owned
/// by `OWNER_1` and created at `CREATED_AT`.
fn stored_document(
    contract: &DataContract,
    doctype: &str,
    id: [u8; 32],
    properties: &[(&str, &str)],
) -> Document {
    stored_document_with(
        contract,
        doctype,
        id,
        properties
            .iter()
            .map(|(name, value)| (*name, Value::Text(value.to_string())))
            .collect(),
        Some(CREATED_AT),
    )
}

/// A document of the stored `doctype` carrying `properties`, owned by
/// `OWNER_1` and created at `created_at`.
pub(super) fn stored_document_with(
    contract: &DataContract,
    doctype: &str,
    id: [u8; 32],
    properties: Vec<(&str, Value)>,
    created_at: Option<u64>,
) -> Document {
    let document_type = contract
        .document_type_for_name(doctype)
        .expect("doctype exists");
    let mut document = document_type
        .random_document(Some(id[0] as u64), platform_version())
        .expect("random document");
    document.set_properties(
        properties
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
    );
    document.set_id(Identifier::from(id));
    document.set_owner_id(Identifier::from(OWNER_1));
    document.set_created_at(created_at);
    document.set_updated_at(None);
    document.set_revision(Some(1));
    document
}

pub(super) fn delete_document(drive: &Drive, contract: &DataContract, doctype: &str, id: [u8; 32]) {
    drive
        .delete_document_for_contract(
            Identifier::from(id),
            contract,
            doctype,
            BlockInfo::default(),
            true,
            None,
            platform_version(),
            None,
        )
        .expect("expected to delete the document");
}

#[test]
fn should_skip_an_untagged_post_and_keep_the_null_key_for_a_missing_language() {
    let (drive, contract) = setup(POSTS);
    let path = doctype_path(&contract, "post");

    insert_post(
        &drive,
        &contract,
        &post(&contract, [1; 32], None, None, None, "no tag"),
        true,
    )
    .expect("insert untagged post");
    assert!(
        keys_under(&drive, &with_key(&path, b"hashtag")).is_empty(),
        "byHashtagLanguageTime skips an untagged post"
    );
    assert!(
        keys_under(&drive, &with_key(&path, b"$createdAt#86400#86400")).is_empty(),
        "byDayHashtag, the grid's only index, builds no window for it"
    );
    assert!(
        keys_under(&drive, &with_key(&path, b"slug")).is_empty(),
        "bySlug skips a post without a slug"
    );
    assert_eq!(
        keys_under(&drive, &with_key(&path, b"language")),
        vec![Vec::<u8>::new()],
        "byLanguage does not skip: a missing language is written under the null key"
    );

    insert_post(
        &drive,
        &contract,
        &post(&contract, [2; 32], Some("dash"), None, None, "tagged"),
        true,
    )
    .expect("insert tagged post without a language");
    let tag = with_key(&with_key(&path, b"hashtag"), b"dash");
    assert_eq!(
        keys_under(&drive, &with_key(&tag, b"language")),
        vec![Vec::<u8>::new()],
        "language is not in the skip set: a tagged post without one takes the null key"
    );
    assert_eq!(
        keys_under(&drive, &with_key(&path, b"$createdAt#86400#86400")).len(),
        1
    );
    assert_grovedb_is_consistent(&drive);
}

#[test]
fn should_refuse_a_duplicate_slug_only_among_posts_carrying_one() {
    let (drive, contract) = setup(POSTS);
    insert_post(
        &drive,
        &contract,
        &post(&contract, [1; 32], None, None, None, "one"),
        true,
    )
    .expect("a post without a slug");
    insert_post(
        &drive,
        &contract,
        &post(&contract, [2; 32], None, None, None, "two"),
        true,
    )
    .expect("a second post without a slug: the unique index skipped both");
    insert_post(
        &drive,
        &contract,
        &post(&contract, [3; 32], None, None, Some("hello"), "three"),
        true,
    )
    .expect("a post with a slug");
    assert!(
        insert_post(
            &drive,
            &contract,
            &post(&contract, [4; 32], None, None, Some("hello"), "four"),
            true,
        )
        .is_err(),
        "a second post with the same slug collides"
    );
}

#[test]
fn should_delete_posts_back_to_the_prior_tree() {
    let (drive, contract) = setup(POSTS);
    let path = doctype_path(&contract, "post");
    let before = snapshot(&drive, path.clone(), false);

    insert_post(
        &drive,
        &contract,
        &post(&contract, [1; 32], None, Some("en"), None, "no tag"),
        true,
    )
    .expect("insert");
    insert_post(
        &drive,
        &contract,
        &post(&contract, [2; 32], Some("dash"), None, Some("s"), "tagged"),
        true,
    )
    .expect("insert");
    delete_document(&drive, &contract, "post", [1; 32]);
    delete_document(&drive, &contract, "post", [2; 32]);
    assert_eq!(snapshot(&drive, path, false), before);
    assert_grovedb_is_consistent(&drive);
}

/// A replace leaves the index trees a fresh insert of the new version would
/// have built — moving the post into a skip index, out of it, or within it —
/// and its estimate bounds its applied fee.
#[test]
fn should_leave_the_tree_of_a_fresh_insert_after_each_replace() {
    type Version = (
        Option<&'static str>,
        Option<&'static str>,
        Option<&'static str>,
        &'static str,
    );
    let cases: [(&str, Version, Version); 6] = [
        (
            "untagged to tagged",
            (None, Some("en"), None, "a"),
            (Some("dash"), Some("en"), None, "a"),
        ),
        (
            "tagged to untagged",
            (Some("dash"), None, None, "a"),
            (None, None, None, "a"),
        ),
        (
            "tag change",
            (Some("dash"), Some("en"), None, "a"),
            (Some("evo"), Some("en"), None, "a"),
        ),
        (
            "untagged text change",
            (None, Some("en"), None, "a"),
            (None, Some("en"), None, "b"),
        ),
        (
            "slug added",
            (Some("dash"), None, None, "a"),
            (Some("dash"), None, Some("s"), "a"),
        ),
        (
            "slug removed",
            (None, None, Some("s"), "a"),
            (None, None, None, "a"),
        ),
    ];
    for (case, old, new) in cases {
        let (replaced_drive, contract) = setup(POSTS);
        let id = [7; 32];
        let old_post = post(&contract, id, old.0, old.1, old.2, old.3);
        insert_post(&replaced_drive, &contract, &old_post, true).expect("insert old");
        let mut new_post = post(&contract, id, new.0, new.1, new.2, new.3);
        new_post.set_revision(Some(2));
        let estimated = replace_post(&replaced_drive, &contract, &new_post, false);
        let applied = replace_post(&replaced_drive, &contract, &new_post, true);
        assert!(
            estimated.total_base_fee() >= applied.total_base_fee(),
            "{case}: estimated {estimated:?} applied {applied:?}"
        );

        let (fresh_drive, fresh_contract) = setup(POSTS);
        insert_post(&fresh_drive, &fresh_contract, &new_post, true).expect("insert new");

        assert_eq!(
            snapshot(&replaced_drive, doctype_path(&contract, "post"), true),
            snapshot(&fresh_drive, doctype_path(&fresh_contract, "post"), true),
            "{case}: the replace left other index trees than a fresh insert"
        );
        assert_grovedb_is_consistent(&replaced_drive);
    }
}

/// A replace moving a note out of one skip index and into another sharing a
/// tree with it (the owner value tree of `byOwnerTag` and `byOwnerTopic`, and
/// the day window of the ttl grid `byDayTag` and `byDayTopic` share) applies,
/// in both directions, and leaves the tree a fresh insert would: the index
/// the note leaves prunes after the one it enters has written, so the prune
/// stops below the shared tree.
#[test]
fn should_move_a_note_between_skip_indexes_sharing_a_tree() {
    let tagged: &[(&str, &str)] = &[("tag", "x"), ("text", "a")];
    let with_topic: &[(&str, &str)] = &[("topic", "y"), ("text", "a")];
    for (case, old, new) in [
        ("tag to topic", tagged, with_topic),
        ("topic to tag", with_topic, tagged),
    ] {
        let (replaced_drive, contract) = setup(POSTS);
        let id = [9; 32];
        insert_document(
            &replaced_drive,
            &contract,
            "note",
            &stored_document(&contract, "note", id, old),
            true,
            platform_version(),
        )
        .expect("insert old");
        let mut new_note = stored_document(&contract, "note", id, new);
        new_note.set_revision(Some(2));
        let estimated = replace_document(
            &replaced_drive,
            &contract,
            "note",
            &new_note,
            false,
            platform_version(),
        )
        .expect("estimate the replace");
        let applied = replace_document(
            &replaced_drive,
            &contract,
            "note",
            &new_note,
            true,
            platform_version(),
        )
        .unwrap_or_else(|error| panic!("{case}: the replace did not apply: {error:?}"));
        assert!(
            estimated.total_base_fee() >= applied.total_base_fee(),
            "{case}: estimated {estimated:?} applied {applied:?}"
        );

        let (fresh_drive, fresh_contract) = setup(POSTS);
        insert_document(
            &fresh_drive,
            &fresh_contract,
            "note",
            &new_note,
            true,
            platform_version(),
        )
        .expect("insert new");
        assert_eq!(
            snapshot(&replaced_drive, doctype_path(&contract, "note"), true),
            snapshot(&fresh_drive, doctype_path(&fresh_contract, "note"), true),
            "{case}: the replace left other index trees than a fresh insert"
        );
        assert_grovedb_is_consistent(&replaced_drive);
    }
}

/// On a type with a `ttl` nothing is refunded and elements carry no flags,
/// so a replace dropping `d` pays for deleting and pruning the entries of
/// the three deep skip indexes it leaves. Its estimate, an insert of the new
/// version, still covers what it applies.
#[test]
fn should_estimate_at_least_the_applied_fee_when_a_memo_leaves_its_skip_indexes() {
    let with_d: &[(&str, &str)] = &[("a", "a"), ("b", "b"), ("c", "c"), ("d", "d")];
    let without_d: &[(&str, &str)] = &[("a", "a"), ("b", "b"), ("c", "c")];
    for (case, old, new) in [
        ("d dropped", with_d, without_d),
        ("d added", without_d, with_d),
    ] {
        let (drive, contract) = setup(MEMOS);
        let id = [5; 32];
        insert_document(
            &drive,
            &contract,
            "memo",
            &stored_document(&contract, "memo", id, old),
            true,
            platform_version(),
        )
        .expect("insert old");
        let mut new_memo = stored_document(&contract, "memo", id, new);
        new_memo.set_revision(Some(2));
        let estimated = replace_document(
            &drive,
            &contract,
            "memo",
            &new_memo,
            false,
            platform_version(),
        )
        .expect("estimate the replace");
        let applied = replace_document(
            &drive,
            &contract,
            "memo",
            &new_memo,
            true,
            platform_version(),
        )
        .expect("apply the replace");
        assert!(
            estimated.total_base_fee() >= applied.total_base_fee(),
            "{case}: estimated {estimated:?} applied {applied:?}"
        );
        assert_grovedb_is_consistent(&drive);
    }
}
