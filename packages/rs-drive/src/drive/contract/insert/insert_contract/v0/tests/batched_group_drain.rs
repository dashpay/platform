//! What several document operations in **one** GroveDB batch do to a ranked
//! index group they jointly empty.
//!
//! `BatchTransitionAction::into_high_level_drive_operations` flattens every
//! transition of a batch state transition into one `Vec<DriveOperation>`, and
//! `apply_drive_operations` converts each of those independently: the ordinary
//! Add / Update / Delete arms hand `batch_delete_up_tree_while_empty` no
//! sibling operations, so every conversion decides whether a group tree has
//! become empty from committed state alone. Two deletes that jointly empty a
//! group therefore each observe the other's document still committed, each
//! conclude the group is not empty, and the group tree survives with nothing
//! behind it. On a ranked index that leftover tree is mirrored into the
//! aggregate secondary, so the group keeps ranking at zero — sorting ahead of
//! every group with a positive aggregate — while primary and secondary agree
//! it exists, which means `verify_grovedb` is clean and the proof reconstructs
//! the live root hash. A wrong ranking that verifies.
//!
//! **That defect is real, and no batch state transition can reach it.**
//! `max_transitions_in_documents_batch` is 1 at every protocol version, so a
//! batch state transition carries at most one document transition, and two
//! transitions in the same block become two separate GroveDB batches
//! (`execute_event` calls `apply_drive_operations` once per state transition).
//! The cases that expose the defect are therefore `#[ignore]`d rather than
//! deleted or weakened: they are the specification of one hazard raising that
//! cap would have to fix — not of the whole shape, since a real multi-transition
//! batch would also fold several writes to the same identity-contract nonce key
//! into one batch. The cases that run pin what the system relies on today — the
//! phantom is characterized as it actually is, the inverse shape fails loud,
//! and the sequential controls drain correctly.
//!
//! The cap is not the only guard of its kind. `update_contract_keywords` builds
//! its own multi-document blind batch over one shared index group and is kept
//! correct by a guard in its caller rather than by this cap; see
//! `clearing_every_keyword_leaves_an_empty_by_contract_id_group_behind` in that
//! module for what that guard is worth.
//!
//! Every case drives its mutations through the same `apply_drive_operations`
//! call a batch transition produces, rather than through the one-op-per-call
//! `add/update/delete_document_for_contract` helpers the rest of this suite
//! uses. That difference is the whole point. Only the document operations are
//! reproduced, not the `UpdateIdentityContractNonce` operations a real
//! transition prepends: those touch the identity tree and have no bearing on
//! index maintenance.

use super::*;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_ranked_query::index_picker::find_ranked_index_for_axis;
use crate::query::{
    DocumentRankedRequest, DocumentRankedResponse, DriveDocumentRankedQuery, OrderClause,
    RankedAxis, RankedEntry, RankedEntryValue, RankedPage, SelectProjection,
    RANKED_COUNT_ORDER_KEY,
};
use crate::util::batch::{DocumentOperationType, DriveOperation};
use crate::util::object_size_info::{DataContractInfo, DocumentTypeInfo};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::prelude::Identifier;

/// The group under test — the one that gets emptied.
const G: &str = "beta";
/// The bystander group, so the index is never globally empty.
const H: &str = "alpha";

// ---------------------------------------------------------------------------
// One test body per ranking axis
// ---------------------------------------------------------------------------

/// The fixture carries one doctype per axis (two ranked indexes on the same
/// property of one doctype is a `DuplicateIndexError`), so "run this scenario
/// on all three axes" means "run it on three doctypes".
#[derive(Clone, Copy, Debug)]
struct Axis {
    doctype: &'static str,
    /// The document property the index aggregates.
    property: &'static str,
    ranked: RankedAxis,
}

const AXES: [Axis; 3] = [
    Axis {
        doctype: "visit",
        property: "guests",
        ranked: RankedAxis::Count,
    },
    Axis {
        doctype: "tip",
        property: "amount",
        ranked: RankedAxis::Sum,
    },
    Axis {
        doctype: "review",
        property: "grade",
        ranked: RankedAxis::Avg,
    },
];

impl Axis {
    fn select(&self) -> SelectProjection {
        match self.ranked {
            RankedAxis::Count => SelectProjection::count_star(),
            RankedAxis::Sum => SelectProjection::sum(self.property),
            RankedAxis::Avg => SelectProjection::avg(self.property),
        }
    }

