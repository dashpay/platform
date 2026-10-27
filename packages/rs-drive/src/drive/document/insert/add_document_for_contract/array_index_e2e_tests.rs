//! End-to-end coverage for indexes on typed arrays (protocol version 14): a
//! document is entered once per distinct element of the array an index names,
//! at whatever position of the index the array sits, an empty or absent list
//! enters nothing, an update moves exactly the entries whose element or other
//! values changed, and a delete removes every entry.
use crate::config::DriveConfig;
use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::drive_document_average_query::{
    AverageEntry, AverageMode, DocumentAverageRequest, DocumentAverageResponse,
};
use crate::query::drive_document_ranked_query::index_picker::find_ranked_index_for_axis;
use crate::query::drive_document_sum_query::index_picker::{
    find_summable_index_for_where_clauses, find_summable_index_with_counts_for_where_clauses,
};
use crate::query::drive_document_sum_query::{
    DocumentSumRequest, DocumentSumResponse, DriveDocumentSumQuery, SumEntry, SumMode,
};
use crate::query::{
    key_array_element_clauses, BindingSource, CountMode, DocumentCountRequest,
    DocumentCountResponse, DocumentRankedRequest, DocumentRankedResponse, DriveDocumentCountQuery,
    DriveDocumentQuery, DriveDocumentRankedQuery, DriveSubQuery, OrderClause, RankedAxis,
    RankedEntry, RankedEntryValue, SelectProjection, SubQueryBinding, SubQueryKind, WhereClause,
    WhereOperator, RANKED_COUNT_ORDER_KEY,
};
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentPropertyType;
use dpp::data_contract::DataContractFactory;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0, DocumentV0Getters, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{Error as GroveError, PathQuery, Query, SizedQuery, TransactionArg};
use std::collections::BTreeMap;

const OWNER: [u8; 32] = [7; 32];
const CREATED_AT: u64 = 1_750_000_000_000;

/// A `post` type indexing its `hashtags` (strings) and `mentions`
/// (identifiers) by element: the array first (`byHashtagTime`,
/// `byMentionTime`), alone and counted (`byHashtag`), after another property
/// (`byOwnerHashtag`), and followed by a mutable property (`byHashtagLanguage`).
fn post_contract() -> DataContract {
    DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(
            Identifier::from([1; 32]),
            0,
            platform_value!({"post": {
                "type": "object",
                "documentsMutable": true,
                "properties": {
                    "hashtags": {
                        "type": "array",
                        "maxItems": 10,
                        "items": {"type": "string", "minLength": 0, "maxLength": 61},
                        "position": 0
                    },
                    "mentions": {
                        "type": "array",
                        "maxItems": 8,
                        "items": {
                            "type": "array", "byteArray": true, "minItems": 32, "maxItems": 32,
                            "contentMediaType": "application/x.dash.dpp.identifier"
                        },
                        "position": 1
                    },
                    "language": {"type": "string", "maxLength": 2, "position": 2}
                },
                "required": ["$createdAt"],
                "indices": [
                    {"name": "byHashtagTime", "properties": [{"hashtags": "asc"}, {"$createdAt": "asc"}]},
                    {"name": "byHashtag", "properties": [{"hashtags": "asc"}],
                     "countable": "countable", "rangeCountable": true, "rankedCountable": true},
                    {"name": "byOwnerHashtag", "properties": [{"$ownerId": "asc"}, {"hashtags": "asc"}]},
                    {"name": "byHashtagLanguage", "properties": [{"hashtags": "asc"}, {"language": "asc"}]},
                    {"name": "byMentionTime", "properties": [{"mentions": "asc"}, {"$createdAt": "asc"}]}
                ],
                "additionalProperties": false
            }}),
            None,
            None,
        )
        .expect("contract")
        .data_contract_owned()
}

fn post(id: u8, hashtags: Option<&[&str]>, mentions: Option<&[[u8; 32]]>) -> Document {
    let mut properties = BTreeMap::new();
    if let Some(hashtags) = hashtags {
        properties.insert(
            "hashtags".to_string(),
            Value::Array(
                hashtags
                    .iter()
                    .map(|tag| Value::Text(tag.to_string()))
                    .collect(),
            ),
        );
    }
    if let Some(mentions) = mentions {
        properties.insert(
            "mentions".to_string(),
            Value::Array(mentions.iter().map(|id| Value::Identifier(*id)).collect()),
        );
    }
    properties.insert("language".to_string(), Value::Text("en".to_string()));
    Document::V0(DocumentV0 {
        id: Identifier::from([id; 32]),
        owner_id: Identifier::from(OWNER),
        properties,
        created_at: Some(CREATED_AT),
        revision: Some(1),
        ..Default::default()
    })
}

fn setup() -> (Drive, DataContract) {
    setup_with(post_contract())
}

/// A contract whose one document type is a `post` of `schema`.
fn post_contract_with(schema: Value) -> DataContract {
    DataContractFactory::new(PlatformVersion::latest().protocol_version)
        .expect("factory")
        .create_with_value_config(
            Identifier::from([1; 32]),
            0,
            platform_value!({ "post": schema }),
            None,
            None,
        )
        .expect("contract")
        .data_contract_owned()
}

/// A drive holding `contract`.
fn setup_with(contract: DataContract) -> (Drive, DataContract) {
    let platform_version = PlatformVersion::latest();
    let drive = setup_drive_with_initial_state_structure(Some(platform_version));
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version,
        )
        .expect("apply contract");
    (drive, contract)
}

fn insert(drive: &Drive, contract: &DataContract, document: &Document) {
    create_fee(drive, contract, document, true);
}

fn update(drive: &Drive, contract: &DataContract, document: &Document) {
    replace_fee(drive, contract, document, true);
}

fn delete(drive: &Drive, contract: &DataContract, id: u8) {
    delete_fee(drive, contract, id, true);
}

/// What creating `document` costs, applied or only priced.
fn create_fee(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    apply: bool,
) -> FeeResult {
    create_in(drive, contract, document, BlockInfo::default(), apply, None)
}

/// [`create_fee`] in `block_info`, inside `transaction`.
fn create_in(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    block_info: BlockInfo,
    apply: bool,
    transaction: TransactionArg,
) -> FeeResult {
    drive
        .add_document_for_contract(
            DocumentAndContractInfo {
                owned_document_info: OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(OWNER),
                },
                contract,
                document_type: contract.document_type_for_name("post").expect("post"),
            },
            false,
            block_info,
            apply,
            transaction,
            PlatformVersion::latest(),
            None,
        )
        .expect("add document")
}

/// What replacing the stored post with `document` costs, applied or only
/// priced.
fn replace_fee(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    apply: bool,
) -> FeeResult {
    replace_in(drive, contract, document, BlockInfo::default(), apply, None)
}

/// [`replace_fee`] in `block_info`, inside `transaction`.
fn replace_in(
    drive: &Drive,
    contract: &DataContract,
    document: &Document,
    block_info: BlockInfo,
    apply: bool,
    transaction: TransactionArg,
) -> FeeResult {
    drive
        .update_document_for_contract(
            document,
            contract,
            contract.document_type_for_name("post").expect("post"),
            Some(OWNER),
            block_info,
            apply,
            None,
            transaction,
            PlatformVersion::latest(),
            None,
        )
        .expect("update document")
}

/// What deleting post `id` costs, applied or only priced.
fn delete_fee(drive: &Drive, contract: &DataContract, id: u8, apply: bool) -> FeeResult {
    delete_in(
        drive,
        contract,
        Identifier::from([id; 32]),
        BlockInfo::default(),
        apply,
        None,
    )
}

/// [`delete_fee`] of the post `id` in `block_info`, inside `transaction`.
fn delete_in(
    drive: &Drive,
    contract: &DataContract,
    id: Identifier,
    block_info: BlockInfo,
    apply: bool,
    transaction: TransactionArg,
) -> FeeResult {
    drive
        .delete_document_for_contract(
            id,
            contract,
            "post",
            block_info,
            apply,
            transaction,
            PlatformVersion::latest(),
            None,
        )
        .expect("delete document")
}

/// The root hash of the drive's committed state.
fn live_root_hash(drive: &Drive) -> [u8; 32] {
    drive
        .grove
        .root_hash(None, &PlatformVersion::latest().drive.grove_version)
        .unwrap()
        .expect("root hash")
}

