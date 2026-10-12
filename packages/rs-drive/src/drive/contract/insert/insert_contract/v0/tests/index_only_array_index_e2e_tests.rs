//! End-to-end coverage for an **indexOnly type's index over the elements of a
//! typed array** (protocol version 14): a `like` carrying its post's `tags`
//! keeps one entry per tag under `byTagPost` and, in each window, under the
//! trending `byTrendTagPost`, while `byPost` keeps one entry per like and is
//! what refuses a second like of the same post by the same owner.
//!
//! The row commitment binds the whole tag list in order, so a delete carrying
//! the tags reordered, shortened or changed fails every probe; a like without
//! tags (absent, null or empty) is in no tag index; the entries of an
//! expired window drain with it; the dry runs upper-bound the applied fees
//! with the fan-out; and element reads and counts prove.

use super::index_only_e2e_tests::{
    assert_grovedb_is_consistent, assert_live_root_hash, platform_version, read_grove_element,
};
use crate::config::DriveConfig;
use crate::drive::document::{
    index_only_row_commitment, index_only_row_commitment_max_preimage_size,
    index_only_row_commitment_with_preimage_size,
};
use crate::drive::{Drive, RootTree};
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::{FunctionOp, HashFunction, LowLevelDriveOperation};
use crate::query::index_only_synthesis::{
    index_only_proof_index, index_only_transition_entry_path_query,
};
use crate::query::{
    key_array_element_clauses, CountMode, DocumentCountRequest, DocumentCountResponse,
    DriveDocumentCountQuery, DriveDocumentQuery, ResolvedTimeRange, WhereClause, WhereOperator,
};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{
    index_only_row_commits_created_at, DocumentPropertyType, DocumentTypeRef, Index,
};
use dpp::data_contract::DataContractFactory;
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use grovedb::{Element, GroveDb, TransactionArg};
use std::collections::{BTreeMap, HashMap};

const POST_A: [u8; 32] = [0xA1; 32];
const POST_B: [u8; 32] = [0xB2; 32];
const AUTHOR: [u8; 32] = [0xAA; 32];
const OWNER_1: [u8; 32] = [0x11; 32];
const OWNER_2: [u8; 32] = [0x22; 32];
/// A like's time.
const T_MS: u64 = 1_750_000_000_000;
const DAY_MS: u64 = 86_400_000;

/// What the trending tag index of the like type is.
#[derive(Clone, Copy, Debug)]
enum Trend {
    /// No trending index.
    None,
    /// `byTrendTagPost`, whose entries outlive a delete.
    Outliving,
    /// `byTrendTagPost`, cleared by a delete like every other index.
    Cleared,
    /// `byTrendTagPost`, whose entries outlive a delete, as the only index
    /// involving `$createdAt`: the author's index is `byAuthorPost`, without
    /// it, so the row commits to no timestamp and a delete carries none.
    OutlivingOnlyTimed,
}

/// The like type: `byPost` (one like per post and owner), `byTagPost` (an
/// entry per tag) and `byAuthorPostTime` (holding the post's author), plus
/// the trending `byTrendTagPost` (a day's window every 6 hours, kept a week)
/// as `trend` says.
fn like_contract(trend: Trend) -> DataContract {
    like_contract_with(trend, "byTagPost")
}

/// [`like_contract`] with the per-tag index named `tag_index`.
fn like_contract_with(trend: Trend, tag_index: &str) -> DataContract {
    let identifier = |position: u32| {
        platform_value!({
            "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": position
        })
    };
    let mut indices = vec![
        platform_value!({
            "name": "byPost", "properties": [{ "postId": "asc" }], "terminal": "$ownerId",
            "rangeCountable": true
        }),
        platform_value!({
            "name": tag_index, "properties": [{ "tags": "asc" }, { "postId": "asc" }],
            "countable": "countable", "rangeCountable": true
        }),
        match trend {
            Trend::OutlivingOnlyTimed => platform_value!({
                "name": "byAuthorPost",
                "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }],
                "terminal": "$ownerId"
            }),
            _ => platform_value!({
                "name": "byAuthorPostTime",
                "properties": [{ "postAuthor": "asc" }, { "postId": "asc" }, { "$createdAt": "asc" }],
                "terminal": "$ownerId"
            }),
        },
    ];
    let outlives = match trend {
        Trend::None => None,
        Trend::Outliving | Trend::OutlivingOnlyTimed => Some(true),
        Trend::Cleared => Some(false),
    };
    if let Some(outlives) = outlives {
        indices.push(platform_value!({
            "name": "byTrendTagPost",
            "properties": [{ "$createdAt": "asc" }, { "tags": "asc" }, { "postId": "asc" }],
            "terminal": "$ownerId",
            "rangeCountable": true,
            "timeRange": { "on": "$createdAt", "range": 86400, "step": 21600, "ttl": 604800 },
            "outlivesDelete": outlives
        }));
    }
    DataContractFactory::new(platform_version().protocol_version)
        .expect("factory")
        .create_with_value_config(
            Identifier::from([1; 32]),
            0,
            platform_value!({ "like": {
                "type": "object",
                "indexOnly": true,
                "documentsMutable": false,
                "canBeDeleted": true,
                "properties": {
                    "postId": identifier(0),
                    "tags": {
                        "type": "array",
                        "items": { "type": "string", "maxLength": 61 },
                        "maxItems": 4,
                        "uniqueItems": true,
                        "position": 1
                    },
                    "postAuthor": identifier(2)
                },
                "required": ["$createdAt", "postId", "postAuthor"],
                "indices": Value::Array(indices),
                "additionalProperties": false
            }}),
            None,
            None,
        )
        .expect("the like contract parses")
        .data_contract_owned()
}

fn setup(trend: Trend) -> (Drive, DataContract) {
    setup_with(like_contract(trend))
}