    /// `ORDER BY` must name the selected aggregate: the `$count` sentinel for
    /// `COUNT(*)`, the aggregated property otherwise.
    fn order_field(&self) -> &'static str {
        match self.ranked {
            RankedAxis::Count => RANKED_COUNT_ORDER_KEY,
            _ => self.property,
        }
    }

    /// What the index picker matches on — empty for `COUNT(*)`.
    fn aggregate_field(&self) -> &'static str {
        match self.ranked {
            RankedAxis::Count => "",
            _ => self.property,
        }
    }

    /// The aggregate a group of `values` must show on this axis.
    fn expected_value(&self, values: &[i64]) -> RankedEntryValue {
        let sum: i64 = values.iter().sum();
        match self.ranked {
            RankedAxis::Count => RankedEntryValue::Count(values.len() as u64),
            RankedAxis::Sum => RankedEntryValue::Sum(sum),
            RankedAxis::Avg => {
                RankedEntryValue::AvgFixedPoint(expected_avg_fixed_point(sum, values.len() as u64))
            }
        }
    }

    /// The aggregate a group with no documents left behind it shows.
    /// `expected_value` cannot express this: an average over zero documents is
    /// a division by zero, whereas the phantom reports a flat zero.
    fn phantom_value(&self) -> RankedEntryValue {
        match self.ranked {
            RankedAxis::Count => RankedEntryValue::Count(0),
            RankedAxis::Sum => RankedEntryValue::Sum(0),
            RankedAxis::Avg => RankedEntryValue::AvgFixedPoint(0),
        }
    }

    /// The `(count, sum)` a phantom's surviving primary value tree carries.
    /// Which halves are present is decided by the index's countability and
    /// summability, so it differs per axis.
    fn phantom_primary(&self) -> GroupAggregate {
        match self.ranked {
            RankedAxis::Count => Some((Some(0), None)),
            RankedAxis::Sum => Some((None, Some(0))),
            RankedAxis::Avg => Some((Some(0), Some(0))),
        }
    }

    /// Group keys straight out of grovedb's secondary, bypassing the query
    /// layer entirely — so a query-layer filter cannot hide a phantom.
    fn raw_group_keys(&self, drive: &Drive, path: &[Vec<u8>], descending: bool) -> Vec<String> {
        match self.ranked {
            RankedAxis::Count => group_keys(&count_top_k(drive, path, 100, descending)),
            RankedAxis::Sum => group_keys(&sum_top_k(drive, path, 100, descending)),
            RankedAxis::Avg => group_keys(&avg_top_k(drive, path, 100, descending)),
        }
    }
}

// ---------------------------------------------------------------------------
// Batch plumbing — the operations a batch transition flattens into
// ---------------------------------------------------------------------------

fn delete_op<'a>(
    contract: &'a DataContract,
    doctype: &'a str,
    document_id: Identifier,
) -> DriveOperation<'a> {
    DriveOperation::DocumentOperation(DocumentOperationType::DeleteDocument {
        document_id,
        contract_info: DataContractInfo::BorrowedDataContract(contract),
        document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr(doctype),
    })
}

fn add_op<'a>(
    contract: &'a DataContract,
    doctype: &'a str,
    document: &'a Document,
) -> DriveOperation<'a> {
    DriveOperation::DocumentOperation(DocumentOperationType::AddDocument {
        owned_document_info: OwnedDocumentInfo {
            document_info: DocumentRefInfo((document, None)),
            owner_id: Some(document.owner_id().to_buffer()),
        },
        contract_info: DataContractInfo::BorrowedDataContract(contract),
        document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr(doctype),
        override_document: false,
    })
}

fn update_op<'a>(
    contract: &'a DataContract,
    doctype: &'a str,
    document: &'a Document,
) -> DriveOperation<'a> {
    DriveOperation::DocumentOperation(DocumentOperationType::UpdateDocument {
        owned_document_info: OwnedDocumentInfo {
            document_info: DocumentRefInfo((document, None)),
            owner_id: Some(document.owner_id().to_buffer()),
        },
        contract_info: DataContractInfo::BorrowedDataContract(contract),
        document_type_info: DocumentTypeInfo::DocumentTypeNameAsStr(doctype),
    })
}

fn apply_batch(drive: &Drive, operations: Vec<DriveOperation>) {
    drive
        .apply_drive_operations(
            operations,
            true,
            &BlockInfo::default(),
            None,
            platform_version(),
            None,
        )
        .expect("the batch must apply");
}

// ---------------------------------------------------------------------------
// Reading the result back — raw, unproved, and proved
// ---------------------------------------------------------------------------

/// A group's primary aggregates: `(count, sum)`, each present only on the axes
/// that record it, and `None` altogether when the group has no value tree.
type GroupAggregate = Option<(Option<u64>, Option<i64>)>;

/// The group's `(count, sum)` as recorded on its primary value tree, with the
/// Merk root key deliberately dropped.
///
/// Tree *shape* is history-dependent, so the root key — and the app hash above
/// it — legitimately differ between a state reached in one batch and the same
/// state reached in several. The aggregates do not, which is why they are what
/// the batched-versus-sequential comparisons below use.
fn primary_group_aggregate(drive: &Drive, path: &[Vec<u8>], group: &str) -> GroupAggregate {
    read_grove_element(drive, path, group.as_bytes()).map(|element| match element {
        Element::ProvableCountProvableSumTree(_, count, sum, _) => (Some(count), Some(sum)),
        Element::CountTree(_, count, _) => (Some(count), None),
        Element::SumTree(_, sum, _) => (None, Some(sum)),
        other => panic!("unexpected group element under a ranked primary: {other:?}"),
    })
}

/// Everything about a ranked doctype that must be identical however the
/// mutations were batched: the full ranked entries in both directions (keys
/// *and* values), and each group's presence plus primary aggregates.
///
/// Deliberately **not** the root hash — see [`primary_group_aggregate`].
#[derive(Debug, PartialEq)]
struct LogicalState {
    descending: Vec<RankedEntry>,
    ascending: Vec<RankedEntry>,
    primary: Vec<(String, GroupAggregate)>,
}

fn logical_state(drive: &Drive, contract: &DataContract, axis: Axis) -> LogicalState {
    let path = indexed_property_name_tree_path(contract, axis.doctype);
    LogicalState {
        descending: entries_of(run(drive, contract, axis, false, false)),
        ascending: entries_of(run(drive, contract, axis, true, false)),
        primary: [G, H]
            .iter()
            .map(|group| {
                (
                    group.to_string(),
                    primary_group_aggregate(drive, &path, group),
                )
            })
            .collect(),
    }
}

fn grovedb_root_hash(drive: &Drive) -> [u8; 32] {
    drive
        .grove
        .root_hash(None, &platform_version().drive.grove_version)
        .unwrap()
        .expect("root hash must be readable")
}