/// The total of an aggregate count.
fn aggregate_total(response: DocumentCountResponse) -> u64 {
    match response {
        DocumentCountResponse::Aggregate(total) => total,
        other => panic!("expected an aggregate, got {other:?}"),
    }
}

/// A post created at `created_at` holding `hashtags` and nothing else.
fn bare_post(id: u8, created_at: u64, hashtags: &[&str]) -> Document {
    let mut document = post_created(id, created_at, hashtags);
    document.properties_mut().remove("language");
    document
}

fn text(value: &str) -> Vec<u8> {
    value.as_bytes().to_vec()
}

/// Whether the index path `segments` (below the document type) holds the
/// non-unique entry of document `id`: `…/<segments>/0/<id>`.
fn has_entry(drive: &Drive, contract: &DataContract, segments: &[Vec<u8>], id: u8) -> bool {
    let mut segments = segments.to_vec();
    segments.push(vec![0]);
    has_key(drive, contract, &segments, &[id; 32])
}

/// Whether the tree at `segments` (below the document type) holds `key`. A
/// missing tree or ancestor holds nothing, since a delete prunes the trees it
/// empties; any other read error fails the test, so an absence is never an
/// unreadable tree.
fn has_key(drive: &Drive, contract: &DataContract, segments: &[Vec<u8>], key: &[u8]) -> bool {
    has_key_in(drive, contract, segments, key, None)
}

/// [`has_key`] read inside `transaction`.
fn has_key_in(
    drive: &Drive,
    contract: &DataContract,
    segments: &[Vec<u8>],
    key: &[u8],
    transaction: TransactionArg,
) -> bool {
    let mut path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "post");
    path.extend(segments.iter().cloned());
    match drive
        .grove
        .has_raw(
            path.as_slice(),
            key,
            transaction,
            &PlatformVersion::latest().drive.grove_version,
        )
        .unwrap()
    {
        Ok(holds) => holds,
        Err(
            GroveError::PathNotFound(_)
            | GroveError::PathParentLayerNotFound(_)
            | GroveError::PathKeyNotFound(_),
        ) => false,
        Err(error) => panic!("reading {key:?} under {path:?}: {error}"),
    }
}

fn created_at_key() -> Vec<u8> {
    DocumentPropertyType::encode_date_timestamp(CREATED_AT)
}

/// Every entry `byHashtag`, `byHashtagTime`, `byOwnerHashtag` and
/// `byHashtagLanguage` hold for document `id` under the element `tag`.
fn hashtag_entries(drive: &Drive, contract: &DataContract, tag: &str, id: u8) -> [bool; 4] {
    let hashtags = text("hashtags");
    [
        has_entry(drive, contract, &[hashtags.clone(), text(tag)], id),
        has_entry(
            drive,
            contract,
            &[
                hashtags.clone(),
                text(tag),
                text("$createdAt"),
                created_at_key(),
            ],
            id,
        ),
        has_entry(
            drive,
            contract,
            &[
                text("$ownerId"),
                OWNER.to_vec(),
                hashtags.clone(),
                text(tag),
            ],
            id,
        ),
        has_entry(
            drive,
            contract,
            &[hashtags, text(tag), text("language"), text("en")],
            id,
        ),
    ]
}

#[test]
fn should_enter_a_document_once_per_distinct_element_and_remove_every_entry_on_delete() {
    let (drive, contract) = setup();
    let mention_a = [11; 32];
    let mention_b = [12; 32];
    insert(
        &drive,
        &contract,
        &post(
            1,
            Some(&["dash", "crypto", "dash"]),
            Some(&[mention_a, mention_b]),
        ),
    );

    for tag in ["dash", "crypto"] {
        assert_eq!(
            hashtag_entries(&drive, &contract, tag, 1),
            [true; 4],
            "the post is entered under {tag} in every index naming the array"
        );
    }
    assert_eq!(
        hashtag_entries(&drive, &contract, "blockchain", 1),
        [false; 4],
        "an element the post does not hold has no entry"
    );
    for mention in [mention_a, mention_b] {
        assert!(has_entry(
            &drive,
            &contract,
            &[
                text("mentions"),
                mention.to_vec(),
                text("$createdAt"),
                created_at_key()
            ],
            1
        ));
    }

    delete(&drive, &contract, 1);
    for tag in ["dash", "crypto"] {
        assert_eq!(hashtag_entries(&drive, &contract, tag, 1), [false; 4]);
        assert!(
            !has_key(&drive, &contract, &[text("hashtags")], &text(tag)),
            "the element's value tree is pruned with its last entry"
        );
    }
    for mention in [mention_a, mention_b] {
        assert!(!has_key(&drive, &contract, &[text("mentions")], &mention));
    }
}

#[test]
fn should_write_nothing_for_an_empty_or_absent_array() {
    let (drive, contract) = setup();
    insert(&drive, &contract, &post(1, Some(&[]), None));

    // The empty list and the missing one enter nothing anywhere under the
    // arrays, not even an empty or null key, nor a branch below another
    // property of an index naming the array.
    assert!(!has_key(&drive, &contract, &[text("hashtags")], &[]));
    assert!(!has_key(&drive, &contract, &[text("mentions")], &[]));
    assert!(!has_key(
        &drive,
        &contract,
        &[text("$ownerId"), OWNER.to_vec()],
        &text("hashtags")
    ));

    insert(&drive, &contract, &post(2, Some(&[""]), None));
    // An empty string is an element, keyed apart from null.
    assert!(has_entry(
        &drive,
        &contract,
        &[text("hashtags"), vec![0]],
        2
    ));
    assert!(!has_entry(
        &drive,
        &contract,
        &[text("hashtags"), vec![0]],
        1
    ));

    // Both delete cleanly: there is nothing to look for under the arrays.
    delete(&drive, &contract, 1);
    delete(&drive, &contract, 2);
    assert!(!has_key(&drive, &contract, &[text("hashtags")], &[0]));
}

#[test]
fn should_move_only_the_changed_entries_on_update() {
    let (drive, contract) = setup();
    let mut document = post(1, Some(&["a", "b"]), None);
    insert(&drive, &contract, &document);

    // Overlapping sets: "a" leaves, "b" stays, "c" arrives.
    document.set("hashtags", platform_value!(["b", "c"]));
    document.set_revision(Some(2));
    update(&drive, &contract, &document);
    assert_eq!(hashtag_entries(&drive, &contract, "a", 1), [false; 4]);
    assert_eq!(hashtag_entries(&drive, &contract, "b", 1), [true; 4]);
    assert_eq!(hashtag_entries(&drive, &contract, "c", 1), [true; 4]);
    assert!(!has_key(&drive, &contract, &[text("hashtags")], &text("a")));

    // Same elements, repeated and reordered: nothing moves.
    document.set("hashtags", platform_value!(["c", "b", "c"]));
    document.set_revision(Some(3));
    update(&drive, &contract, &document);
    assert_eq!(hashtag_entries(&drive, &contract, "b", 1), [true; 4]);
    assert_eq!(hashtag_entries(&drive, &contract, "c", 1), [true; 4]);

    // A value after the array changes: every element's entry in that index
    // moves to the new value, the other indexes keep theirs.
    document.set("language", Value::Text("fr".to_string()));
    document.set_revision(Some(4));
    update(&drive, &contract, &document);
    for tag in ["b", "c"] {
        assert!(!has_entry(
            &drive,
            &contract,
            &[text("hashtags"), text(tag), text("language"), text("en")],
            1
        ));
        assert!(has_entry(
            &drive,
            &contract,
            &[text("hashtags"), text(tag), text("language"), text("fr")],
            1
        ));
        assert!(has_entry(
            &drive,
            &contract,
            &[text("hashtags"), text(tag)],
            1
        ));
    }

    // The list empties: the post leaves every index naming it.
    document.set("hashtags", platform_value!([]));
    document.set_revision(Some(5));
    update(&drive, &contract, &document);
    for tag in ["b", "c"] {
        assert!(!has_key(&drive, &contract, &[text("hashtags")], &text(tag)));
        assert!(!has_entry(
            &drive,
            &contract,
            &[
                text("$ownerId"),
                OWNER.to_vec(),
                text("hashtags"),
                text(tag)
            ],
            1
        ));
    }

    // And a fully new list brings it back.
    document.set("hashtags", platform_value!(["x", "y"]));
    document.set_revision(Some(6));
    update(&drive, &contract, &document);
    for tag in ["x", "y"] {
        assert!(has_entry(
            &drive,
            &contract,
            &[text("hashtags"), text(tag)],
            1
        ));
        assert!(has_entry(
            &drive,
            &contract,
            &[text("hashtags"), text(tag), text("language"), text("fr")],
            1
        ));
    }

    delete(&drive, &contract, 1);
    for tag in ["x", "y"] {
        assert!(!has_key(&drive, &contract, &[text("hashtags")], &text(tag)));
    }
}