fn setup_with(contract: DataContract) -> (Drive, DataContract) {
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
        .expect("apply the like contract");
    (drive, contract)
}

fn like_type(contract: &DataContract) -> DocumentTypeRef<'_> {
    contract
        .document_types()
        .get("like")
        .expect("the like type")
        .as_ref()
}

fn index<'a>(contract: &'a DataContract, name: &str) -> &'a Index {
    contract
        .document_types()
        .get("like")
        .expect("the like type")
        .indexes()
        .get(name)
        .expect("the index exists")
}

/// A like of `post` by `owner` at `created_at`, carrying `tags` as given: a
/// list of them, or no `tags` at all.
fn like(post: [u8; 32], owner: [u8; 32], tags: Option<&[&str]>, created_at: u64) -> Document {
    let mut properties = BTreeMap::from([
        ("postId".to_string(), Value::Identifier(post)),
        ("postAuthor".to_string(), Value::Identifier(AUTHOR)),
    ]);
    if let Some(tags) = tags {
        properties.insert("tags".to_string(), tags_value(tags));
    }
    let mut id = post;
    id[..16].copy_from_slice(&owner[..16]);
    Document::V0(DocumentV0 {
        id: Identifier::from(id),
        owner_id: Identifier::from(owner),
        properties,
        created_at: Some(created_at),
        ..Default::default()
    })
}

fn tags_value(tags: &[&str]) -> Value {
    Value::Array(
        tags.iter()
            .map(|tag| Value::Text(tag.to_string()))
            .collect(),
    )
}

/// `document` carrying `tags` instead.
fn with_tags(document: &Document, tags: Value) -> Document {
    let mut document = document.clone();
    document.set("tags", tags);
    document
}

fn block_at(time_ms: u64) -> BlockInfo {
    BlockInfo {
        time_ms,
        ..Default::default()
    }
}

fn insert_at(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    time_ms: u64,
    apply: bool,
    transaction: TransactionArg,
) -> Result<FeeResult, Error> {
    drive.add_document_for_contract(
        DocumentAndContractInfo {
            owned_document_info: OwnedDocumentInfo {
                document_info: DocumentRefInfo((document, StorageFlags::optional_default_as_cow())),
                owner_id: None,
            },
            contract,
            document_type: like_type(contract),
        },
        false,
        block_at(time_ms),
        apply,
        transaction,
        platform_version(),
        None,
    )
}

fn insert(drive: &Drive, contract: &DataContract, document: &Document, apply: bool) -> FeeResult {
    insert_at(drive, contract, document, T_MS, apply, None).expect("the like inserts")
}

fn delete_at(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    time_ms: u64,
    apply: bool,
    transaction: TransactionArg,
) -> Result<FeeResult, Error> {
    drive.delete_index_only_document_for_contract(
        document.clone(),
        contract,
        like_type(contract),
        block_at(time_ms),
        apply,
        transaction,
        platform_version(),
        None,
    )
}

fn delete(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    apply: bool,
) -> Result<FeeResult, Error> {
    delete_at(drive, contract, document, T_MS, apply, None)
}

/// `[DataContractDocuments, contract, 1, "like"]` followed by `segments`.
fn like_path(contract: &DataContract, segments: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut path = vec![
        vec![RootTree::DataContractDocuments as u8],
        contract.id().to_vec(),
        vec![1],
        b"like".to_vec(),
    ];
    path.extend(segments.iter().cloned());
    path
}

fn text(value: &str) -> Vec<u8> {
    value.as_bytes().to_vec()
}

/// The entry `owner` holds under `segments` (below the like type), followed by
/// the `0` member tree.
fn entry(
    drive: &Drive,
    contract: &DataContract,
    segments: &[Vec<u8>],
    owner: [u8; 32],
) -> Option<Element> {
    let mut segments = segments.to_vec();
    segments.push(vec![0]);
    read_grove_element(drive, &like_path(contract, &segments), &owner)
}

/// `byTagPost`'s entry of `owner`'s like of `post` under `tag`.
fn tag_entry(
    drive: &Drive,
    contract: &DataContract,
    tag: &str,
    post: [u8; 32],
    owner: [u8; 32],
) -> Option<Element> {
    entry(
        drive,
        contract,
        &[text("tags"), text(tag), text("postId"), post.to_vec()],
        owner,
    )
}

/// `byPost`'s entry of `owner`'s like of `post`.
fn post_entry(
    drive: &Drive,
    contract: &DataContract,
    post: [u8; 32],
    owner: [u8; 32],
) -> Option<Element> {
    entry(drive, contract, &[text("postId"), post.to_vec()], owner)
}

/// The windows of `byTrendTagPost` a like at `created_at` falls in.
fn trend_windows(contract: &DataContract, created_at: u64) -> Vec<Vec<u8>> {
    index(contract, "byTrendTagPost")
        .bucketing()
        .expect("a window")
        .entry_keys_for_raw(&DocumentPropertyType::encode_date_timestamp(created_at))
}

fn trend_level_key(contract: &DataContract) -> Vec<u8> {
    index(contract, "byTrendTagPost")
        .time_range
        .as_ref()
        .expect("a window")
        .storage_key("$createdAt")
        .into_bytes()
}

/// `byTrendTagPost`'s entry of `owner`'s like of `post` under `tag` in the
/// window starting at `window`.
fn trend_entry(
    drive: &Drive,
    contract: &DataContract,
    window: &[u8],
    tag: &str,
    post: [u8; 32],
    owner: [u8; 32],
) -> Option<Element> {
    entry(
        drive,
        contract,
        &[
            trend_level_key(contract),
            window.to_vec(),
            text("tags"),
            text(tag),
            text("postId"),
            post.to_vec(),
        ],
        owner,
    )
}