/// `SELECT <agg> GROUP BY restaurantId ORDER BY <agg> [ASC|DESC] LIMIT 100
/// OFFSET 0` through the public dispatcher — the same call drive-abci makes.
fn run(
    drive: &Drive,
    contract: &DataContract,
    axis: Axis,
    ascending: bool,
    prove: bool,
) -> DocumentRankedResponse {
    let group_by = vec![GROUP_PROPERTY.to_string()];
    let order_by = vec![OrderClause {
        field: axis.order_field().to_string(),
        ascending,
    }];
    drive
        .execute_document_ranked_request(
            DocumentRankedRequest {
                contract,
                document_type: contract
                    .document_type_for_name(axis.doctype)
                    .expect("doctype exists"),
                group_by: &group_by,
                select: axis.select(),
                having: &[],
                order_by: &order_by,
                where_clauses: &[],
                limit: Some(100),
                offset: Some(0),
                has_start_at: false,
                prove,
                resolved_time_ranges: &[],
            },
            None,
            platform_version(),
        )
        .expect("the ranked request must succeed")
}

fn entries_of(response: DocumentRankedResponse) -> Vec<RankedEntry> {
    match response {
        DocumentRankedResponse::Entries(page) => page.entries,
        DocumentRankedResponse::Proof(_) => panic!("expected entries, got a proof"),
    }
}

fn proof_of(response: DocumentRankedResponse) -> Vec<u8> {
    match response {
        DocumentRankedResponse::Proof(proof) => proof,
        DocumentRankedResponse::Entries(_) => panic!("expected a proof, got entries"),
    }
}

fn entry_keys(entries: &[RankedEntry]) -> Vec<String> {
    entries
        .iter()
        .map(|entry| String::from_utf8(entry.key.clone()).expect("fixture group keys are utf-8"))
        .collect()
}

/// Prove the same page, verify it, and compare the **reconstructed root hash**
/// against the live one rather than merely asserting that verification
/// succeeded: a proof can verify against a root hash that is not the chain's.
fn verified_page(
    drive: &Drive,
    contract: &DataContract,
    axis: Axis,
    ascending: bool,
) -> RankedPage {
    let proof = proof_of(run(drive, contract, axis, ascending, true));
    let indexes = contract
        .document_types()
        .get(axis.doctype)
        .expect("doctype exists")
        .indexes();
    let query = DriveDocumentRankedQuery {
        document_type: contract
            .document_type_for_name(axis.doctype)
            .expect("doctype exists"),
        contract_id: contract.id().to_buffer(),
        document_type_name: axis.doctype.to_string(),
        index: find_ranked_index_for_axis(
            indexes,
            GROUP_PROPERTY,
            &[],
            axis.ranked,
            axis.aggregate_field(),
        )
        .expect("the fixture declares this axis"),
        axis: axis.ranked,
        prefix_branches: vec![Vec::new()],
        descending: !ascending,
        k: 100,
        offset: 0,
    };
    let (root_hash, page) = query
        .verify_ranked_top_k_proof(&proof, platform_version())
        .expect("the proof must verify");
    assert_eq!(
        root_hash,
        grovedb_root_hash(drive),
        "the proof must reconstruct the live grovedb root hash"
    );
    page
}

/// The full observation battery every case ends with: both directions,
/// unproved and proved, plus the primary tree and grovedb's own integrity
/// sweep.
///
/// `expected_descending` maps group key → the values still contributing to it,
/// in ranking order. A group that must be gone is simply absent.
fn assert_ranking_is(
    drive: &Drive,
    contract: &DataContract,
    axis: Axis,
    expected_descending: &[(&str, &[i64])],
    context: &str,
) {
    let path = indexed_property_name_tree_path(contract, axis.doctype);

    let descending_keys: Vec<String> = expected_descending
        .iter()
        .map(|(key, _)| key.to_string())
        .collect();
    let ascending_keys: Vec<String> = descending_keys.iter().rev().cloned().collect();

    for (ascending, expected_keys) in [(false, &descending_keys), (true, &ascending_keys)] {
        let direction = if ascending { "ASC" } else { "DESC" };

        // Raw secondary, no query layer in the way.
        assert_eq!(
            &axis.raw_group_keys(drive, &path, !ascending),
            expected_keys,
            "{context}: raw {direction} secondary for the {:?} axis",
            axis.ranked
        );

        // Unproved dispatcher read.
        let entries = entries_of(run(drive, contract, axis, ascending, false));
        assert_eq!(
            &entry_keys(&entries),
            expected_keys,
            "{context}: unproved {direction} ranking for the {:?} axis",
            axis.ranked
        );
        for (entry, (key, values)) in entries.iter().zip(if ascending {
            expected_descending.iter().rev().collect::<Vec<_>>()
        } else {
            expected_descending.iter().collect::<Vec<_>>()
        }) {
            assert_eq!(
                entry.value,
                axis.expected_value(values),
                "{context}: {direction} aggregate for group {key} on the {:?} axis",
                axis.ranked
            );
        }

        // Proved path, root hash compared.
        let verified = verified_page(drive, contract, axis, ascending);
        assert_eq!(
            verified.entries, entries,
            "{context}: the proved {direction} page must equal the unproved one \
             for the {:?} axis",
            axis.ranked
        );
    }

    // The primary side: a group with no documents must have no value tree.
    for group in [G, H] {
        let present = expected_descending.iter().any(|(key, _)| *key == group);
        assert_eq!(
            read_grove_element(drive, &path, group.as_bytes()).is_some(),
            present,
            "{context}: group {group}'s primary value tree presence on the {:?} axis \
             must match its presence in the ranking",
            axis.ranked
        );
    }

    assert_grovedb_is_consistent(drive);
}