/// A unique index beside an array index under the same first property: a
/// replace reads the unique index's earlier sibling levels for a missing
/// value, and passes the array's level by (an element is never empty, and a
/// document without one is entered nowhere under the array), so replaces that
/// change, empty or keep the list all apply and move exactly their entries.
#[test]
fn should_replace_beside_a_unique_index_sharing_a_level_with_the_array() {
    let (drive, contract) = setup_with(post_contract_with(platform_value!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "hashtags": {
                "type": "array",
                "maxItems": 4,
                "items": {"type": "string", "minLength": 1, "maxLength": 20},
                "position": 0
            },
            "slug": {"type": "string", "maxLength": 20, "position": 1}
        },
        "required": ["slug"],
        "indices": [
            {"name": "byOwnerSlug", "properties": [{"$ownerId": "asc"}, {"slug": "asc"}], "unique": true},
            {"name": "byOwnerHashtag", "properties": [{"$ownerId": "asc"}, {"hashtags": "asc"}]}
        ],
        "additionalProperties": false
    })));
    let slug_entry = |slug: &str| {
        has_key(
            &drive,
            &contract,
            &[text("$ownerId"), OWNER.to_vec(), text("slug")],
            &text(slug),
        )
    };
    let hashtag_entry = |tag: &str| {
        has_entry(
            &drive,
            &contract,
            &[
                text("$ownerId"),
                OWNER.to_vec(),
                text("hashtags"),
                text(tag),
            ],
            1,
        )
    };
    let mut document = Document::V0(DocumentV0 {
        id: Identifier::from([1; 32]),
        owner_id: Identifier::from(OWNER),
        properties: BTreeMap::from([
            ("slug".to_string(), Value::Text("first".to_string())),
            ("hashtags".to_string(), platform_value!(["dash"])),
        ]),
        revision: Some(1),
        ..Default::default()
    });
    insert(&drive, &contract, &document);
    assert!(slug_entry("first") && hashtag_entry("dash"));

    document.set("slug", Value::Text("second".to_string()));
    document.set("hashtags", platform_value!(["news"]));
    document.set_revision(Some(2));
    update(&drive, &contract, &document);
    assert!(slug_entry("second") && !slug_entry("first"));
    assert!(hashtag_entry("news") && !hashtag_entry("dash"));

    document.set("hashtags", platform_value!([]));
    document.set_revision(Some(3));
    update(&drive, &contract, &document);
    assert!(slug_entry("second") && !hashtag_entry("news"));

    document.set_revision(Some(4));
    update(&drive, &contract, &document);
    assert!(slug_entry("second"));
}

/// Pricing a write without applying it walks the same element fan-out with
/// estimated layers, and touches no state: a create and a replace of a real
/// post price its elements, a delete prices the stored post as the worst case
/// (`maxItems` estimated elements per array, each a key of its own). Each
/// estimate covers what the write then costs from the same state, including
/// replaces that move to a disjoint element set, shrink it to one element and
/// empty it (removing more entries than they insert), and an array under
/// overlapping windows, whose entries multiply.
#[test]
fn should_estimate_at_least_what_writes_of_an_indexed_array_cost() {
    let created_at = 20_000 * DAY_MS + 1_000;
    let elements = ["dash", "news", "web3", "defi"];
    for (contract, document) in [
        (post_contract(), post(1, Some(&elements), Some(&[[7; 32]]))),
        (
            trending_three_days_contract(),
            bare_post(1, created_at, &elements),
        ),
        (
            widest_windows_contract(),
            post_created(1, created_at, &elements),
        ),
    ] {
        let (drive, contract) = setup_with(contract);
        // The processing too: a replace removing entries pays processing for
        // them and writes few bytes, so a storage surplus is no cover.
        let covers = |write: &str, estimated: FeeResult, actual: FeeResult| {
            assert!(
                estimated.total_base_fee() >= actual.total_base_fee(),
                "{write}: estimated {} below actual {}",
                estimated.total_base_fee(),
                actual.total_base_fee()
            );
            assert!(
                estimated.processing_fee >= actual.processing_fee,
                "{write}: estimated processing {} below actual {}",
                estimated.processing_fee,
                actual.processing_fee
            );
        };

        let empty = live_root_hash(&drive);
        let estimated = create_fee(&drive, &contract, &document, false);
        assert_eq!(live_root_hash(&drive), empty, "pricing writes nothing");
        covers(
            "create",
            estimated,
            create_fee(&drive, &contract, &document, true),
        );

        let mut replaced = document.clone();
        for (revision, replaced_elements) in [
            (2, vec!["crypto", "nft", "dao", "art"]),
            (3, vec!["crypto"]),
            (4, vec![]),
        ] {
            replaced.set("hashtags", platform_value!(replaced_elements.clone()));
            replaced.set_revision(Some(revision));
            let stored = live_root_hash(&drive);
            let estimated = replace_fee(&drive, &contract, &replaced, false);
            assert_eq!(live_root_hash(&drive), stored, "pricing writes nothing");
            covers(
                &format!("replace to {replaced_elements:?}"),
                estimated,
                replace_fee(&drive, &contract, &replaced, true),
            );
        }

        let replaced_root = live_root_hash(&drive);
        let estimated = delete_fee(&drive, &contract, 1, false);
        assert_eq!(
            live_root_hash(&drive),
            replaced_root,
            "pricing writes nothing"
        );
        covers("delete", estimated, delete_fee(&drive, &contract, 1, true));
    }
}

#[test]
fn should_keep_shared_element_trees_when_another_post_holds_the_element() {
    let (drive, contract) = setup();
    let mut first = post(1, Some(&["dash", "news"]), None);
    insert(&drive, &contract, &first);
    insert(&drive, &contract, &post(2, Some(&["dash"]), None));

    first.set("hashtags", platform_value!(["news"]));
    first.set_revision(Some(2));
    update(&drive, &contract, &first);
    assert!(!has_entry(
        &drive,
        &contract,
        &[text("hashtags"), text("dash")],
        1
    ));
    assert!(has_entry(
        &drive,
        &contract,
        &[text("hashtags"), text("dash")],
        2
    ));

    delete(&drive, &contract, 2);
    assert!(!has_key(
        &drive,
        &contract,
        &[text("hashtags")],
        &text("dash")
    ));
    assert!(has_entry(
        &drive,
        &contract,
        &[text("hashtags"), text("news")],
        1
    ));
}

fn post_created(id: u8, created_at: u64, hashtags: &[&str]) -> Document {
    let mut document = post(id, Some(hashtags), None);
    document.set_created_at(Some(created_at));
    document
}

fn documents_query<'a>(contract: &'a DataContract, query: Value) -> DriveDocumentQuery<'a> {
    DriveDocumentQuery::from_value(
        query,
        contract,
        contract.document_type_for_name("post").expect("post"),
        &DriveConfig::default(),
        PlatformVersion::latest(),
    )
    .expect("the query parses")
}

fn first_bytes(contract: &DataContract, serialized: &[Vec<u8>]) -> Vec<u8> {
    let document_type = contract.document_type_for_name("post").expect("post");
    serialized
        .iter()
        .map(|bytes| {
            Document::from_bytes(bytes, document_type, PlatformVersion::latest())
                .expect("a stored post")
                .id()
                .to_buffer()[0]
        })
        .collect()
}

/// The ids (first byte) a documents query returns, unproved and proved: the
/// two must agree, and the proof must reach the live root hash.
fn query_ids(drive: &Drive, contract: &DataContract, query: Value) -> Vec<u8> {
    let platform_version = PlatformVersion::latest();
    let query = documents_query(contract, query);
    let (unproved, _, _) = query
        .execute_raw_results_no_proof(drive, None, None, platform_version)
        .expect("the query executes");
    let unproved = first_bytes(contract, &unproved);
    let (proof, _) = query
        .clone()
        .execute_with_proof(drive, None, None, platform_version)
        .expect("the query proves");
    let (root_hash, proved) = query
        .verify_proof(&proof, platform_version)
        .expect("the proof verifies");
    assert_eq!(
        root_hash,
        live_root_hash(drive),
        "the proof reaches the live root hash"
    );
    let proved: Vec<u8> = proved
        .iter()
        .map(|document| document.id().to_buffer()[0])
        .collect();
    assert_eq!(unproved, proved, "proved and unproved pages agree");
    proved
}