fn commitment(contract: &DataContract, document: &Document) -> [u8; 32] {
    index_only_row_commitment(document, like_type(contract), platform_version())
        .expect("the row commitment computes")
}

/// Whether `element` is an entry carrying `commitment`.
fn carries(element: Option<Element>, commitment: &[u8; 32]) -> bool {
    matches!(element, Some(Element::Item(payload, _)) if payload.get(..32) == Some(commitment.as_slice()))
}

fn is_not_found(result: Result<FeeResult, Error>) -> bool {
    matches!(
        result,
        Err(Error::Drive(DriveError::DeletingDocumentThatDoesNotExist(
            _
        )))
    )
}

// ---------------------------------------------------------------------------
// Entries
// ---------------------------------------------------------------------------

/// A like writes one entry per tag under `byTagPost`, one per tag in each of
/// its windows under `byTrendTagPost`, and one under `byPost`, every one
/// carrying the like's row commitment; the paths the probes read are exactly
/// those.
#[test]
fn should_write_one_entry_per_element_under_each_array_index() {
    for trend in [Trend::Outliving, Trend::Cleared] {
        let (drive, contract) = setup(trend);
        let tags = ["dash", "evo", "news"];
        let like = like(POST_A, OWNER_1, Some(&tags), T_MS);
        insert(&drive, &contract, &like, true);
        let commitment = commitment(&contract, &like);

        assert!(carries(
            post_entry(&drive, &contract, POST_A, OWNER_1),
            &commitment
        ));
        let windows = trend_windows(&contract, T_MS);
        assert_eq!(windows.len(), 4, "a day's window every 6 hours");
        for tag in tags {
            assert!(
                carries(
                    tag_entry(&drive, &contract, tag, POST_A, OWNER_1),
                    &commitment
                ),
                "{trend:?}: byTagPost holds {tag}"
            );
            for window in windows.iter() {
                assert!(
                    carries(
                        trend_entry(&drive, &contract, window, tag, POST_A, OWNER_1),
                        &commitment
                    ),
                    "{trend:?}: byTrendTagPost holds {tag} in each window"
                );
            }
        }
        assert!(tag_entry(&drive, &contract, "other", POST_A, OWNER_1).is_none());

        // The probes read the paths the walkers wrote: one per tag, and one
        // per window and tag.
        let paths = |name: &str| {
            Drive::index_only_entry_paths_and_key(
                contract.id(),
                like_type(&contract),
                index(&contract, name),
                &like,
                platform_version(),
            )
            .expect("the entry paths")
            .0
            .len()
        };
        assert_eq!(paths("byPost"), 1);
        assert_eq!(paths("byTagPost"), 3);
        assert_eq!(paths("byTrendTagPost"), 12);
        assert_grovedb_is_consistent(&drive);
    }
}