/// Insert `(group, value, seed)` rows with explicit seeds so two independently
/// built drives get byte-identical documents.
fn insert_seeded(
    drive: &Drive,
    contract: &DataContract,
    axis: Axis,
    rows: &[(&str, i64, u64)],
) -> Vec<Document> {
    rows.iter()
        .map(|(group, value, seed)| {
            let doc = build_doc(contract, axis.doctype, axis.property, group, *value, *seed);
            insert_doc(drive, contract, axis.doctype, &doc);
            doc
        })
        .collect()
}

/// The fixture on a Drive configured the way a **shipped node** is.
///
/// `setup_drive_with_initial_state_structure` forces
/// `batching_consistency_verification: true`, but the shipped default is
/// `false` (`DEFAULT_GROVE_BATCHING_CONSISTENCY_VERIFICATION_ENABLED`), and
/// with it off Drive additionally hands grovedb
/// `disable_operation_consistency_check: true`. The two configurations reject
/// a malformed batch at different places, so a batch the test configuration
/// refuses has to be re-run here before that refusal can be called the
/// system's real behaviour.
fn setup_restaurants_with_shipped_batching_config() -> (Drive, DataContract) {
    use crate::config::DriveConfig;
    use crate::util::test_helpers::setup::setup_drive;

    let pv = platform_version();
    let drive = setup_drive(Some(DriveConfig::default()));
    assert!(
        !drive.config.batching_consistency_verification,
        "this fixture exists to exercise the shipped default"
    );
    drive
        .create_initial_state_structure(None, pv)
        .expect("should create root tree successfully");
    let contract = dpp::tests::json_document::json_document_to_contract(
        "tests/supporting_files/contract/restaurants/restaurants-contract.json",
        false,
        pv,
    )
    .expect("expected to parse the restaurants contract");
    drive
        .apply_contract(
            &contract,
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            pv,
        )
        .expect("expected to apply the restaurants contract");
    (drive, contract)
}

/// `H` holds one document worth 10; `G` holds two, worth 20 and 30.
/// Returned in that order.
fn setup_g_two_h_one(axis: Axis) -> (Drive, DataContract, Vec<Document>) {
    let (drive, contract) = setup_restaurants();
    let docs = insert_seeded(
        &drive,
        &contract,
        axis,
        &[(H, 10, 1), (G, 20, 2), (G, 30, 3)],
    );
    (drive, contract, docs)
}

/// The population every drain case starts from, asserted before the mutation
/// so a failure afterwards cannot be blamed on a bad fixture.
fn assert_baseline(drive: &Drive, contract: &DataContract, axis: Axis) {
    // On every axis G(20,30) outranks H(10): count 2>1, sum 50>10, avg 25>10.
    assert_ranking_is(
        drive,
        contract,
        axis,
        &[(G, &[20, 30]), (H, &[10])],
        "baseline",
    );
}

// ---------------------------------------------------------------------------
// Two last deletes in one batch — the core defect
// ---------------------------------------------------------------------------

/// **This test asserts the defect, not the desired behaviour.** It is green,
/// and it is the evidence behind the severity claim documented on
/// `SystemLimits::max_transitions_in_documents_batch`: that the group a
/// batched drain leaves behind is not a detectable inconsistency but a
/// perfectly self-consistent lie. Every claim in that documentation is checked
/// here on every axis — the group is present with a zero aggregate, it sorts
/// *first* on ascending order, both its documents really are gone from primary
/// storage, grovedb's integrity sweep reports nothing, and the proved page
/// both equals the unproved one and reconstructs the live root hash.
///
/// It goes red the day the defect is fixed, or the day grovedb starts
/// detecting it. That is the intended signal, not a regression: delete this
/// test then, and un-ignore the cases below.
#[test]
fn a_batched_drain_leaves_a_phantom_group_that_verifies_and_proves() {
    for axis in AXES {
        let (drive, contract, docs) = setup_g_two_h_one(axis);
        assert_baseline(&drive, &contract, axis);

        apply_batch(
            &drive,
            vec![
                delete_op(&contract, axis.doctype, docs[1].id()),
                delete_op(&contract, axis.doctype, docs[2].id()),
            ],
        );

        let path = indexed_property_name_tree_path(&contract, axis.doctype);
        let descending = entries_of(run(&drive, &contract, axis, false, false));
        let ascending = entries_of(run(&drive, &contract, axis, true, false));

        assert_eq!(
            entry_keys(&descending),
            vec![H.to_string(), G.to_string()],
            "{:?}: the drained group must still be ranked — that is the defect",
            axis.ranked
        );
        assert_eq!(
            entry_keys(&ascending),
            vec![G.to_string(), H.to_string()],
            "{:?}: and it must sort ahead of a group with a positive aggregate ascending, \
             so a BOTTOM(k) query returns it first",
            axis.ranked
        );
        assert_eq!(
            descending.last().expect("the phantom is ranked").value,
            axis.phantom_value(),
            "{:?}: the phantom's aggregate",
            axis.ranked
        );
        assert_eq!(
            primary_group_aggregate(&drive, &path, G),
            axis.phantom_primary(),
            "{:?}: the primary value tree survives the drain, empty",
            axis.ranked
        );

        for document in &docs[1..3] {
            assert!(
                !document_is_stored(&drive, &contract, axis, document.id()),
                "{:?}: the documents really are deleted — the group ranks with nothing \
                 behind it",
                axis.ranked
            );
        }

        let issues = drive
            .grove
            .verify_grovedb(None, true, false, &platform_version().drive.grove_version)
            .expect("verify_grovedb must run");
        assert!(
            issues.is_empty(),
            "{:?}: integrity verification is structurally incapable of catching this — \
             primary and secondary agree that an empty group exists; got {issues:?}",
            axis.ranked
        );

        for (direction_ascending, expected) in [(false, &descending), (true, &ascending)] {
            // `verified_page` compares the reconstructed root hash against the
            // live one, so this is the claim that the phantom proves against
            // the real chain state rather than merely verifying.
            assert_eq!(
                &verified_page(&drive, &contract, axis, direction_ascending).entries,
                expected,
                "{:?}: the proof attests the phantom",
                axis.ranked
            );
        }
    }
}