fn query_error(drive: &Drive, contract: &DataContract, query: Value) -> String {
    let platform_version = PlatformVersion::latest();
    let document_type = contract.document_type_for_name("post").expect("post");
    let parsed = DriveDocumentQuery::from_value(
        query,
        contract,
        document_type,
        &DriveConfig::default(),
        platform_version,
    );
    let error = match parsed {
        Err(error) => error,
        Ok(query) => query
            .execute_raw_results_no_proof(drive, None, None, platform_version)
            .expect_err("the query is refused"),
    };
    format!("{error:?}")
}

fn tagged_posts() -> (Drive, DataContract) {
    let (drive, contract) = setup();
    for (id, at, tags) in [
        (1, CREATED_AT + 1, &["dash", "crypto"][..]),
        (2, CREATED_AT + 2, &["crypto"][..]),
        (3, CREATED_AT + 3, &["dash"][..]),
        (4, CREATED_AT + 4, &["news", "dash", "crypto"][..]),
        (5, CREATED_AT + 5, &["news"][..]),
    ] {
        insert(&drive, &contract, &post_created(id, at, tags));
    }
    (drive, contract)
}

#[test]
fn should_return_the_posts_holding_an_element_newest_first() {
    let (drive, contract) = tagged_posts();
    assert_eq!(
        query_ids(
            &drive,
            &contract,
            platform_value!({
                "where": [["hashtags", "contains", "dash"]],
                "orderBy": [["$createdAt", "desc"]]
            })
        ),
        vec![4, 3, 1]
    );
    assert_eq!(
        query_ids(
            &drive,
            &contract,
            platform_value!({
                "where": [["hashtags", "contains", "crypto"], ["$createdAt", ">", CREATED_AT + 1]],
                "orderBy": [["$createdAt", "asc"]]
            })
        ),
        vec![2, 4]
    );
    // The array after another property of the index.
    assert_eq!(
        query_ids(
            &drive,
            &contract,
            platform_value!({
                "where": [["$ownerId", "==", Identifier::from(OWNER)], ["hashtags", "contains", "news"]]
            })
        ),
        vec![4, 5]
    );
    assert!(query_ids(
        &drive,
        &contract,
        platform_value!({"where": [["hashtags", "contains", "nothing"]]})
    )
    .is_empty());
}

#[test]
fn should_page_through_one_element_with_a_cursor() {
    let (drive, contract) = tagged_posts();
    let page = |start_after: Option<u8>| {
        let query = match start_after {
            None => platform_value!({
                "where": [["hashtags", "contains", "dash"]],
                "orderBy": [["$createdAt", "desc"]],
                "limit": 2
            }),
            Some(id) => platform_value!({
                "where": [["hashtags", "contains", "dash"]],
                "orderBy": [["$createdAt", "desc"]],
                "limit": 2,
                "startAfter": Identifier::from([id; 32])
            }),
        };
        query_ids(&drive, &contract, query)
    };
    assert_eq!(page(None), vec![4, 3]);
    assert_eq!(page(Some(3)), vec![1]);
    assert!(page(Some(1)).is_empty());

    // A cursor document that does not hold the element is not on the page.
    let error = query_error(
        &drive,
        &contract,
        platform_value!({
            "where": [["hashtags", "contains", "dash"]],
            "orderBy": [["$createdAt", "desc"]],
            "startAfter": Identifier::from([2; 32])
        }),
    );
    assert!(error.contains("does not hold the element"), "{error}");
}

#[test]
fn should_return_each_post_once_for_several_elements() {
    let (drive, contract) = tagged_posts();
    // Walked element by element ("crypto", then "dash", then "news"), each
    // post where it first appears.
    assert_eq!(
        query_ids(
            &drive,
            &contract,
            platform_value!({
                "where": [["hashtags", "containsAny", ["news", "dash", "crypto"]]],
                "orderBy": [["hashtags", "asc"], ["$createdAt", "asc"]]
            })
        ),
        vec![1, 2, 4, 3, 5]
    );
    assert_eq!(
        query_ids(
            &drive,
            &contract,
            platform_value!({
                "where": [["hashtags", "containsAny", ["news", "dash"]]],
                "orderBy": [["hashtags", "desc"], ["$createdAt", "desc"]]
            })
        ),
        vec![5, 4, 3, 1]
    );
    // The limit counts entries: "crypto" fills it with 1, 2, 4 before "dash"
    // repeats 1 and 4.
    assert_eq!(
        query_ids(
            &drive,
            &contract,
            platform_value!({
                "where": [["hashtags", "containsAny", ["dash", "crypto"]]],
                "orderBy": [["hashtags", "asc"], ["$createdAt", "asc"]],
                "limit": 5
            })
        ),
        vec![1, 2, 4, 3]
    );
}

#[test]
fn should_refuse_shapes_that_could_repeat_or_misread_a_post() {
    let (drive, contract) = tagged_posts();
    for (query, expected) in [
        // An array is read by its elements only.
        (
            platform_value!({"where": [["hashtags", "==", "dash"]]}),
            "read with contains or containsAny",
        ),
        (
            platform_value!({"where": [["hashtags", "in", ["dash"]]]}),
            "read with contains or containsAny",
        ),
        (
            platform_value!({"where": [["language", "contains", "en"]]}),
            "read the elements of a typed array",
        ),
        (
            platform_value!({"where": [["hashtags", "contains", 7]]}),
            "invalid value type for an element",
        ),
        // A cursor can not place a page reached under several elements.
        (
            platform_value!({
                "where": [["hashtags", "containsAny", ["dash", "news"]]],
                "orderBy": [["hashtags", "asc"]],
                "startAfter": Identifier::from([1; 32])
            }),
            "not supported with containsAny",
        ),
        // A list has no single value to order documents by.
        (
            platform_value!({
                "where": [["hashtags", "contains", "dash"]],
                "orderBy": [["hashtags", "asc"]]
            }),
            "can only be ordered by with containsAny",
        ),
        // An index naming the array is not read without its elements: the
        // owner's posts would come back once per hashtag.
        (
            platform_value!({"where": [["$ownerId", "==", Identifier::from(OWNER)]]}),
            "query must be for valid indexes",
        ),
        // `$id` is no typed array: an element clause on it is refused, not
        // dropped from a query that would then read every post.
        (
            platform_value!({"where": [["$id", "contains", Identifier::from([1; 32])]]}),
            "read the elements of a typed array",
        ),
        (
            platform_value!({
                "where": [["$id", "containsAny", [Identifier::from([1; 32])]]],
                "orderBy": [["$id", "asc"]]
            }),
            "read the elements of a typed array",
        ),
    ] {
        let error = query_error(&drive, &contract, query.clone());
        assert!(error.contains(expected), "{query:?}: {error}");
    }
}

/// A composite query reads one element at most in each sub-query: a
/// sub-query's documents are read out of the merged proof without the
/// repeats `containsAny` reaches a post under, and a count sub-query keys its
/// element pin as the count surface does.
#[test]
fn should_read_one_element_at_most_in_a_sub_query() {
    let (_drive, contract) = tagged_posts();
    let platform_version = PlatformVersion::latest();
    let document_type = contract.document_type_for_name("post").expect("post");
    let page = || {
        documents_query(
            &contract,
            platform_value!({"where": [["hashtags", "contains", "dash"]], "limit": 10}),
        )
    };

    let mut composite = page();
    composite.sub_queries.push(DriveSubQuery {
        contract: &contract,
        document_type,
        kind: SubQueryKind::Documents,
        where_clauses: element_clause(
            WhereOperator::ContainsAny,
            platform_value!(["dash", "news"]),
        ),
        order_by: vec![OrderClause {
            field: "hashtags".to_string(),
            ascending: true,
        }],
        limit: Some(10),
        binding: None,
    });
    let error = composite
        .validate_composite(platform_version)
        .expect_err("containsAny in a sub-query is refused");
    assert!(
        error
            .to_string()
            .contains("containsAny is not supported in a sub-query"),
        "{error}"
    );

    // A count sub-query judges its element clauses as the count surface
    // does: an equality on the array is refused, not keyed as an element.
    let mut composite = page();
    composite.sub_queries.push(DriveSubQuery {
        contract: &contract,
        document_type,
        kind: SubQueryKind::Count,
        where_clauses: element_clause(WhereOperator::Equal, Value::Text("dash".to_string())),
        order_by: vec![],
        limit: None,
        binding: Some(SubQueryBinding {
            source: BindingSource::Page,
            source_property: "$ownerId".to_string(),
            field: "$ownerId".to_string(),
        }),
    });
    let error = composite
        .sub_query_count_query(
            &composite.sub_queries[0],
            &[Identifier::from(OWNER)],
            platform_version,
        )
        .expect_err("an equality on a typed array is refused");
    assert!(
        format!("{error:?}").contains("read with contains or containsAny"),
        "{error:?}"
    );
}