/// `byPost` keeps one like per post and owner: a second like of the post by
/// the same owner collides there whatever its tags, which is why the tag
/// indexes are no uniqueness rule of their own. Another owner's like sharing
/// every tag writes entries of its own.
#[test]
fn should_refuse_a_second_like_of_a_post_by_its_owner_whatever_its_tags() {
    let (drive, contract) = setup(Trend::Outliving);
    let first = like(POST_A, OWNER_1, Some(&["dash", "evo"]), T_MS);
    insert(&drive, &contract, &first, true);

    let probe = |document: &Document, name: &str| {
        drive
            .has_index_only_document_entry(
                contract.id(),
                like_type(&contract),
                index(&contract, name),
                document,
                None,
                &mut vec![],
                platform_version(),
            )
            .expect("the probe reads")
    };
    // Disjoint tags: only byPost finds the earlier like
    let other_tags = like(POST_A, OWNER_1, Some(&["news"]), T_MS + 1);
    assert!(probe(&other_tags, "byPost"));
    assert!(!probe(&other_tags, "byTagPost"));
    let error = insert_at(&drive, &contract, &other_tags, T_MS + 1, true, None)
        .expect_err("the second like of the post is refused");
    assert!(
        matches!(error, Error::Drive(DriveError::CorruptedContractIndexes(_))),
        "{error}"
    );
    // No tags at all: refused the same way
    let untagged = like(POST_A, OWNER_1, None, T_MS + 2);
    assert!(probe(&untagged, "byPost"));
    assert!(insert_at(&drive, &contract, &untagged, T_MS + 2, true, None).is_err());

    // Another owner's like with the same tags is a like of its own
    let second = like(POST_A, OWNER_2, Some(&["dash", "evo"]), T_MS);
    assert!(!probe(&second, "byPost") && !probe(&second, "byTagPost"));
    insert(&drive, &contract, &second, true);
    for tag in ["dash", "evo"] {
        assert!(tag_entry(&drive, &contract, tag, POST_A, OWNER_1).is_some());
        assert!(tag_entry(&drive, &contract, tag, POST_A, OWNER_2).is_some());
    }
    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Deletes
// ---------------------------------------------------------------------------

/// A delete removes the like's entry under every tag, prunes a tag no other
/// like holds, and keeps the entries of another like sharing a tag.
#[test]
fn should_remove_every_element_entry_on_delete() {
    let (drive, contract) = setup(Trend::Cleared);
    let first = like(
        POST_A,
        OWNER_1,
        Some(&["dash", "evo", "news", "solo"]),
        T_MS,
    );
    let second = like(POST_B, OWNER_2, Some(&["dash"]), T_MS);
    insert(&drive, &contract, &first, true);
    insert(&drive, &contract, &second, true);

    delete(&drive, &contract, &first, true).expect("the like deletes");
    for tag in ["dash", "evo", "news", "solo"] {
        assert!(tag_entry(&drive, &contract, tag, POST_A, OWNER_1).is_none());
        for window in trend_windows(&contract, T_MS) {
            assert!(trend_entry(&drive, &contract, &window, tag, POST_A, OWNER_1).is_none());
        }
    }
    assert!(post_entry(&drive, &contract, POST_A, OWNER_1).is_none());
    // A tag no other like holds is pruned; one another like holds stays
    assert!(read_grove_element(&drive, &like_path(&contract, &[text("tags")]), b"solo").is_none());
    assert!(tag_entry(&drive, &contract, "dash", POST_B, OWNER_2).is_some());
    assert_grovedb_is_consistent(&drive);
}

/// The row commitment binds the whole tag list in its order: a delete
/// carrying the tags reordered, shortened, lengthened, changed or left out
/// matches no entry, `byPost`'s included, and removes nothing; the list as
/// created deletes.
#[test]
fn should_refuse_a_delete_carrying_other_tags() {
    let (drive, contract) = setup(Trend::Cleared);
    let created = like(POST_A, OWNER_1, Some(&["dash", "evo", "news"]), T_MS);
    insert(&drive, &contract, &created, true);
    let expected = commitment(&contract, &created);

    for tags in [
        tags_value(&["evo", "dash", "news"]),
        tags_value(&["dash", "evo"]),
        tags_value(&["dash", "evo", "news", "solo"]),
        tags_value(&["dash", "evo", "solo"]),
        tags_value(&[]),
        Value::Null,
    ] {
        let carried = with_tags(&created, tags.clone());
        assert_ne!(commitment(&contract, &carried), expected, "{tags:?}");
        // Even the index the tags are no part of refuses the values
        assert!(!drive
            .index_only_entry_commitment_matches(
                contract.id(),
                like_type(&contract),
                index(&contract, "byPost"),
                &carried,
                &commitment(&contract, &carried),
                T_MS,
                None,
                &mut vec![],
                platform_version(),
            )
            .expect("the probe reads"));
        assert!(
            is_not_found(delete(&drive, &contract, &carried, true)),
            "a delete carrying {tags:?} is refused"
        );
    }
    let mut untagged = created.clone();
    untagged.properties_mut().remove("tags");
    assert!(is_not_found(delete(&drive, &contract, &untagged, true)));

    // Nothing was removed, and the tags as created delete
    assert!(carries(
        post_entry(&drive, &contract, POST_A, OWNER_1),
        &expected
    ));
    for tag in ["dash", "evo", "news"] {
        assert!(carries(
            tag_entry(&drive, &contract, tag, POST_A, OWNER_1),
            &expected
        ));
    }
    delete(&drive, &contract, &created, true).expect("the tags as created delete");
    assert!(post_entry(&drive, &contract, POST_A, OWNER_1).is_none());
    assert_grovedb_is_consistent(&drive);
}

/// A like without tags, absent, null or empty, is in no tag index and in
/// every other one. The three forms write the same entries, so the row
/// commits them alike and a delete may carry any of them.
#[test]
fn should_enter_a_like_without_tags_in_no_tag_index() {
    let (drive, contract) = setup(Trend::Outliving);
    let absent = like(POST_A, OWNER_1, None, T_MS);
    let empty = like(POST_B, OWNER_1, Some(&[]), T_MS);
    let null = with_tags(&like(POST_A, OWNER_2, None, T_MS), Value::Null);
    for document in [&absent, &empty, &null] {
        insert(&drive, &contract, document, true);
        for name in ["byTagPost", "byTrendTagPost"] {
            assert!(Drive::index_only_entry_paths_and_key(
                contract.id(),
                like_type(&contract),
                index(&contract, name),
                document,
                platform_version(),
            )
            .expect("the entry paths")
            .0
            .is_empty());
        }
    }
    assert!(
        read_grove_element(&drive, &like_path(&contract, &[]), b"tags").is_some(),
        "the tag index's top tree stands, empty"
    );
    assert!(drive
        .grove
        .is_empty_tree(
            like_path(&contract, &[text("tags")])
                .iter()
                .map(Vec::as_slice)
                .collect::<Vec<_>>()
                .as_slice(),
            None,
            &platform_version().drive.grove_version
        )
        .unwrap()
        .expect("the tag tree reads"));
    assert!(post_entry(&drive, &contract, POST_A, OWNER_1).is_some());
    assert!(post_entry(&drive, &contract, POST_B, OWNER_1).is_some());

    let empty_list = tags_value(&[]);
    assert_eq!(
        commitment(&contract, &absent),
        commitment(&contract, &with_tags(&absent, empty_list.clone()))
    );
    assert_eq!(
        commitment(&contract, &absent),
        commitment(&contract, &with_tags(&absent, Value::Null))
    );
    // Each deletes in another form than it was created in
    delete(&drive, &contract, &with_tags(&absent, empty_list), true)
        .expect("an absent list deletes as an empty one");
    let mut empty_as_absent = empty.clone();
    empty_as_absent.properties_mut().remove("tags");
    delete(&drive, &contract, &empty_as_absent, true)
        .expect("an empty list deletes as an absent one");
    delete(
        &drive,
        &contract,
        &like(POST_A, OWNER_2, Some(&[]), T_MS),
        true,
    )
    .expect("a null list deletes as an empty one");
    for (post, owner) in [(POST_A, OWNER_1), (POST_B, OWNER_1), (POST_A, OWNER_2)] {
        assert!(post_entry(&drive, &contract, post, owner).is_none());
    }
    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Windows
// ---------------------------------------------------------------------------

/// The trending windows of a deleted like keep their entries under each tag
/// when they outlive deletes, and every window's per-tag entries drain with
/// the window once a later write finds it past its time to live. A like of a
/// cleared trending index deletes after its windows drained, its other
/// entries going as usual.
#[test]
fn should_drain_the_per_element_entries_of_an_expired_window() {
    let tags = ["dash", "evo", "news", "solo"];
    let later = T_MS + 9 * DAY_MS;
    let window_stands = |drive: &Drive, contract: &DataContract, window: &[u8]| {
        read_grove_element(
            drive,
            &like_path(contract, &[trend_level_key(contract)]),
            window,
        )
        .is_some()
    };

    let (drive, contract) = setup(Trend::Outliving);
    let liked = like(POST_A, OWNER_1, Some(&tags), T_MS);
    insert(&drive, &contract, &liked, true);
    delete(&drive, &contract, &liked, true).expect("the like deletes");
    let windows = trend_windows(&contract, T_MS);
    for tag in tags {
        assert!(tag_entry(&drive, &contract, tag, POST_A, OWNER_1).is_none());
        for window in windows.iter() {
            assert!(
                trend_entry(&drive, &contract, window, tag, POST_A, OWNER_1).is_some(),
                "the outliving window keeps {tag}"
            );
        }
    }
    // A later like drains the expired windows, entries and all
    insert_at(
        &drive,
        &contract,
        &like(POST_B, OWNER_2, Some(&["dash"]), later),
        later,
        true,
        None,
    )
    .expect("a later like inserts");
    for window in windows.iter() {
        assert!(!window_stands(&drive, &contract, window));
    }
    assert!(trend_windows(&contract, later)
        .iter()
        .all(|window| trend_entry(&drive, &contract, window, "dash", POST_B, OWNER_2).is_some()));
    assert_grovedb_is_consistent(&drive);

    let (drive, contract) = setup(Trend::Cleared);
    let liked = like(POST_A, OWNER_1, Some(&tags), T_MS);
    insert(&drive, &contract, &liked, true);
    insert_at(
        &drive,
        &contract,
        &like(POST_B, OWNER_2, Some(&["dash"]), later),
        later,
        true,
        None,
    )
    .expect("a later like inserts");
    assert!(windows
        .iter()
        .all(|window| !window_stands(&drive, &contract, window)));
    delete_at(&drive, &contract, &liked, later, true, None)
        .expect("a like whose windows drained deletes");
    for tag in tags {
        assert!(tag_entry(&drive, &contract, tag, POST_A, OWNER_1).is_none());
    }
    assert!(post_entry(&drive, &contract, POST_A, OWNER_1).is_none());
    assert!(windows
        .iter()
        .all(|window| !window_stands(&drive, &contract, window)));
    assert_grovedb_is_consistent(&drive);
}

/// When only the outliving window involves `$createdAt`, the row commits to
/// no timestamp and a delete carries none: it removes the like's entry under
/// every tag of `byTagPost` by its values alone, leaves the per-tag window
/// entries to expire, refuses the tags reordered, and the window drains later.
#[test]
fn should_delete_a_like_without_its_time_when_only_an_outliving_window_holds_it() {
    let (drive, contract) = setup(Trend::OutlivingOnlyTimed);
    assert!(!index_only_row_commits_created_at(
        like_type(&contract).required_fields(),
        like_type(&contract).index_structure(),
    ));
    let tags = ["dash", "evo", "news"];
    let liked = like(POST_A, OWNER_1, Some(&tags), T_MS);
    insert(&drive, &contract, &liked, true);
    let windows = trend_windows(&contract, T_MS);

    // The delete carries the values, not the time
    let mut values = liked.clone();
    values.set_created_at(None);
    assert!(is_not_found(delete(
        &drive,
        &contract,
        &with_tags(&values, tags_value(&["news", "dash", "evo"])),
        true
    )));
    delete(&drive, &contract, &values, true).expect("the like deletes by its values");
    assert!(post_entry(&drive, &contract, POST_A, OWNER_1).is_none());
    assert!(entry(
        &drive,
        &contract,
        &[
            text("postAuthor"),
            AUTHOR.to_vec(),
            text("postId"),
            POST_A.to_vec()
        ],
        OWNER_1
    )
    .is_none());
    for tag in tags {
        assert!(tag_entry(&drive, &contract, tag, POST_A, OWNER_1).is_none());
        for window in windows.iter() {
            assert!(
                trend_entry(&drive, &contract, window, tag, POST_A, OWNER_1).is_some(),
                "the outliving window keeps {tag}"
            );
        }
    }

    // A like again writes over the entries the deleted one left, and a later
    // like drains the windows
    insert(&drive, &contract, &liked, true);
    let later = T_MS + 9 * DAY_MS;
    insert_at(
        &drive,
        &contract,
        &like(POST_B, OWNER_2, Some(&["dash"]), later),
        later,
        true,
        None,
    )
    .expect("a later like inserts");
    for window in windows.iter() {
        assert!(read_grove_element(
            &drive,
            &like_path(&contract, &[trend_level_key(&contract)]),
            window
        )
        .is_none());
    }
    delete_at(&drive, &contract, &values, later, true, None)
        .expect("the like again deletes after its windows drained");
    assert_grovedb_is_consistent(&drive);
}

/// An index on a typed array is never the proof index, though it sorts first:
/// the executed-transition proof of a like, tagged or not, is its `byPost`
/// entry, which every like has.
#[test]
fn should_prove_a_like_through_its_document_index_when_a_tag_index_sorts_first() {
    let (drive, contract) = setup_with(like_contract_with(Trend::None, "aTagPost"));
    let document_type = like_type(&contract);
    assert_eq!(
        document_type.indexes().keys().next().map(String::as_str),
        Some("aTagPost"),
        "the tag index sorts first"
    );
    assert_eq!(
        index_only_proof_index(&document_type)
            .expect("a proof index")
            .name,
        "byPost"
    );
    for (post, tags) in [(POST_A, Some(&["dash", "evo"][..])), (POST_B, None)] {
        let liked = like(post, OWNER_1, tags, T_MS);
        insert(&drive, &contract, &liked, true);
        let path_query = index_only_transition_entry_path_query(
            contract.id(),
            document_type,
            liked.properties(),
            Identifier::from(OWNER_1),
            platform_version(),
        )
        .expect("the proof query builds from the values");
        let proof = drive
            .grove_get_proved_path_query(&path_query, None, &mut vec![], &platform_version().drive)
            .expect("the entry proves");
        let (root_hash, elements) =
            GroveDb::verify_query(&proof, &path_query, &platform_version().drive.grove_version)
                .expect("the proof verifies");
        assert_live_root_hash(&drive, root_hash);
        let element = elements
            .into_iter()
            .next()
            .and_then(|(_, _, element)| element);
        assert!(
            carries(element, &commitment(&contract, &liked)),
            "{tags:?}: the like's byPost entry is proven"
        );
    }
}

// ---------------------------------------------------------------------------
// Fees
// ---------------------------------------------------------------------------

/// The dry runs upper-bound the applied fees with the per-element fan-out: an
/// insert's storage and processing, and a delete's processing (its storage is
/// refunded), for a like of one tag and of four, with and without a
/// trending window.
#[test]
fn should_estimate_at_least_what_a_like_costs_for_one_and_four_tags() {
    for trend in [Trend::None, Trend::Outliving, Trend::Cleared] {
        for tags in [&["dash"][..], &["dash", "evo", "news", "solo"][..]] {
            let (drive, contract) = setup(trend);
            // Another like first, so the trees are not all new
            insert(
                &drive,
                &contract,
                &like(POST_B, OWNER_2, Some(&["dash"]), T_MS),
                true,
            );
            let liked = like(POST_A, OWNER_1, Some(tags), T_MS);
            let estimated = insert(&drive, &contract, &liked, false);
            let applied = insert(&drive, &contract, &liked, true);
            assert!(
                estimated.storage_fee >= applied.storage_fee,
                "{trend:?}, {} tags: insert storage estimated {} under applied {}",
                tags.len(),
                estimated.storage_fee,
                applied.storage_fee
            );
            assert!(
                estimated.processing_fee >= applied.processing_fee,
                "{trend:?}, {} tags: insert processing estimated {} under applied {}",
                tags.len(),
                estimated.processing_fee,
                applied.processing_fee
            );

            let estimated = delete(&drive, &contract, &liked, false).expect("the dry run");
            let applied = delete(&drive, &contract, &liked, true).expect("the delete");
            assert!(applied.processing_fee > 0);
            assert!(
                estimated.processing_fee >= applied.processing_fee,
                "{trend:?}, {} tags: delete processing estimated {} under applied {}",
                tags.len(),
                estimated.processing_fee,
                applied.processing_fee
            );
            assert_grovedb_is_consistent(&drive);
        }
    }
}

/// Every entry's row commitment is a double-SHA256 over the like's values, its
/// whole tag list included, and the insert bills each one: by its preimage
/// when applied, by the widest preimage the type allows in the dry run.
#[test]
fn should_bill_the_commitment_hash_of_every_entry() {
    let (drive, contract) = setup(Trend::Outliving);
    let liked = like(
        POST_A,
        OWNER_1,
        Some(&["dash", "evo", "news", "solo"]),
        T_MS,
    );
    let entries: usize = like_type(&contract)
        .indexes()
        .values()
        .map(|index| {
            Drive::index_only_entry_paths_and_key(
                contract.id(),
                like_type(&contract),
                index,
                &liked,
                platform_version(),
            )
            .expect("the entry paths")
            .0
            .len()
        })
        .sum();
    assert_eq!(
        entries,
        1 + 4 + 16 + 1,
        "byPost, byTagPost, byTrendTagPost, byAuthorPostTime"
    );
    let (_, preimage_size) = index_only_row_commitment_with_preimage_size(
        &liked,
        like_type(&contract),
        platform_version(),
    )
    .expect("the row commitment computes");
    let max_preimage_size =
        index_only_row_commitment_max_preimage_size(like_type(&contract), platform_version());
    assert!(max_preimage_size >= preimage_size);

    let transaction = drive.grove.start_transaction();
    let hashes = |estimate: bool| -> Vec<FunctionOp> {
        let mut estimated_costs_only_with_layer_info = estimate.then(HashMap::new);
        drive
            .add_document_for_contract_operations(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            &liked,
                            StorageFlags::optional_default_as_cow(),
                        )),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type: like_type(&contract),
                },
                false,
                &block_at(T_MS),
                &mut None,
                &mut estimated_costs_only_with_layer_info,
                Some(&transaction),
                platform_version(),
            )
            .expect("the insert operations")
            .into_iter()
            .filter_map(|operation| match operation {
                LowLevelDriveOperation::FunctionOperation(hash) => Some(hash),
                _ => None,
            })
            .collect()
    };
    let applied = hashes(false);
    assert_eq!(applied.len(), entries);
    assert!(applied.iter().all(
        |hash| *hash == FunctionOp::new_with_byte_count(HashFunction::Sha256_2, preimage_size)
    ));
    let estimated = hashes(true);
    assert_eq!(estimated.len(), entries);
    assert!(estimated
        .iter()
        .all(|hash| *hash
            == FunctionOp::new_with_byte_count(HashFunction::Sha256_2, max_preimage_size)));
    drive
        .grove
        .rollback_transaction(&transaction)
        .expect("the transaction rolls back");
}