/// **Known latent defect.** Both of `G`'s documents are deleted in one
/// `apply_drive_operations` call. Neither delete can see the other, so neither
/// removes `G`'s group tree: `G` stays in the ranked secondary with a zero
/// aggregate and no documents behind it, on all three axes, with
/// `verify_grovedb` clean and the proof reconstructing the live root hash.
///
/// Unreachable today because `max_transitions_in_documents_batch` is 1, which
/// keeps any two document operations out of a shared GroveDB batch. Raising
/// that cap arms this; see the documentation on
/// `SystemLimits::max_transitions_in_documents_batch`.
#[test]
#[ignore = "documents a latent defect in shared write-path machinery that is unreachable while max_transitions_in_documents_batch is 1; un-ignore before raising that cap"]
fn deleting_a_groups_last_two_documents_in_one_batch_removes_the_group() {
    for axis in AXES {
        let (drive, contract, docs) = setup_g_two_h_one(axis);
        assert_baseline(&drive, &contract, axis);

        // The whole experiment: both deletes in ONE apply_drive_operations call.
        apply_batch(
            &drive,
            vec![
                delete_op(&contract, axis.doctype, docs[1].id()),
                delete_op(&contract, axis.doctype, docs[2].id()),
            ],
        );

        assert_ranking_is(
            &drive,
            &contract,
            axis,
            &[(H, &[10])],
            "after batching G's last two deletes",
        );
    }
}

/// **Known latent defect**, same as
/// [`deleting_a_groups_last_two_documents_in_one_batch_removes_the_group`],
/// with the two deletes in the opposite order — the group's emptiness is
/// mis-observed either way, so the defect is not an artefact of one ordering.
///
/// Unreachable while `max_transitions_in_documents_batch` is 1.
#[test]
#[ignore = "documents a latent defect in shared write-path machinery that is unreachable while max_transitions_in_documents_batch is 1; un-ignore before raising that cap"]
fn reverse_ordered_batched_deletes_remove_the_group_too() {
    for axis in AXES {
        let (drive, contract, docs) = setup_g_two_h_one(axis);
        assert_baseline(&drive, &contract, axis);

        apply_batch(
            &drive,
            vec![
                delete_op(&contract, axis.doctype, docs[2].id()),
                delete_op(&contract, axis.doctype, docs[1].id()),
            ],
        );

        assert_ranking_is(
            &drive,
            &contract,
            axis,
            &[(H, &[10])],
            "after batching G's last two deletes in reverse order",
        );
    }
}

/// **Known latent defect**, same as
/// [`deleting_a_groups_last_two_documents_in_one_batch_removes_the_group`], on
/// a Drive configured the way a shipped node is — so it cannot be dismissed as
/// an artefact of the test harness's `batching_consistency_verification: true`.
///
/// Unreachable while `max_transitions_in_documents_batch` is 1.
#[test]
#[ignore = "documents a latent defect in shared write-path machinery that is unreachable while max_transitions_in_documents_batch is 1; un-ignore before raising that cap"]
fn batched_deletes_remove_the_group_under_the_shipped_batching_config() {
    for axis in AXES {
        let (drive, contract) = setup_restaurants_with_shipped_batching_config();
        let docs = insert_seeded(
            &drive,
            &contract,
            axis,
            &[(H, 10, 1), (G, 20, 2), (G, 30, 3)],
        );

        apply_batch(
            &drive,
            vec![
                delete_op(&contract, axis.doctype, docs[1].id()),
                delete_op(&contract, axis.doctype, docs[2].id()),
            ],
        );

        assert_ranking_is(
            &drive,
            &contract,
            axis,
            &[(H, &[10])],
            "after batching G's last two deletes on a shipped-config Drive",
        );
    }
}

/// **Known latent defect.** The batched drain and the identical drain applied
/// one delete at a time must land the same logical index; they do not.
///
/// The comparison is over entries and primary aggregates rather than the root
/// hash: a secondary Merk's shape depends on the order and grouping of the
/// writes that built it, so one batch and two batches reaching the same
/// logical state can legitimately hash differently. That is still
/// deterministic for nodes replaying identical history, so it is not itself a
/// consensus hazard — but it makes the app hash the wrong instrument here.
///
/// Unreachable while `max_transitions_in_documents_batch` is 1.
#[test]
#[ignore = "documents a latent defect in shared write-path machinery that is unreachable while max_transitions_in_documents_batch is 1; un-ignore before raising that cap"]
fn batched_and_sequential_drains_agree_on_the_logical_state() {
    for axis in AXES {
        let (batched, batched_contract, batched_docs) = setup_g_two_h_one(axis);
        let (sequential, sequential_contract, sequential_docs) = setup_g_two_h_one(axis);
        assert_eq!(
            logical_state(&batched, &batched_contract, axis),
            logical_state(&sequential, &sequential_contract, axis),
            "{:?}: the two fixtures must start from the same logical state",
            axis.ranked
        );

        apply_batch(
            &batched,
            vec![
                delete_op(&batched_contract, axis.doctype, batched_docs[1].id()),
                delete_op(&batched_contract, axis.doctype, batched_docs[2].id()),
            ],
        );
        for doc in &sequential_docs[1..3] {
            apply_batch(
                &sequential,
                vec![delete_op(&sequential_contract, axis.doctype, doc.id())],
            );
        }

        assert_eq!(
            logical_state(&batched, &batched_contract, axis),
            logical_state(&sequential, &sequential_contract, axis),
            "{:?}: draining G in one batch must land the same logical state as draining \
             it one delete at a time",
            axis.ranked
        );
    }
}