fn element_clause(operator: WhereOperator, value: Value) -> Vec<WhereClause> {
    vec![WhereClause {
        field: "hashtags".to_string(),
        operator,
        value,
    }]
}

fn count(
    drive: &Drive,
    contract: &DataContract,
    where_clauses: Vec<WhereClause>,
    mode: CountMode,
    prove: bool,
) -> Result<DocumentCountResponse, Error> {
    drive.execute_document_count_request(
        DocumentCountRequest {
            contract,
            document_type: contract.document_type_for_name("post").expect("post"),
            where_clauses,
            resolved_time_ranges: vec![],
            order_clauses: vec![],
            mode,
            limit: None,
            prove,
            drive_config: &DriveConfig::default(),
        },
        None,
        PlatformVersion::latest(),
    )
}

/// The verified per-element counts of a proved point-lookup count, keyed by
/// element.
fn proved_counts(
    drive: &Drive,
    contract: &DataContract,
    where_clauses: Vec<WhereClause>,
    mode: CountMode,
) -> BTreeMap<String, u64> {
    let platform_version = PlatformVersion::latest();
    let document_type = contract.document_type_for_name("post").expect("post");
    let proof = match count(drive, contract, where_clauses.clone(), mode, true).expect("proves") {
        DocumentCountResponse::Proof(proof) => proof,
        other => panic!("expected a proof, got {other:?}"),
    };
    // The verifier keys the clauses as the prover did.
    let keyed = key_array_element_clauses(
        where_clauses,
        document_type,
        matches!(mode, CountMode::GroupByIn),
        platform_version,
    )
    .expect("keyed");
    let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
        document_type.indexes(),
        &keyed,
        &[],
    )
    .expect("byHashtag counts the elements");
    let (root_hash, entries) = DriveDocumentCountQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "post".to_string(),
        index,
        where_clauses: keyed,
    }
    .verify_point_lookup_count_proof(&proof, platform_version)
    .expect("the count proof verifies");
    assert_eq!(root_hash, live_root_hash(drive));
    entries
        .into_iter()
        .map(|entry| {
            let key = entry.in_key.unwrap_or(entry.key);
            (
                String::from_utf8(key).expect("a hashtag"),
                entry.count.unwrap_or(0),
            )
        })
        .collect()
}

#[test]
fn should_count_the_posts_holding_each_element() {
    let (drive, contract) = tagged_posts();
    let dash = element_clause(WhereOperator::Contains, Value::Text("dash".to_string()));
    assert_eq!(
        aggregate_total(
            count(&drive, &contract, dash.clone(), CountMode::Aggregate, false).expect("counts")
        ),
        3
    );
    assert_eq!(
        proved_counts(&drive, &contract, dash, CountMode::Aggregate)
            .values()
            .sum::<u64>(),
        3
    );

    // Several elements are counted per element, each post under every one it
    // holds.
    let several = element_clause(
        WhereOperator::ContainsAny,
        platform_value!(["dash", "crypto", "news"]),
    );
    assert_eq!(
        proved_counts(&drive, &contract, several.clone(), CountMode::GroupByIn),
        BTreeMap::from([
            ("crypto".to_string(), 3),
            ("dash".to_string(), 3),
            ("news".to_string(), 2),
        ])
    );
    // A total over several elements would count post 4 three times, and a
    // range clause folds a group's totals: both refused, naming the rule.
    let error = count(
        &drive,
        &contract,
        several.clone(),
        CountMode::Aggregate,
        false,
    )
    .expect_err("a total over several elements is refused");
    assert!(
        format!("{error:?}").contains("only answered grouped by \\\"hashtags\\\" alone"),
        "{error:?}"
    );
    let mut several_in_a_range = several;
    several_in_a_range.push(WhereClause {
        field: "$createdAt".to_string(),
        operator: WhereOperator::GreaterThan,
        value: Value::U64(0),
    });
    let error = count(
        &drive,
        &contract,
        several_in_a_range,
        CountMode::GroupByIn,
        false,
    )
    .expect_err("a range beside several elements is refused");
    assert!(
        format!("{error:?}").contains("and without a range clause"),
        "{error:?}"
    );
    // An empty string is an element like any other.
    insert(&drive, &contract, &post_created(6, CREATED_AT + 6, &[""]));
    let empty_string = count(
        &drive,
        &contract,
        element_clause(WhereOperator::Contains, Value::Text(String::new())),
        CountMode::Aggregate,
        false,
    )
    .expect("an empty string element counts");
    assert_eq!(aggregate_total(empty_string), 1);
    // And a raw equality on the array is no element read.
    let error = count(
        &drive,
        &contract,
        element_clause(WhereOperator::Equal, Value::Text("dash".to_string())),
        CountMode::Aggregate,
        false,
    )
    .expect_err("an equality on a typed array is refused");
    assert!(
        format!("{error:?}").contains("read with contains or containsAny"),
        "{error:?}"
    );
}

#[test]
fn should_rank_elements_by_the_posts_holding_them() {
    let (drive, contract) = tagged_posts();
    let platform_version = PlatformVersion::latest();
    let document_type = contract.document_type_for_name("post").expect("post");
    let group_by = vec!["hashtags".to_string()];
    let order_by = vec![OrderClause {
        field: RANKED_COUNT_ORDER_KEY.to_string(),
        ascending: false,
    }];
    let ranked = |prove: bool| {
        drive
            .execute_document_ranked_request(
                DocumentRankedRequest {
                    contract: &contract,
                    document_type,
                    group_by: &group_by,
                    select: SelectProjection::count_star(),
                    having: &[],
                    order_by: &order_by,
                    where_clauses: &[],
                    limit: Some(10),
                    offset: Some(0),
                    has_start_at: false,
                    prove,
                    resolved_time_ranges: &[],
                },
                None,
                platform_version,
            )
            .expect("the ranking executes")
    };
    let counts = |entries: Vec<RankedEntry>| -> Vec<(String, u64)> {
        entries
            .into_iter()
            .map(|entry| {
                let RankedEntryValue::Count(count) = entry.value else {
                    panic!("a count ranking");
                };
                (String::from_utf8(entry.key).expect("a hashtag"), count)
            })
            .collect()
    };
    let unproved = match ranked(false) {
        DocumentRankedResponse::Entries(page) => counts(page.entries),
        DocumentRankedResponse::Proof(_) => panic!("expected entries"),
    };
    assert_eq!(
        unproved,
        vec![
            ("dash".to_string(), 3),
            ("crypto".to_string(), 3),
            ("news".to_string(), 2)
        ]
    );
    let proof = match ranked(true) {
        DocumentRankedResponse::Proof(proof) => proof,
        DocumentRankedResponse::Entries(_) => panic!("expected a proof"),
    };
    let (root_hash, page) = DriveDocumentRankedQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "post".to_string(),
        index: find_ranked_index_for_axis(
            document_type.indexes(),
            "hashtags",
            &[],
            RankedAxis::Count,
            "",
            &[],
        )
        .expect("byHashtag ranks the elements"),
        axis: RankedAxis::Count,
        prefix_branches: vec![Vec::new()],
        descending: true,
        k: 10,
        offset: 0,
    }
    .verify_ranked_top_k_proof(&proof, platform_version)
    .expect("the ranking proof verifies");
    assert_eq!(
        root_hash,
        live_root_hash(&drive),
        "the ranking proof reaches the live root hash"
    );
    assert_eq!(counts(page.entries), unproved);
}

const DAY_MS: u64 = 86_400_000;