/// Four tags cost more than one: the fan-out is priced, not just allowed.
#[test]
fn should_price_each_tag_of_a_like() {
    let (drive, contract) = setup(Trend::Outliving);
    let one = insert(
        &drive,
        &contract,
        &like(POST_A, OWNER_1, Some(&["dash"]), T_MS),
        false,
    );
    let four = insert(
        &drive,
        &contract,
        &like(
            POST_A,
            OWNER_1,
            Some(&["dash", "evo", "news", "solo"]),
            T_MS,
        ),
        false,
    );
    assert!(four.storage_fee > one.storage_fee);
    assert!(four.processing_fee > one.processing_fee);
}

// ---------------------------------------------------------------------------
// Queries and counts
// ---------------------------------------------------------------------------

/// Four likes: `POST_A` by `OWNER_1` tagged `dash` and `evo`, `POST_B` by
/// `OWNER_1` tagged `dash`, `POST_A` by `OWNER_2` tagged `news`, and `POST_B`
/// by `OWNER_2` untagged.
fn tagged_likes() -> (Drive, DataContract) {
    let (drive, contract) = setup(Trend::Outliving);
    for like in [
        like(POST_A, OWNER_1, Some(&["dash", "evo"]), T_MS),
        like(POST_B, OWNER_1, Some(&["dash"]), T_MS),
        like(POST_A, OWNER_2, Some(&["news"]), T_MS),
        like(POST_B, OWNER_2, None, T_MS),
    ] {
        insert(&drive, &contract, &like, true);
    }
    (drive, contract)
}