// ---------------------------------------------------------------------------
// Multi-move drain in one batch — the same defect through UpdateDocument
// ---------------------------------------------------------------------------

/// Move `documents` into group `H` by rewriting their index property, and
/// return them so they can be handed to update operations.
fn moved_to_h(documents: &[Document]) -> Vec<Document> {
    documents
        .iter()
        .map(|doc| {
            let mut moved = doc.clone();
            let mut props = moved.properties().clone();
            props.insert(GROUP_PROPERTY.to_string(), Value::Text(H.to_string()));
            moved.set_properties(props);
            moved.set_revision(Some(2));
            moved
        })
        .collect()
}

fn update_document_singly(drive: &Drive, contract: &DataContract, axis: Axis, doc: &Document) {
    drive
        .update_document_for_contract(
            doc,
            contract,
            contract
                .document_type_for_name(axis.doctype)
                .expect("doctype exists"),
            Some(doc.owner_id().to_buffer()),
            BlockInfo::default(),
            true,
            StorageFlags::optional_default_as_cow(),
            None,
            platform_version(),
            None,
        )
        .expect("the single-document update must succeed");
}

/// **Known latent defect**, reached through `UpdateDocument` rather than
/// `DeleteDocument`: one batch moves both of `G`'s documents into `H`. `H`
/// correctly gains all three contributions, but `G` survives as a zero-valued
/// phantom, so the defect is not specific to deletes — any operation that
/// empties a group while a sibling operation in the same batch has not been
/// observed reproduces it.
///
/// Unreachable while `max_transitions_in_documents_batch` is 1.
#[test]
#[ignore = "documents a latent defect in shared write-path machinery that is unreachable while max_transitions_in_documents_batch is 1; un-ignore before raising that cap"]
fn moving_a_groups_last_two_documents_to_another_group_in_one_batch_drains_it() {
    for axis in AXES {
        let (drive, contract, docs) = setup_g_two_h_one(axis);
        assert_baseline(&drive, &contract, axis);

        // A second drive that will receive the identical moves one at a time.
        let (sequential_drive, sequential_contract, sequential_docs) = setup_g_two_h_one(axis);
        assert_eq!(
            logical_state(&drive, &contract, axis),
            logical_state(&sequential_drive, &sequential_contract, axis),
            "{:?}: the two fixtures must start from the same logical state, otherwise \
             the comparison below proves nothing",
            axis.ranked
        );

        let moved = moved_to_h(&docs[1..3]);
        apply_batch(
            &drive,
            moved
                .iter()
                .map(|doc| update_op(&contract, axis.doctype, doc))
                .collect(),
        );

        assert_ranking_is(
            &drive,
            &contract,
            axis,
            &[(H, &[10, 20, 30])],
            "after batching both of G's documents into H",
        );

        for doc in moved_to_h(&sequential_docs[1..3]) {
            update_document_singly(&sequential_drive, &sequential_contract, axis, &doc);
        }
        assert_ranking_is(
            &sequential_drive,
            &sequential_contract,
            axis,
            &[(H, &[10, 20, 30])],
            "after moving both of G's documents into H sequentially",
        );

        assert_eq!(
            logical_state(&drive, &contract, axis),
            logical_state(&sequential_drive, &sequential_contract, axis),
            "{:?}: batching the two moves must land the same logical state as applying \
             them one at a time",
            axis.ranked
        );
    }
}

// ---------------------------------------------------------------------------
// Emptying and refilling a group in one batch — the inverse shape
// ---------------------------------------------------------------------------

/// Everything the delete-and-refill cases need to tell a correct outcome from
/// a silently wrong one, observed *after* the batch either applied or was
/// refused.
#[derive(Debug, PartialEq)]
struct Observation {
    /// `G`'s entry in the descending ranking, if it has one.
    g_in_ranking: Option<RankedEntryValue>,
    /// `G`'s primary value tree aggregates, if the tree survives.
    g_primary_present: bool,
    /// Is the arriving document in primary document storage?
    arrival_stored: bool,
    /// Is the departing document still in primary document storage?
    departure_stored: bool,
    /// grovedb's own integrity sweep.
    grovedb_issues: usize,
}

/// Primary document storage for a doctype: the `0` child of the doctype tree,
/// alongside the per-index property-name trees.
fn document_storage_path(contract: &DataContract, document_type_name: &str) -> Vec<Vec<u8>> {
    vec![
        vec![crate::drive::RootTree::DataContractDocuments as u8],
        contract.id().as_bytes().to_vec(),
        vec![1],
        document_type_name.as_bytes().to_vec(),
        vec![0],
    ]
}

fn document_is_stored(drive: &Drive, contract: &DataContract, axis: Axis, id: Identifier) -> bool {
    read_grove_element(
        drive,
        &document_storage_path(contract, axis.doctype),
        id.as_bytes(),
    )
    .is_some()
}