/// A `post` type ranking each day's hashtags: `[$createdAt day window,
/// hashtags]`, the array held to 4 elements under the window.
fn trending_contract() -> DataContract {
    post_contract_with(platform_value!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "hashtags": {
                "type": "array",
                "maxItems": 4,
                "items": {"type": "string", "minLength": 1, "maxLength": 59},
                "position": 0
            }
        },
        "required": ["$createdAt"],
        "indices": [{
            "name": "trendingToday",
            "properties": [{"$createdAt": "asc"}, {"hashtags": "asc"}],
            "timeRange": {"on": "$createdAt", "range": 86400, "step": 86400},
            "rangeCountable": true,
            "rankedCountable": true
        }],
        "additionalProperties": false
    }))
}

#[test]
fn should_enter_a_post_once_per_element_in_its_window() {
    let (drive, contract) = setup_with(trending_contract());
    let document_type = contract.document_type_for_name("post").expect("post");
    let level_key = document_type
        .indexes()
        .get("trendingToday")
        .and_then(|index| index.time_range.as_ref())
        .expect("a window")
        .storage_key("$createdAt");
    let day = 20_000 * DAY_MS;
    let window = DocumentPropertyType::encode_date_timestamp(day);
    let entry = |tag: &str, id: u8| {
        has_entry(
            &drive,
            &contract,
            &[
                level_key.as_bytes().to_vec(),
                window.clone(),
                text("hashtags"),
                text(tag),
            ],
            id,
        )
    };

    let mut first = bare_post(1, day + 1_000, &["dash", "news"]);
    insert(&drive, &contract, &first);
    insert(&drive, &contract, &bare_post(2, day + 2_000, &["dash"]));
    assert!(entry("dash", 1) && entry("news", 1) && entry("dash", 2));
    assert!(!entry("news", 2));

    first.set("hashtags", platform_value!(["news", "crypto"]));
    first.set_revision(Some(2));
    update(&drive, &contract, &first);
    assert!(!entry("dash", 1) && entry("news", 1) && entry("crypto", 1));
    assert!(entry("dash", 2), "the other post keeps the shared element");

    delete(&drive, &contract, 1);
    assert!(!entry("news", 1) && !entry("crypto", 1));
    assert!(!has_key(
        &drive,
        &contract,
        &[
            level_key.as_bytes().to_vec(),
            window.clone(),
            text("hashtags")
        ],
        &text("news")
    ));
    assert!(entry("dash", 2));
}

#[test]
fn should_read_an_identifier_element_given_as_bytes_or_base58_text() {
    let (drive, contract) = setup();
    let mentioned = [11; 32];
    insert(
        &drive,
        &contract,
        &post(1, None, Some(&[mentioned, [12; 32]])),
    );
    insert(&drive, &contract, &post(2, None, Some(&[[12; 32]])));
    let as_identifier = query_ids(
        &drive,
        &contract,
        platform_value!({"where": [["mentions", "contains", Identifier::from(mentioned)]]}),
    );
    let as_text = query_ids(
        &drive,
        &contract,
        platform_value!({
            "where": [["mentions", "contains", bs58::encode(mentioned).into_string()]]
        }),
    );
    assert_eq!(as_identifier, vec![1]);
    assert_eq!(as_text, as_identifier);
}

/// The `hashtags` schema the summing and skipping contracts share: strings,
/// the empty one included.
fn hashtags_schema() -> Value {
    platform_value!({
        "type": "array",
        "maxItems": 10,
        "items": {"type": "string", "minLength": 0, "maxLength": 61},
        "position": 0
    })
}

/// An index naming the array beside a declared skip property: any element
/// binds the array, an empty string included, on the aggregate surfaces as on
/// the documents one, while the declared property keeps the keyword's rule.
#[test]
fn should_bind_an_empty_string_element_beside_a_declared_skip_property() {
    let (drive, contract) = setup_with(post_contract_with(platform_value!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "hashtags": hashtags_schema(),
            "language": {"type": "string", "maxLength": 2, "position": 1}
        },
        "required": ["$createdAt"],
        "indices": [{
            "name": "byHashtagLanguage",
            "properties": [{"hashtags": "asc"}, {"language": "asc"}],
            "countable": "countable",
            "skipIfAbsent": ["language"]
        }],
        "additionalProperties": false
    })));
    insert(&drive, &contract, &post(1, Some(&["", "dash"]), None));
    let mut unlabelled = post(2, Some(&[""]), None);
    unlabelled.properties_mut().remove("language");
    insert(&drive, &contract, &unlabelled);

    let empty_in_english = vec![
        WhereClause {
            field: "hashtags".to_string(),
            operator: WhereOperator::Contains,
            value: Value::Text(String::new()),
        },
        WhereClause {
            field: "language".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("en".to_string()),
        },
    ];
    let empty_string = count(
        &drive,
        &contract,
        empty_in_english.clone(),
        CountMode::Aggregate,
        false,
    )
    .expect("the index counts the empty-string element");
    assert_eq!(aggregate_total(empty_string), 1);
    assert_eq!(
        proved_counts(&drive, &contract, empty_in_english, CountMode::Aggregate)
            .values()
            .sum::<u64>(),
        1
    );
    assert_eq!(
        query_ids(
            &drive,
            &contract,
            platform_value!({"where": [["hashtags", "contains", ""], ["language", "==", "en"]]})
        ),
        vec![1]
    );
}

/// The per-element `(count, sum)` of `amount` over `elements`, through the
/// sum and the average surfaces, unproved and proved alike (the proofs
/// verified against the live root hash).
fn element_totals(
    drive: &Drive,
    contract: &DataContract,
    elements: &[&str],
) -> BTreeMap<String, (u64, i64)> {
    let platform_version = PlatformVersion::latest();
    let document_type = contract.document_type_for_name("post").expect("post");
    let drive_config = DriveConfig::default();
    let where_clauses = element_clause(
        WhereOperator::ContainsAny,
        Value::Array(
            elements
                .iter()
                .map(|element| Value::Text(element.to_string()))
                .collect(),
        ),
    );
    let keyed =
        key_array_element_clauses(where_clauses.clone(), document_type, true, platform_version)
            .expect("keyed");
    let live = live_root_hash(drive);
    let element = |in_key: Option<Vec<u8>>, key: Vec<u8>| {
        String::from_utf8(in_key.unwrap_or(key)).expect("a hashtag")
    };
    let sum_query = |index| DriveDocumentSumQuery {
        document_type,
        contract_id: contract.id().to_buffer(),
        document_type_name: "post".to_string(),
        index,
        where_clauses: keyed.clone(),
        sum_property: "amount".to_string(),
    };
    let sum_request = |prove| DocumentSumRequest {
        contract,
        document_type,
        sum_property: "amount".to_string(),
        where_clauses: where_clauses.clone(),
        resolved_time_ranges: vec![],
        order_clauses: vec![],
        mode: SumMode::GroupByIn,
        limit: None,
        prove,
        drive_config: &drive_config,
    };
    let average_request = |prove| DocumentAverageRequest {
        contract,
        document_type,
        sum_property: "amount".to_string(),
        where_clauses: where_clauses.clone(),
        resolved_time_ranges: vec![],
        order_clauses: vec![],
        mode: AverageMode::GroupByIn,
        limit: None,
        prove,
        drive_config: &drive_config,
    };

    let sums = |entries: Vec<SumEntry>| -> BTreeMap<String, i64> {
        entries
            .into_iter()
            .map(|entry| {
                (
                    element(entry.in_key, entry.key),
                    entry.sum.expect("a summed element"),
                )
            })
            .collect()
    };
    let DocumentSumResponse::Entries(unproved) = drive
        .execute_document_sum_request(sum_request(false), None, platform_version)
        .expect("the sums execute")
    else {
        panic!("expected sum entries");
    };
    let DocumentSumResponse::Proof(proof) = drive
        .execute_document_sum_request(sum_request(true), None, platform_version)
        .expect("the sums prove")
    else {
        panic!("expected a sum proof");
    };
    let index =
        find_summable_index_for_where_clauses(document_type.indexes(), &keyed, "amount", &[])
            .expect("byHashtag sums the elements");
    let (root_hash, proved) = sum_query(index)
        .verify_point_lookup_sum_proof(&proof, platform_version)
        .expect("the sum proof verifies");
    assert_eq!(root_hash, live);
    let sums_by_element = sums(unproved);
    assert_eq!(
        sums(proved),
        sums_by_element,
        "proved and unproved sums agree"
    );

    let totals = |entries: Vec<AverageEntry>| -> BTreeMap<String, (u64, i64)> {
        entries
            .into_iter()
            .map(|entry| {
                (
                    element(entry.in_key, entry.key),
                    (
                        entry.count.expect("a counted element"),
                        entry.sum.expect("a summed element"),
                    ),
                )
            })
            .collect()
    };
    let DocumentAverageResponse::Entries(unproved) = drive
        .execute_document_average_request(average_request(false), None, platform_version)
        .expect("the averages execute")
    else {
        panic!("expected average entries");
    };
    let DocumentAverageResponse::Proof(proof) = drive
        .execute_document_average_request(average_request(true), None, platform_version)
        .expect("the averages prove")
    else {
        panic!("expected an average proof");
    };
    let index = find_summable_index_with_counts_for_where_clauses(
        document_type.indexes(),
        &keyed,
        "amount",
        &[],
    )
    .expect("byHashtag counts and sums the elements");
    let (root_hash, proved) = sum_query(index)
        .verify_point_lookup_count_and_sum_proof(&proof, platform_version)
        .expect("the average proof verifies");
    assert_eq!(root_hash, live);
    let totals_by_element = totals(unproved);
    assert_eq!(
        totals(proved),
        totals_by_element,
        "proved and unproved averages agree"
    );
    assert_eq!(
        totals_by_element
            .iter()
            .map(|(element, (_, sum))| (element.clone(), *sum))
            .collect::<BTreeMap<_, _>>(),
        sums_by_element,
        "the sum and the average surfaces agree"
    );
    totals_by_element
}