fn like_query<'a>(contract: &'a DataContract, query: Value) -> DriveDocumentQuery<'a> {
    DriveDocumentQuery::from_value(
        query,
        contract,
        like_type(contract),
        &DriveConfig::default(),
        platform_version(),
    )
    .expect("the query parses")
}

/// The `(post, owner, tags)` of each like a proved query returns, after
/// checking the proof reaches the live root hash.
fn proved_likes(
    drive: &Drive,
    contract: &DataContract,
    query: Value,
) -> Vec<([u8; 32], [u8; 32], Value)> {
    let query = like_query(contract, query);
    let (proof, _) = query
        .clone()
        .execute_with_proof(drive, None, None, platform_version())
        .expect("the query proves");
    let (root_hash, documents) = query
        .verify_proof(&proof, platform_version())
        .expect("the proof verifies");
    assert_live_root_hash(drive, root_hash);
    documents
        .iter()
        .map(|document| {
            let post = document
                .properties()
                .get("postId")
                .expect("the post")
                .to_identifier()
                .expect("an identifier")
                .to_buffer();
            let tags = document
                .properties()
                .get("tags")
                .cloned()
                .expect("the tag the like was found under");
            (post, document.owner_id().to_buffer(), tags)
        })
        .collect()
}

/// An element read on `byTagPost` proves the likes holding the tag, each
/// synthesized from its entry as a like listing that tag alone: `contains`
/// reads one tag, beside the post too, and `containsAny` several, a like
/// holding two of them once under each. Unproved, the read is refused like
/// any read through an index that does not hold every property.
#[test]
fn should_prove_the_likes_holding_a_tag() {
    let (drive, contract) = tagged_likes();
    let dash = tags_value(&["dash"]);
    assert_eq!(
        proved_likes(
            &drive,
            &contract,
            platform_value!({ "where": [["tags", "contains", "dash"]] })
        ),
        vec![
            (POST_A, OWNER_1, dash.clone()),
            (POST_B, OWNER_1, dash.clone())
        ]
    );
    assert_eq!(
        proved_likes(
            &drive,
            &contract,
            platform_value!({
                "where": [["tags", "contains", "dash"], ["postId", "==", Identifier::from(POST_B)]]
            })
        ),
        vec![(POST_B, OWNER_1, dash)]
    );
    assert_eq!(
        proved_likes(
            &drive,
            &contract,
            platform_value!({
                "where": [["tags", "containsAny", ["dash", "evo", "news"]]],
                "orderBy": [["tags", "asc"]]
            })
        ),
        vec![
            (POST_A, OWNER_1, tags_value(&["dash"])),
            (POST_B, OWNER_1, tags_value(&["dash"])),
            (POST_A, OWNER_1, tags_value(&["evo"])),
            (POST_A, OWNER_2, tags_value(&["news"])),
        ]
    );
    assert!(proved_likes(
        &drive,
        &contract,
        platform_value!({ "where": [["tags", "contains", "nothing"]] })
    )
    .is_empty());
    // Did this owner like this post under this tag: the terminal bound too
    let did_i_like = |tag: &str, owner: [u8; 32]| {
        proved_likes(
            &drive,
            &contract,
            platform_value!({
                "where": [
                    ["tags", "contains", tag],
                    ["postId", "==", Identifier::from(POST_A)],
                    ["$ownerId", "==", Identifier::from(owner)]
                ]
            }),
        )
    };
    assert_eq!(
        did_i_like("evo", OWNER_1),
        vec![(POST_A, OWNER_1, tags_value(&["evo"]))]
    );
    assert!(did_i_like("evo", OWNER_2).is_empty());

    let error = like_query(
        &contract,
        platform_value!({ "where": [["tags", "contains", "dash"]] }),
    )
    .execute_raw_results_no_proof(&drive, None, None, platform_version())
    .expect_err("an unproved read through byTagPost is refused");
    assert!(
        format!("{error:?}").contains("does not cover every required property"),
        "{error:?}"
    );
}