/// Read back everything the refill cases assert on.
fn observe(
    drive: &Drive,
    contract: &DataContract,
    axis: Axis,
    arrival: Identifier,
    departure: Identifier,
) -> Observation {
    let path = indexed_property_name_tree_path(contract, axis.doctype);
    let ranking = entries_of(run(drive, contract, axis, false, false));
    Observation {
        g_in_ranking: ranking
            .iter()
            .find(|entry| entry.key == G.as_bytes())
            .map(|entry| entry.value),
        g_primary_present: primary_group_aggregate(drive, &path, G).is_some(),
        arrival_stored: document_is_stored(drive, contract, axis, arrival),
        departure_stored: document_is_stored(drive, contract, axis, departure),
        grovedb_issues: drive
            .grove
            .verify_grovedb(None, true, false, &platform_version().drive.grove_version)
            .expect("verify_grovedb must run")
            .len(),
    }
}

/// Apply `operations` without panicking on a refusal — a refusal is a result
/// here, not a harness failure — and observe the state either way.
fn apply_and_observe(
    drive: &Drive,
    contract: &DataContract,
    axis: Axis,
    operations: Vec<DriveOperation>,
    arrival: Identifier,
    departure: Identifier,
) -> (Result<(), Error>, Observation) {
    let applied = drive
        .apply_drive_operations(
            operations,
            true,
            &BlockInfo::default(),
            None,
            platform_version(),
            None,
        )
        .map(|_| ());
    let observation = observe(drive, contract, axis, arrival, departure);
    (applied, observation)
}

/// The state a refused batch must leave behind: exactly the state before it,
/// with `G` still holding only its original document and the arrival nowhere.
///
/// `departure_stored` differs between the two cases — the delete case removes
/// the departing document if it applies, the update case keeps it — but on a
/// *refusal* both must still have it.
fn untouched(axis: Axis) -> Observation {
    Observation {
        g_in_ranking: Some(axis.expected_value(&[20])),
        g_primary_present: true,
        arrival_stored: false,
        departure_stored: true,
        grovedb_issues: 0,
    }
}

/// Assert a refusal is the loud, shipped-node kind.
///
/// With `batching_consistency_verification` on, Drive's own pre-flight check
/// on the assembled batch catches the collision. With it off — the shipped
/// default — Drive skips that check *and* tells grovedb to skip its
/// operation-consistency check, so the batch reaches the applier and grovedb's
/// tree-building pass is what refuses it. Only the second is what a real node
/// relies on, so the two are asserted apart rather than lumped together as
/// "some error".
fn assert_refusal_is_loud(error: &Error, consistency_verification: bool, label: &str) {
    if consistency_verification {
        assert!(
            matches!(error, Error::Drive(DriveError::GroveDBInsertion(_))),
            "{label}: with Drive's pre-flight batch check on, the refusal must come from \
             that check; got {error:?}"
        );
    } else {
        assert!(
            matches!(
                error,
                Error::GroveDB(inner) if matches!(**inner, grovedb::Error::InvalidBatchOperation(_))
            ),
            "{label}: on the shipped batching configuration the refusal must come from \
             grovedb's batch applier — that is the only guard a real node has here; \
             got {error:?}"
        );
    }
}

/// The fixture for both refill cases, on whichever batching configuration is
/// under test.
fn setup_g_one_h_one(
    axis: Axis,
    consistency_verification: bool,
) -> (Drive, DataContract, Document) {
    let (drive, contract) = if consistency_verification {
        setup_restaurants()
    } else {
        setup_restaurants_with_shipped_batching_config()
    };
    let mut docs = insert_seeded(&drive, &contract, axis, &[(H, 10, 1), (G, 20, 2)]);
    let g_document = docs.remove(1);
    (drive, contract, g_document)
}

/// `G` holds exactly one document; one batch removes it and puts a *different*
/// document in the same group.
///
/// This is the inverse of the phantom: the mutation that empties `G` schedules
/// its group tree for deletion while the mutation that refills it sees the
/// group still present pre-batch and emits no tree insert, so the arriving
/// document could end up in primary storage with no index entry pointing at
/// it. `verify_grovedb` cannot see that, which is why the assertions check
/// reachability through the ranked index *and* presence in primary document
/// storage.
///
/// The only two acceptable outcomes are a loud refusal and a fully correct
/// apply. Both operation orders and both batching configurations are swept,
/// because Drive's own pre-flight consistency check and grovedb's applier
/// reject a malformed batch at different places and only the second is what a
/// shipped node relies on.
#[test]
fn deleting_a_groups_only_document_and_creating_another_in_one_batch_never_lands_silently_wrong() {
    let mut report = String::new();
    let mut bad = Vec::new();

    for axis in AXES {
        for delete_first in [true, false] {
            for consistency_verification in [true, false] {
                let (drive, contract, departing) =
                    setup_g_one_h_one(axis, consistency_verification);
                let arrival = build_doc(&contract, axis.doctype, axis.property, G, 40, 99);
                assert_ne!(
                    arrival.id(),
                    departing.id(),
                    "the replacement must be a genuinely different document"
                );

                let delete = delete_op(&contract, axis.doctype, departing.id());
                let add = add_op(&contract, axis.doctype, &arrival);
                let operations = if delete_first {
                    vec![delete, add]
                } else {
                    vec![add, delete]
                };

                let label = format!(
                    "{:?} axis / {} / consistency_verification={consistency_verification}",
                    axis.ranked,
                    if delete_first {
                        "delete-then-create"
                    } else {
                        "create-then-delete"
                    },
                );

                let (applied, observed) = apply_and_observe(
                    &drive,
                    &contract,
                    axis,
                    operations,
                    arrival.id(),
                    departing.id(),
                );
                report.push_str(&format!("  {label}\n    → {applied:?} {observed:?}\n"));

                match &applied {
                    Err(error) => {
                        assert_refusal_is_loud(error, consistency_verification, &label);
                        assert_eq!(
                            observed,
                            untouched(axis),
                            "{label}: a refused batch must leave the state exactly as it was"
                        );
                    }
                    Ok(()) => {
                        // G keeps exactly the arriving document, and the
                        // deleted one is gone from primary storage.
                        let correct = Observation {
                            g_in_ranking: Some(axis.expected_value(&[40])),
                            g_primary_present: true,
                            arrival_stored: true,
                            departure_stored: false,
                            grovedb_issues: 0,
                        };
                        if observed != correct {
                            bad.push(label);
                        }
                    }
                }
            }
        }
    }

    assert!(
        bad.is_empty(),
        "these combinations applied but landed a wrong index: {bad:?}\n{report}"
    );
}