/// A summable index naming the array: each element's sum and average are
/// those of the posts holding it, through a replace that keeps the elements
/// and changes the amount (every element's entry refreshes its sum), one that
/// changes the elements, and a delete. A total over several elements is
/// refused.
#[test]
fn should_sum_and_average_each_element_through_updates() {
    let (drive, contract) = setup_with(post_contract_with(platform_value!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "hashtags": hashtags_schema(),
            "amount": {"type": "integer", "minimum": 0, "maximum": 1000, "position": 1}
        },
        "required": ["$createdAt", "amount"],
        "indices": [{
            "name": "byHashtag",
            "properties": [{"hashtags": "asc"}],
            "countable": "countable",
            "summable": "amount"
        }],
        "additionalProperties": false
    })));
    let tip = |id: u8, hashtags: &[&str], amount: u64| {
        let mut document = post(id, Some(hashtags), None);
        document.properties_mut().remove("language");
        document.set("amount", Value::U64(amount));
        document
    };
    let all = ["crypto", "dash", "news"];
    let totals = |expected: [(&str, (u64, i64)); 3]| {
        BTreeMap::from(expected.map(|(element, total)| (element.to_string(), total)))
    };

    insert(&drive, &contract, &tip(1, &["dash", "news"], 10));
    insert(&drive, &contract, &tip(2, &["dash"], 5));
    insert(&drive, &contract, &tip(3, &["news", "crypto"], 7));
    assert_eq!(
        element_totals(&drive, &contract, &all),
        totals([("crypto", (1, 7)), ("dash", (2, 15)), ("news", (2, 17))])
    );

    update(&drive, &contract, &tip(1, &["dash", "news"], 20));
    assert_eq!(
        element_totals(&drive, &contract, &all),
        totals([("crypto", (1, 7)), ("dash", (2, 25)), ("news", (2, 27))])
    );

    update(&drive, &contract, &tip(3, &["crypto", "dash"], 7));
    assert_eq!(
        element_totals(&drive, &contract, &all),
        totals([("crypto", (1, 7)), ("dash", (3, 32)), ("news", (1, 20))])
    );

    delete(&drive, &contract, 2);
    assert_eq!(
        element_totals(&drive, &contract, &all),
        totals([("crypto", (1, 7)), ("dash", (2, 27)), ("news", (1, 20))])
    );

    // One element is a total of its own; several are not.
    let platform_version = PlatformVersion::latest();
    let drive_config = DriveConfig::default();
    let sum = |where_clauses: Vec<WhereClause>| {
        drive.execute_document_sum_request(
            DocumentSumRequest {
                contract: &contract,
                document_type: contract.document_type_for_name("post").expect("post"),
                sum_property: "amount".to_string(),
                where_clauses,
                resolved_time_ranges: vec![],
                order_clauses: vec![],
                mode: SumMode::Aggregate,
                limit: None,
                prove: false,
                drive_config: &drive_config,
            },
            None,
            platform_version,
        )
    };
    match sum(element_clause(
        WhereOperator::Contains,
        Value::Text("dash".to_string()),
    ))
    .expect("one element sums")
    {
        DocumentSumResponse::Aggregate(total) => assert_eq!(total, 27),
        other => panic!("expected an aggregate, got {other:?}"),
    }
    let error = sum(element_clause(
        WhereOperator::ContainsAny,
        platform_value!(["dash", "news"]),
    ))
    .expect_err("a sum over several elements is refused");
    assert!(
        format!("{error:?}").contains("only answered grouped by"),
        "{error:?}"
    );
}

/// A `post` whose `hashtags` are indexed under the widest overlap a window
/// allows, 24 one-day windows, with a property below the array: a post is
/// entered 24 x 4 times.
fn widest_windows_contract() -> DataContract {
    post_contract_with(platform_value!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "hashtags": {
                "type": "array",
                "maxItems": 4,
                "items": {"type": "string", "minLength": 1, "maxLength": 59},
                "position": 0
            },
            "language": {"type": "string", "maxLength": 2, "position": 1}
        },
        "required": ["$createdAt", "language"],
        "indices": [{
            "name": "trendingTwentyFourDays",
            "properties": [{"$createdAt": "asc"}, {"hashtags": "asc"}, {"language": "asc"}],
            "timeRange": {"on": "$createdAt", "range": 2073600, "step": 86400}
        }],
        "additionalProperties": false
    }))
}

/// A `post` whose `hashtags` are indexed under three-day windows stepping
/// daily (`trendingThreeDays`), so a post lies in three windows.
fn trending_three_days_contract() -> DataContract {
    post_contract_with(platform_value!({
        "type": "object",
        "documentsMutable": true,
        "properties": {
            "hashtags": {
                "type": "array",
                "maxItems": 4,
                "items": {"type": "string", "minLength": 1, "maxLength": 59},
                "position": 0
            }
        },
        "required": ["$createdAt"],
        "indices": [{
            "name": "trendingThreeDays",
            "properties": [{"$createdAt": "asc"}, {"hashtags": "asc"}],
            "timeRange": {"on": "$createdAt", "range": 259200, "step": 86400}
        }],
        "additionalProperties": false
    }))
}

/// An array under overlapping windows: a post is entered once per window
/// holding its `$createdAt` and per distinct element, every combination; an
/// update moves the combinations of the elements it changes in every window,
/// and a delete removes them all, pruning an element's trees in every window
/// where no other post holds it.
#[test]
fn should_enter_a_post_per_window_and_element_across_overlapping_windows() {
    let (drive, contract) = setup_with(trending_three_days_contract());
    let document_type = contract.document_type_for_name("post").expect("post");
    let index = document_type
        .indexes()
        .get("trendingThreeDays")
        .expect("the index");
    let level_key = index
        .time_range
        .as_ref()
        .expect("a window")
        .storage_key("$createdAt");
    let created_at = 20_000 * DAY_MS + 1_000;
    let windows = index
        .bucketing()
        .expect("a window")
        .entry_keys_for_raw(&DocumentPropertyType::encode_date_timestamp(created_at));
    assert_eq!(windows.len(), 3, "a day lies in three three-day windows");
    let entry = |window: &[u8], tag: &str, id: u8| {
        has_entry(
            &drive,
            &contract,
            &[
                level_key.as_bytes().to_vec(),
                window.to_vec(),
                text("hashtags"),
                text(tag),
            ],
            id,
        )
    };
    let in_every_window = |tag: &str, id: u8| windows.iter().all(|window| entry(window, tag, id));
    let in_no_window = |tag: &str, id: u8| windows.iter().all(|window| !entry(window, tag, id));

    let mut first = bare_post(1, created_at, &["dash", "news"]);
    insert(&drive, &contract, &first);
    insert(
        &drive,
        &contract,
        &bare_post(2, created_at + 1_000, &["dash"]),
    );
    assert!(in_every_window("dash", 1) && in_every_window("news", 1));
    assert!(in_every_window("dash", 2) && in_no_window("news", 2));

    // Keeps `news`, drops `dash`, adds `crypto`, in every window at once.
    first.set("hashtags", platform_value!(["news", "crypto"]));
    first.set_revision(Some(2));
    update(&drive, &contract, &first);
    assert!(in_no_window("dash", 1));
    assert!(in_every_window("news", 1) && in_every_window("crypto", 1));
    assert!(
        in_every_window("dash", 2),
        "the other post keeps the shared element"
    );

    delete(&drive, &contract, 1);
    assert!(in_no_window("news", 1) && in_no_window("crypto", 1));
    for window in &windows {
        let elements = [
            level_key.as_bytes().to_vec(),
            window.clone(),
            text("hashtags"),
        ];
        assert!(!has_key(&drive, &contract, &elements, &text("news")));
        assert!(!has_key(&drive, &contract, &elements, &text("crypto")));
    }
    assert!(in_every_window("dash", 2));
}