fn tag_clause(operator: WhereOperator, value: Value) -> WhereClause {
    WhereClause {
        field: "tags".to_string(),
        operator,
        value,
    }
}

fn count_request<'a>(
    contract: &'a DataContract,
    where_clauses: Vec<WhereClause>,
    resolved_time_ranges: Vec<ResolvedTimeRange>,
    mode: CountMode,
    prove: bool,
    drive_config: &'a DriveConfig,
) -> DocumentCountRequest<'a> {
    DocumentCountRequest {
        contract,
        document_type: like_type(contract),
        where_clauses,
        resolved_time_ranges,
        order_clauses: vec![],
        mode,
        limit: None,
        prove,
        drive_config,
    }
}

/// The count of `where_clauses`, unproved, and proved through the point
/// lookup verifier the SDK runs, which must agree and reach the live root
/// hash: one total per tag the clauses name, keyed by tag.
pub(super) fn counted(
    drive: &Drive,
    contract: &DataContract,
    where_clauses: Vec<WhereClause>,
    resolved_time_ranges: Vec<ResolvedTimeRange>,
    mode: CountMode,
) -> BTreeMap<String, u64> {
    let config = DriveConfig::default();
    let unproved = match drive
        .execute_document_count_request(
            count_request(
                contract,
                where_clauses.clone(),
                resolved_time_ranges.clone(),
                mode,
                false,
                &config,
            ),
            None,
            platform_version(),
        )
        .expect("the count executes")
    {
        DocumentCountResponse::Aggregate(total) => BTreeMap::from([(String::new(), total)]),
        DocumentCountResponse::Entries(entries) => entries
            .into_iter()
            .map(|entry| {
                (
                    String::from_utf8(entry.in_key.unwrap_or(entry.key)).expect("a tag"),
                    entry.count.unwrap_or(0),
                )
            })
            .collect(),
        other => panic!("expected a count, got {other:?}"),
    };
    let proof = match drive
        .execute_document_count_request(
            count_request(
                contract,
                where_clauses.clone(),
                resolved_time_ranges.clone(),
                mode,
                true,
                &config,
            ),
            None,
            platform_version(),
        )
        .expect("the count proves")
    {
        DocumentCountResponse::Proof(proof) => proof,
        other => panic!("expected a proof, got {other:?}"),
    };
    let keyed = key_array_element_clauses(
        where_clauses,
        like_type(contract),
        matches!(mode, CountMode::GroupByIn),
        platform_version(),
    )
    .expect("the clauses key");
    let indexes = contract
        .document_types()
        .get("like")
        .expect("the like type")
        .indexes();
    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        indexes,
        &keyed,
        &resolved_time_ranges,
    )
    .expect("a tag index counts the likes");
    let (root_hash, entries) = DriveDocumentCountQuery {
        document_type: like_type(contract),
        contract_id: contract.id().to_buffer(),
        document_type_name: "like".to_string(),
        index,
        where_clauses: keyed,
    }
    .verify_point_lookup_count_proof(&proof, platform_version())
    .expect("the count proof verifies");
    assert_live_root_hash(drive, root_hash);
    let proved: BTreeMap<String, u64> = match mode {
        CountMode::Aggregate => BTreeMap::from([(
            String::new(),
            entries.iter().map(|entry| entry.count.unwrap_or(0)).sum(),
        )]),
        _ => entries
            .into_iter()
            .map(|entry| {
                (
                    String::from_utf8(entry.in_key.unwrap_or(entry.key)).expect("a tag"),
                    entry.count.unwrap_or(0),
                )
            })
            .collect(),
    };
    assert_eq!(unproved, proved, "unproved and proved counts agree");
    proved
}