/// The mirror image: one batch moves `dA` *out* of `G` with an update while
/// creating `dB` *into* `G`. Same blind spot — the update's delete walker can
/// schedule `G`'s tree for removal while the create sees `G` still present and
/// emits nothing — but reached through the update path rather than the delete
/// path, and with `G`'s membership never actually dropping to zero.
///
/// Same acceptance rule as
/// [`deleting_a_groups_only_document_and_creating_another_in_one_batch_never_lands_silently_wrong`],
/// except that the departing document is updated rather than deleted, so it
/// must still be in primary storage afterwards.
#[test]
fn moving_one_document_out_of_a_group_while_creating_another_into_it_never_lands_silently_wrong() {
    let mut report = String::new();
    let mut bad = Vec::new();

    for axis in AXES {
        for update_first in [true, false] {
            for consistency_verification in [true, false] {
                let (drive, contract, departing) =
                    setup_g_one_h_one(axis, consistency_verification);

                let moved = moved_to_h(&[departing]).remove(0);
                let arrival = build_doc(&contract, axis.doctype, axis.property, G, 40, 99);

                let update = update_op(&contract, axis.doctype, &moved);
                let add = add_op(&contract, axis.doctype, &arrival);
                let operations = if update_first {
                    vec![update, add]
                } else {
                    vec![add, update]
                };

                let label = format!(
                    "{:?} axis / {} / consistency_verification={consistency_verification}",
                    axis.ranked,
                    if update_first {
                        "update-out-then-create-in"
                    } else {
                        "create-in-then-update-out"
                    },
                );

                let (applied, observed) = apply_and_observe(
                    &drive,
                    &contract,
                    axis,
                    operations,
                    arrival.id(),
                    moved.id(),
                );
                report.push_str(&format!("  {label}\n    → {applied:?} {observed:?}\n"));

                match &applied {
                    Err(error) => {
                        assert_refusal_is_loud(error, consistency_verification, &label);
                        assert_eq!(
                            observed,
                            untouched(axis),
                            "{label}: a refused batch must leave the state exactly as it was"
                        );
                    }
                    Ok(()) => {
                        // G keeps exactly the arriving document; the moved one
                        // stays in primary storage (it was updated, not deleted).
                        let correct = Observation {
                            g_in_ranking: Some(axis.expected_value(&[40])),
                            g_primary_present: true,
                            arrival_stored: true,
                            departure_stored: true,
                            grovedb_issues: 0,
                        };
                        if observed != correct {
                            bad.push(label);
                        }
                    }
                }
            }
        }
    }

    assert!(
        bad.is_empty(),
        "these combinations applied but landed a wrong index: {bad:?}\n{report}"
    );
}

// ---------------------------------------------------------------------------
// The control: the same mutations, separate batches
// ---------------------------------------------------------------------------

/// The control for the move path: the same two moves applied one at a time
/// drain `G` cleanly. Without it, a failure of the ignored batched-move case
/// would be ambiguous between "batching is the problem" and "moves are the
/// problem".
#[test]
fn moving_the_same_two_documents_to_another_group_one_at_a_time_drains_it() {
    for axis in AXES {
        let (drive, contract, docs) = setup_g_two_h_one(axis);
        assert_baseline(&drive, &contract, axis);

        for doc in moved_to_h(&docs[1..3]) {
            update_document_singly(&drive, &contract, axis, &doc);
        }

        assert_ranking_is(
            &drive,
            &contract,
            axis,
            &[(H, &[10, 20, 30])],
            "after moving both of G's documents into H one at a time",
        );
    }
}

/// The control that makes the ignored cases above mean something: the same two
/// deletes in two separate `apply_drive_operations` calls drain `G` cleanly
/// from both the primary and the secondary. If this ever fails, the defect is
/// not batch-specific and is much larger than the one documented here.
#[test]
fn deleting_the_same_two_documents_in_separate_batches_removes_the_group() {
    for axis in AXES {
        let (drive, contract, docs) = setup_g_two_h_one(axis);
        assert_baseline(&drive, &contract, axis);

        apply_batch(
            &drive,
            vec![delete_op(&contract, axis.doctype, docs[1].id())],
        );
        assert_ranking_is(
            &drive,
            &contract,
            axis,
            &[(G, &[30]), (H, &[10])],
            "after the first of two separate deletes",
        );

        apply_batch(
            &drive,
            vec![delete_op(&contract, axis.doctype, docs[2].id())],
        );
        assert_ranking_is(
            &drive,
            &contract,
            axis,
            &[(H, &[10])],
            "after the second of two separate deletes",
        );
    }
}
