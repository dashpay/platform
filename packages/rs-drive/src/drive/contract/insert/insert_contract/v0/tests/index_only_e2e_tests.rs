//! End-to-end coverage for **indexOnly document types** (meta schema v3 /
//! PV14): the storage mode where a document is never written to primary
//! storage — the index entries ARE the rows, each terminating in an empty
//! `Item` keyed by the index's `terminal` property instead of a `Reference`
//! keyed by the document id.
//!
//! Everything runs against the `yappr-likes` fixture at
//! `tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json`:
//! a `like` doctype with two properties (`hashtag`, `postId`) and three
//! indexes —
//!
//! | index           | properties            | terminal   | axes                        |
//! |-----------------|-----------------------|------------|-----------------------------|
//! | `byHashtagPost` | `[hashtag, postId]`   | `$ownerId` | countable + range + ranked  |
//! | `byPost`        | `[postId]`            | `$ownerId` | countable + range + ranked  |
//! | `byLiker`       | `[$ownerId]`          | `postId`   | none                        |
//!
//! Three layers are pinned. The *registration shape*: no `[0]` primary-key
//! tree is created for the doctype, while the top-level property-name trees
//! (including `byPost`'s `ProvableCountIndexedTree`) are. The *entry
//! layout*: an insert writes one `[…values, 0, <terminal value>] → Item`
//! per index and nothing else, Items count exactly as References under
//! count and ranked trees, and a duplicate entry is refused. And the
//! *delete symmetry*: delete-by-values removes every entry, prunes drained
//! groups so ranked secondaries drop them, and leaves grovedb's integrity
//! sweep clean.

use crate::drive::Drive;
use crate::util::grove_operations::DirectQueryType;
use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
use crate::util::storage_flags::StorageFlags;
use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::random_document::CreateRandomDocument;
use dpp::document::{Document, DocumentV0Setters};
use dpp::fee::fee_result::FeeResult;
use dpp::platform_value::{Identifier, Value};
use dpp::prelude::DataContract;
use dpp::tests::json_document::json_document_to_contract;
use dpp::version::PlatformVersion;
use grovedb::Element;

const DOCTYPE: &str = "like";

fn platform_version() -> &'static PlatformVersion {
    PlatformVersion::latest()
}

fn setup_likes() -> (Drive, DataContract) {
    let drive = setup_drive_with_initial_state_structure(None);
    let pv = platform_version();
    let contract = json_document_to_contract(
        "tests/supporting_files/contract/yappr-likes/yappr-likes-contract.json",
        false,
        pv,
    )
    .expect("expected to parse the yappr-likes contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the yappr-likes contract");
    (drive, contract)
}

/// `[DataContractDocuments, contract_id, 1, "like"]` — the doctype tree.
fn doctype_path(contract: &DataContract) -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        DOCTYPE.as_bytes().to_vec(),
    ]
}

fn read_grove_element(drive: &Drive, path: &[Vec<u8>], key: &[u8]) -> Option<Element> {
    let path_refs: Vec<&[u8]> = path.iter().map(|v| v.as_slice()).collect();
    drive
        .grove_get_raw_optional(
            path_refs.as_slice().into(),
            key,
            DirectQueryType::StatefulDirectQuery,
            None,
            &mut vec![],
            &platform_version().drive,
        )
        .expect("grove_get_raw_optional should succeed")
}

fn assert_grovedb_is_consistent(drive: &Drive) {
    let issues = drive
        .grove
        .verify_grovedb(None, true, false, &platform_version().drive.grove_version)
        .expect("verify_grovedb must run");
    assert!(
        issues.is_empty(),
        "grovedb integrity verification reported issues: {issues:?}"
    );
}

/// A like on `post` under `hashtag` by `owner`.
fn build_like(
    contract: &DataContract,
    hashtag: &str,
    post: [u8; 32],
    owner: [u8; 32],
    seed: u64,
) -> Document {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");
    let mut doc = document_type
        .random_document(Some(seed), pv)
        .expect("random document");
    let mut props = std::collections::BTreeMap::new();
    props.insert("hashtag".to_string(), Value::Text(hashtag.to_string()));
    props.insert("postId".to_string(), Value::Identifier(post));
    doc.set_properties(props);
    doc.set_owner_id(Identifier::from(owner));
    doc
}

fn insert_like(
    drive: &Drive,
    contract: &DataContract,
    doc: &Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");
    drive.add_document_for_contract(
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
        apply,
        None,
        pv,
        None,
    )
}