/// An expiring window over an indexed array, with a property below the array.
/// Sustained writes, each copied into 24 windows under 4 distinct elements,
/// drain every expired window as they go: the drain budget counts the array's
/// `maxItems` value trees under its branch, where one value tree would leave
/// it short. A replace and a delete of a post whose oldest windows the drain
/// took neither rebuild them nor fail on the entries it removed, and the
/// live windows keep exactly the post's current elements.
#[test]
fn should_drain_expired_windows_of_an_indexed_array() {
    let platform_version = PlatformVersion::latest();
    let (drive, contract) = setup_with(post_contract_with(platform_value!({
        "type": "object",
        "documentsMutable": true,
        "canBeDeleted": true,
        "properties": {
            "hashtags": {
                "type": "array",
                "maxItems": 4,
                "items": {"type": "string", "minLength": 1, "maxLength": 59},
                "position": 0
            },
            "language": {"type": "string", "maxLength": 59, "position": 1}
        },
        "required": ["$createdAt", "language"],
        "indices": [{
            "name": "windowed",
            "properties": [{"$createdAt": "asc"}, {"hashtags": "asc"}, {"language": "asc"}],
            "timeRange": {"on": "$createdAt", "range": 24, "step": 1, "ttl": 24}
        }],
        "additionalProperties": false
    })));
    let document_type = contract.document_type_for_name("post").expect("post");
    let index = document_type.indexes().get("windowed").expect("the index");
    let transform = index.time_range.as_ref().expect("a window");
    let level_key = transform.storage_key("$createdAt").into_bytes();
    let tx = drive.grove.start_transaction();
    let block = |time_ms: u64| BlockInfo {
        time_ms,
        ..Default::default()
    };
    let tagged = |n: u32, created_at: u64, tags: Vec<String>| {
        let mut id = [1u8; 32];
        id[..4].copy_from_slice(&n.to_be_bytes());
        Document::V0(DocumentV0 {
            id: Identifier::from(id),
            owner_id: Identifier::from(OWNER),
            properties: BTreeMap::from([
                (
                    "hashtags".to_string(),
                    Value::Array(tags.into_iter().map(Value::Text).collect()),
                ),
                ("language".to_string(), Value::Text("en".to_string())),
            ]),
            created_at: Some(created_at),
            revision: Some(1),
            ..Default::default()
        })
    };
    let distinct_tags = |n: u32| {
        (0..4)
            .map(|k| format!("t{}", 4 * n + k))
            .collect::<Vec<_>>()
    };
    let write = |document: &Document, time_ms: u64| {
        create_in(&drive, &contract, document, block(time_ms), true, Some(&tx));
    };
    // How far the oldest window standing lags behind the expiry horizon at
    // `time_ms`: 0 when every expired window is gone.
    let lag = |time_ms: u64| {
        let mut path = contract_document_type_path_vec(contract.id_ref().as_bytes(), "post");
        path.push(level_key.clone());
        let mut query = Query::new();
        query.insert_all();
        let (result, _) = drive
            .grove_get_raw_path_query(
                &PathQuery::new(path, SizedQuery::new(query, Some(1), None)),
                Some(&tx),
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("the windows read");
        let oldest = result
            .to_key_elements()
            .first()
            .and_then(|(key, _)| DocumentPropertyType::decode_date_timestamp(key))
            .expect("a window stands");
        let horizon = transform
            .expiry_horizon_ms(time_ms)
            .expect("the window expires");
        horizon.saturating_sub(oldest)
    };
    // Whether the window `start` stands. The drain drops a window's trees flat, so a
    // read below a dropped window is no evidence: an expired window is judged by this.
    let window_stands = |start: u64| {
        has_key_in(
            &drive,
            &contract,
            std::slice::from_ref(&level_key),
            &DocumentPropertyType::encode_date_timestamp(start),
            Some(&tx),
        )
    };
    // Whether `document`'s entry under `tag` stands in the live window `start`.
    let holds = |document: &Document, start: u64, tag: &str| {
        has_key_in(
            &drive,
            &contract,
            &[
                level_key.clone(),
                DocumentPropertyType::encode_date_timestamp(start),
                text("hashtags"),
                text(tag),
                text("language"),
                text("en"),
                vec![0],
            ],
            &document.id().to_buffer(),
            Some(&tx),
        )
    };
    let windows_of = |created_at: u64| -> Vec<u64> {
        index
            .bucketing()
            .expect("a window")
            .entry_keys_for_raw(&DocumentPropertyType::encode_date_timestamp(created_at))
            .iter()
            .filter_map(|key| DocumentPropertyType::decode_date_timestamp(key))
            .collect()
    };

    let start = 1_000_000u64;
    let mut lags = vec![];
    let mut last_time = start;
    for n in 0..200u32 {
        last_time = start + u64::from(n) * 1_000;
        write(&tagged(n, last_time, distinct_tags(n)), last_time);
        if [95, 150, 199].contains(&n) {
            lags.push(lag(last_time));
        }
    }
    assert_eq!(
        lags,
        vec![0, 0, 0],
        "sustained writes drain every expired window"
    );

    // The newest post whose oldest windows are drained while its newer ones live.
    let now = last_time;
    let horizon = transform
        .expiry_horizon_ms(now)
        .expect("the window expires");
    let (n, expired, live) = (0..200u32)
        .rev()
        .find_map(|n| {
            let (expired, live): (Vec<u64>, Vec<u64>) = windows_of(start + u64::from(n) * 1_000)
                .into_iter()
                .partition(|window| *window < horizon);
            (!expired.is_empty() && !live.is_empty()).then_some((n, expired, live))
        })
        .expect("a post straddles the expiry horizon");
    let partly_drained = tagged(n, start + u64::from(n) * 1_000, distinct_tags(n));
    let kept = format!("t{}", 4 * n);
    let dropped = format!("t{}", 4 * n + 1);
    assert!(expired.iter().all(|window| !window_stands(*window)));
    assert!(live
        .iter()
        .all(|window| holds(&partly_drained, *window, &kept)));

    // A replace keeping one element and changing three: the live windows move,
    // the drained ones stay drained.
    let mut replaced = partly_drained.clone();
    replaced.set(
        "hashtags",
        platform_value!([kept.clone(), "fresh0", "fresh1", "fresh2"]),
    );
    replaced.set_revision(Some(2));
    // A replace over drained windows applies
    replace_in(&drive, &contract, &replaced, block(now), true, Some(&tx));
    for window in &live {
        assert!(holds(&replaced, *window, &kept) && holds(&replaced, *window, "fresh0"));
        assert!(!holds(&replaced, *window, &dropped));
    }
    assert!(
        expired.iter().all(|window| !window_stands(*window)),
        "the replace rebuilt no drained window"
    );

    // A post whose every window the drain took is replaced and deleted without
    // failing on the entries already gone, and without rebuilding any of its
    // windows.
    let mut drained = tagged(10, start + 10_000, distinct_tags(10));
    let drained_windows = windows_of(start + 10_000);
    let all_drained = || drained_windows.iter().all(|window| !window_stands(*window));
    assert!(all_drained());
    drained.set("hashtags", platform_value!(["late"]));
    drained.set_revision(Some(2));
    replace_in(&drive, &contract, &drained, block(now), true, Some(&tx));
    assert!(
        all_drained(),
        "the replace rebuilt none of the drained post's windows"
    );
    delete_in(&drive, &contract, drained.id(), block(now), true, Some(&tx));
    assert!(
        all_drained(),
        "the delete rebuilt none of the drained post's windows"
    );
    delete_in(
        &drive,
        &contract,
        replaced.id(),
        block(now),
        true,
        Some(&tx),
    );
    assert!(live
        .iter()
        .all(|window| !holds(&replaced, *window, "fresh0")));
    assert_eq!(lag(now), 0, "the deletes rebuilt no drained window");
}