/// Likes are counted per tag, each like once under every tag it holds:
/// `byTagPost` counts a tag's likes (its post level carries the tag's total),
/// one post's likes under a tag, and one total per tag of several; the
/// trending `byTrendTagPost` counts a window's likes under a tag, a deleted
/// like included since its window outlives it. Unproved and proved agree.
#[test]
fn should_count_likes_per_tag_with_proofs() {
    let (drive, contract) = tagged_likes();
    let total = |total: u64| BTreeMap::from([(String::new(), total)]);
    let contains = |tag: &str| tag_clause(WhereOperator::Contains, Value::Text(tag.to_string()));

    assert_eq!(
        counted(
            &drive,
            &contract,
            vec![contains("dash")],
            vec![],
            CountMode::Aggregate
        ),
        total(2)
    );
    assert_eq!(
        counted(
            &drive,
            &contract,
            vec![contains("news")],
            vec![],
            CountMode::Aggregate
        ),
        total(1)
    );
    assert_eq!(
        counted(
            &drive,
            &contract,
            vec![
                contains("dash"),
                WhereClause {
                    field: "postId".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier(POST_A),
                },
            ],
            vec![],
            CountMode::Aggregate
        ),
        total(1)
    );
    assert_eq!(
        counted(
            &drive,
            &contract,
            vec![tag_clause(
                WhereOperator::ContainsAny,
                platform_value!(["dash", "evo", "news"]),
            )],
            vec![],
            CountMode::GroupByIn
        ),
        BTreeMap::from([
            ("dash".to_string(), 2),
            ("evo".to_string(), 1),
            ("news".to_string(), 1),
        ])
    );

    // A deleted like stays in the windows that outlive it
    delete(
        &drive,
        &contract,
        &like(POST_B, OWNER_1, Some(&["dash"]), T_MS),
        true,
    )
    .expect("the like deletes");
    assert_eq!(
        counted(
            &drive,
            &contract,
            vec![contains("dash")],
            vec![],
            CountMode::Aggregate
        ),
        total(1)
    );
    let window = trend_windows(&contract, T_MS)
        .first()
        .and_then(|key| DocumentPropertyType::decode_date_timestamp(key))
        .expect("a window");
    let in_window = |tag: &str| {
        vec![
            WhereClause {
                field: "$createdAt".to_string(),
                operator: WhereOperator::Equal,
                value: Value::U64(window),
            },
            contains(tag),
        ]
    };
    let trending = vec![ResolvedTimeRange {
        transform: index(&contract, "byTrendTagPost")
            .time_range
            .clone()
            .expect("a window")
            .into(),
    }];
    assert_eq!(
        counted(
            &drive,
            &contract,
            in_window("dash"),
            trending.clone(),
            CountMode::Aggregate
        ),
        total(2)
    );
    assert_eq!(
        counted(
            &drive,
            &contract,
            in_window("evo"),
            trending,
            CountMode::Aggregate
        ),
        total(1)
    );
    assert_grovedb_is_consistent(&drive);
}