fn delete_like(
    drive: &Drive,
    contract: &DataContract,
    doc: Document,
    apply: bool,
) -> Result<FeeResult, crate::error::Error> {
    let pv = platform_version();
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");
    drive.delete_index_only_document_for_contract(
        doc,
        contract,
        document_type,
        BlockInfo::default(),
        apply,
        None,
        pv,
        None,
    )
}

fn count_top_k(drive: &Drive, path: &[Vec<u8>], k: u16, descending: bool) -> Vec<(u64, Vec<u8>)> {
    let path_query = grovedb::PathQuery::new_axis(
        path.to_vec(),
        grovedb_query::AxisQuery::top_k(grovedb_query::IndexAxis::Count, k, 0, descending)
            .keys_only(),
    );
    match drive
        .grove
        .run_path_query(
            &path_query,
            true,
            true,
            true,
            grovedb::query_result_type::QueryResultType::QueryKeyElementPairResultType,
            None,
            &platform_version().drive.grove_version,
        )
        .unwrap()
        .expect("the keys-only axis read must succeed")
    {
        grovedb::PathQueryRun::AxisKeys {
            keys: grovedb::AxisKeys::Count(pairs),
            ..
        } => pairs,
        other => panic!("expected count keys, got {other:?}"),
    }
}

const POST_A: [u8; 32] = [0xA1; 32];
const POST_B: [u8; 32] = [0xB2; 32];
const OWNER_1: [u8; 32] = [0x11; 32];
const OWNER_2: [u8; 32] = [0x22; 32];
const OWNER_3: [u8; 32] = [0x33; 32];

// ---------------------------------------------------------------------------
// Registration shape
// ---------------------------------------------------------------------------

/// An indexOnly doctype registers WITHOUT a `[0]` primary-key tree, while
/// its top-level property-name trees exist: `hashtag` and `$ownerId` as
/// plain trees (compound prefix / plain index), `postId` as the
/// `ProvableCountIndexedTree` the single-property ranked `byPost` demands.
#[test]
fn index_only_doctype_registers_without_primary_key_tree() {
    let (drive, contract) = setup_likes();
    let path = doctype_path(&contract);

    assert!(
        read_grove_element(&drive, &path, &[0]).is_none(),
        "an indexOnly doctype must not create the [0] primary-key tree"
    );

    assert!(
        matches!(
            read_grove_element(&drive, &path, b"hashtag"),
            Some(Element::Tree(..))
        ),
        "the compound index's leading property-name tree must exist as a plain tree"
    );
    assert!(
        matches!(
            read_grove_element(&drive, &path, b"$ownerId"),
            Some(Element::Tree(..))
        ),
        "byLiker's leading property-name tree must exist as a plain tree"
    );
    assert!(
        matches!(
            read_grove_element(&drive, &path, b"postId"),
            Some(Element::ProvableCountIndexedTree(..))
        ),
        "byPost's ranked property-name tree must be a ProvableCountIndexedTree"
    );

    // The sibling non-indexOnly doctype in the same contract keeps its
    // primary-key tree — the skip is per-doctype, not per-contract.
    let mut post_path = doctype_path(&contract);
    *post_path.last_mut().expect("path non-empty") = b"post".to_vec();
    assert!(
        read_grove_element(&drive, &post_path, &[0]).is_some(),
        "a stored doctype in the same contract keeps its primary-key tree"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Entry layout
// ---------------------------------------------------------------------------

/// One like writes exactly its three index entries: empty Items keyed by
/// the terminal value, in the docId slot of the ordinary non-unique layout.
#[test]
fn like_insert_writes_terminal_items_for_every_index() {
    let (drive, contract) = setup_likes();
    let like = build_like(&contract, "dash", POST_A, OWNER_1, 1);
    insert_like(&drive, &contract, &like, true).expect("insert like");

    let base = doctype_path(&contract);

    // byHashtagPost: [.., hashtag, "dash", postId, POST_A, 0] key OWNER_1
    let mut by_hashtag_post = base.clone();
    by_hashtag_post.extend([
        b"hashtag".to_vec(),
        b"dash".to_vec(),
        b"postId".to_vec(),
        POST_A.to_vec(),
        vec![0],
    ]);
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");
    let expected_commitment =
        crate::drive::document::index_only_row_commitment(&like, document_type, platform_version())
            .expect("commitment computes");
    match read_grove_element(&drive, &by_hashtag_post, &OWNER_1) {
        Some(Element::Item(data, _)) => {
            assert_eq!(
                data,
                expected_commitment.to_vec(),
                "the terminal item carries the row commitment binding it to the full tuple"
            )
        }
        other => {
            panic!("expected the row-commitment Item at the byHashtagPost terminal, got {other:?}")
        }
    }

    // byPost: [.., postId, POST_A, 0] key OWNER_1
    let mut by_post = base.clone();
    by_post.extend([b"postId".to_vec(), POST_A.to_vec(), vec![0]]);
    match read_grove_element(&drive, &by_post, &OWNER_1) {
        Some(Element::Item(data, _)) => assert_eq!(
            data,
            expected_commitment.to_vec(),
            "every index's entry carries the SAME row commitment"
        ),
        other => panic!("expected the byPost terminal item, got {other:?}"),
    }

    // byLiker: [.., $ownerId, OWNER_1, 0] key POST_A — the generalized
    // terminal: a refersTo-typed property as member key.
    let mut by_liker = base.clone();
    by_liker.extend([b"$ownerId".to_vec(), OWNER_1.to_vec(), vec![0]]);
    assert!(
        matches!(
            read_grove_element(&drive, &by_liker, &POST_A),
            Some(Element::Item(..))
        ),
        "byLiker terminal item must be keyed by the postId value"
    );

    assert_grovedb_is_consistent(&drive);
}

/// The any-entry-exists rule at the storage layer: re-inserting the same
/// like refuses on the first colliding entry.
#[test]
fn duplicate_like_insert_is_refused() {
    let (drive, contract) = setup_likes();
    let like = build_like(&contract, "dash", POST_A, OWNER_1, 1);
    insert_like(&drive, &contract, &like, true).expect("first insert");

    let error = insert_like(&drive, &contract, &like, true)
        .expect_err("the identical like must be refused");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(crate::error::drive::DriveError::CorruptedContractIndexes(
                _
            ))
        ),
        "expected the CorruptedContractIndexes backstop, got: {error}"
    );

    // A different owner liking the same post is a different member key and
    // must go through.
    let like_2 = build_like(&contract, "dash", POST_A, OWNER_2, 2);
    insert_like(&drive, &contract, &like_2, true).expect("a second owner may like the same post");

    assert_grovedb_is_consistent(&drive);
}

/// Items count exactly as references: group counts and the ranked Count
/// axis order posts by their number of likes, globally and per hashtag.
#[test]
fn likes_count_and_rank_posts() {
    let (drive, contract) = setup_likes();

    // POST_A: 2 likes under #dash; POST_B: 1 like under #dash.
    for (post, owner, seed) in [
        (POST_A, OWNER_1, 1u64),
        (POST_A, OWNER_2, 2),
        (POST_B, OWNER_3, 3),
    ] {
        let like = build_like(&contract, "dash", post, owner, seed);
        insert_like(&drive, &contract, &like, true).expect("insert like");
    }

    let base = doctype_path(&contract);

    // Group counts on the byPost value trees.
    let mut by_post_level = base.clone();
    by_post_level.push(b"postId".to_vec());
    match read_grove_element(&drive, &by_post_level, &POST_A) {
        Some(Element::CountTree(_, count, _)) => assert_eq!(count, 2),
        other => panic!("expected POST_A's group to be a CountTree, got {other:?}"),
    }

    // Global ranking: POST_A (2) before POST_B (1).
    let global = count_top_k(&drive, &by_post_level, 10, true);
    assert_eq!(
        global,
        vec![(2, POST_A.to_vec()), (1, POST_B.to_vec())],
        "byPost's Count axis must rank posts by like count"
    );

    // Per-hashtag ranking through the compound index's terminal level.
    let mut per_hashtag_level = base.clone();
    per_hashtag_level.extend([b"hashtag".to_vec(), b"dash".to_vec(), b"postId".to_vec()]);
    let per_hashtag = count_top_k(&drive, &per_hashtag_level, 10, true);
    assert_eq!(
        per_hashtag,
        vec![(2, POST_A.to_vec()), (1, POST_B.to_vec())],
        "byHashtagPost's Count axis must rank posts within the hashtag"
    );

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Delete symmetry
// ---------------------------------------------------------------------------

/// Delete-by-values removes every entry; a drained group prunes away and
/// the ranked secondary drops it.
#[test]
fn like_delete_removes_entries_and_prunes_drained_groups() {
    let (drive, contract) = setup_likes();

    let like_a1 = build_like(&contract, "dash", POST_A, OWNER_1, 1);
    let like_a2 = build_like(&contract, "dash", POST_A, OWNER_2, 2);
    let like_b3 = build_like(&contract, "dash", POST_B, OWNER_3, 3);
    for like in [&like_a1, &like_a2, &like_b3] {
        insert_like(&drive, &contract, like, true).expect("insert like");
    }

    let base = doctype_path(&contract);
    let mut by_post_level = base.clone();
    by_post_level.push(b"postId".to_vec());

    // Unlike POST_B's only like: the group must vanish entirely.
    delete_like(&drive, &contract, like_b3, true).expect("delete like");
    assert!(
        read_grove_element(&drive, &by_post_level, &POST_B).is_none(),
        "POST_B's drained group must prune away"
    );
    assert_eq!(
        count_top_k(&drive, &by_post_level, 10, true),
        vec![(2, POST_A.to_vec())],
        "the ranked secondary must drop the drained group"
    );

    // Unlike one of POST_A's likes: count drops, group stays.
    delete_like(&drive, &contract, like_a2, true).expect("delete like");
    assert_eq!(
        count_top_k(&drive, &by_post_level, 10, true),
        vec![(1, POST_A.to_vec())]
    );

    // The byLiker entries mirror: OWNER_2's entry is gone, OWNER_1's stays.
    let mut owner_2_likes = base.clone();
    owner_2_likes.extend([b"$ownerId".to_vec(), OWNER_2.to_vec(), vec![0]]);
    assert!(read_grove_element(&drive, &owner_2_likes, &POST_A).is_none());
    let mut owner_1_likes = base.clone();
    owner_1_likes.extend([b"$ownerId".to_vec(), OWNER_1.to_vec(), vec![0]]);
    assert!(read_grove_element(&drive, &owner_1_likes, &POST_A).is_some());

    assert_grovedb_is_consistent(&drive);
}

// ---------------------------------------------------------------------------
// Estimation
// ---------------------------------------------------------------------------

/// The `apply: false` dry-run must work (every traversed layer emits its
/// estimation info) and must not under-estimate the applied storage fee —
/// the invariant consensus fee validation depends on.
#[test]
fn estimated_fees_upper_bound_actual_fees() {
    let (drive, contract) = setup_likes();
    let like = build_like(&contract, "dash", POST_A, OWNER_1, 1);

    let estimated_insert =
        insert_like(&drive, &contract, &like, false).expect("estimated insert must work");
    let actual_insert = insert_like(&drive, &contract, &like, true).expect("actual insert");
    assert!(
        estimated_insert.storage_fee >= actual_insert.storage_fee,
        "estimated insert storage fee {} must upper-bound actual {}",
        estimated_insert.storage_fee,
        actual_insert.storage_fee
    );

    let estimated_delete =
        delete_like(&drive, &contract, like.clone(), false).expect("estimated delete must work");
    let actual_delete = delete_like(&drive, &contract, like, true).expect("actual delete");
    // Deletes refund storage; both processing fees must be present and the
    // estimation path must simply not error or under-run the layer sweep.
    assert!(estimated_delete.processing_fee > 0);
    assert!(actual_delete.processing_fee > 0);

    assert_grovedb_is_consistent(&drive);
}

/// Terminal items carry owner-and-epoch storage flags, so deleting them in
/// a later epoch must refund the freed bytes to the inserting owner,
/// attributed to the insertion epoch — the same refund contract stored
/// documents follow.
#[test]
fn deletion_refunds_flow_from_terminal_item_flags() {
    use dpp::block::epoch::Epoch;
    use dpp::fee::default_costs::{CachedEpochIndexFeeVersions, EpochCosts};
    use dpp::version::fee::FeeVersion;
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    let (drive, contract) = setup_likes();
    let document_type = contract
        .document_type_for_name(DOCTYPE)
        .expect("like doctype exists");
    let previous_fee_versions: CachedEpochIndexFeeVersions =
        BTreeMap::from([(0, FeeVersion::first())]);

    let like = build_like(&contract, "dash", POST_A, OWNER_1, 1);

    // Insert at epoch 0 with owner-attributed flags — what the production
    // create-operation converter writes.
    let pv = platform_version();
    drive
        .add_document_for_contract(
            crate::util::object_size_info::DocumentAndContractInfo {
                owned_document_info: crate::util::object_size_info::OwnedDocumentInfo {
                    document_info: crate::util::object_size_info::DocumentInfo::DocumentRefInfo((
                        &like,
                        Some(Cow::Owned(StorageFlags::SingleEpochOwned(0, OWNER_1))),
                    )),
                    owner_id: None,
                },
                contract: &contract,
                document_type,
            },
            false,
            BlockInfo::default(),
            true,
            None,
            pv,
            None,
        )
        .expect("insert like with owned flags");

    // Delete at a later epoch: the freed bytes must come back to OWNER_1,
    // attributed to insertion epoch 0.
    let later_epoch_block = BlockInfo {
        epoch: Epoch::new(1).expect("epoch 1"),
        ..Default::default()
    };
    let fee_result = drive
        .delete_index_only_document_for_contract(
            like,
            &contract,
            document_type,
            later_epoch_block,
            true,
            None,
            pv,
            Some(&previous_fee_versions),
        )
        .expect("delete like in a later epoch");

    let refund = fee_result
        .fee_refunds
        .calculate_refunds_amount_for_identity(dpp::identifier::Identifier::from(OWNER_1))
        .expect("the refund must be attributed to the inserting owner");
    assert!(
        refund > 0,
        "deleting owner-flagged terminal items must refund storage to the owner"
    );

    assert_grovedb_is_consistent(&drive);
}

/// Row integrity: a values tuple spliced from two of the SAME owner's
/// documents addresses entries that all exist — but they carry different
/// row commitments, so the delete is refused and both documents keep every
/// projection. (The `mark` doctype's two single-property indexes are the
/// splice-prone shape; likes are immune because their compound index binds
/// the values together, but the layout must not depend on that.)
#[test]
fn spliced_delete_across_two_rows_is_refused() {
    use dpp::document::DocumentV0Setters;
    use dpp::platform_value::Value;

    let (drive, contract) = setup_likes();
    let mark_type = contract
        .document_type_for_name("mark")
        .expect("mark doctype exists");

    let build_mark = |a: &str, b: &str, seed: u64| {
        let mut doc = mark_type
            .random_document(Some(seed), platform_version())
            .expect("random mark");
        let mut props = std::collections::BTreeMap::new();
        props.insert("a".to_string(), Value::Text(a.to_string()));
        props.insert("b".to_string(), Value::Text(b.to_string()));
        doc.set_properties(props);
        doc.set_owner_id(dpp::identifier::Identifier::from(OWNER_1));
        doc
    };
    let insert_mark = |doc: &dpp::document::Document| {
        drive
            .add_document_for_contract(
                crate::util::object_size_info::DocumentAndContractInfo {
                    owned_document_info: crate::util::object_size_info::OwnedDocumentInfo {
                        document_info: crate::util::object_size_info::DocumentInfo::DocumentRefInfo(
                            (doc, None),
                        ),
                        owner_id: None,
                    },
                    contract: &contract,
                    document_type: mark_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version(),
                None,
            )
            .expect("insert mark");
    };

    let mark_1 = build_mark("a1", "b1", 1);
    let mark_2 = build_mark("a2", "b2", 2);
    insert_mark(&mark_1);
    insert_mark(&mark_2);

    // The spliced tuple (a1, b2): byA's entry exists (mark_1's), byB's
    // entry exists (mark_2's) — but each carries its own row's commitment.
    let spliced = build_mark("a1", "b2", 3);
    let error = drive
        .delete_index_only_document_for_contract(
            spliced,
            &contract,
            mark_type,
            BlockInfo::default(),
            true,
            None,
            platform_version(),
            None,
        )
        .expect_err("a spliced tuple must be refused");
    assert!(
        matches!(
            error,
            crate::error::Error::Drive(
                crate::error::drive::DriveError::DeletingDocumentThatDoesNotExist(_)
            )
        ),
        "expected the row-integrity refusal, got: {error}"
    );

    // Both real rows survive intact and stay individually deletable.
    drive
        .delete_index_only_document_for_contract(
            mark_1,
            &contract,
            mark_type,
            BlockInfo::default(),
            true,
            None,
            platform_version(),
            None,
        )
        .expect("the real row must still delete");
    drive
        .delete_index_only_document_for_contract(
            mark_2,
            &contract,
            mark_type,
            BlockInfo::default(),
            true,
            None,
            platform_version(),
            None,
        )
        .expect("the other real row must still delete");

    assert_grovedb_is_consistent(&drive);
}
