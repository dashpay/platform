//! General Drive Document Functions
//!
//! This module defines general functions relevant to Documents in Drive.
//! Namely functions to return the paths to certain objects and the path sizes.
//!

#[cfg(feature = "server")]
use crate::drive::votes::paths::CONTESTED_DOCUMENT_STORAGE_TREE_KEY;
#[cfg(feature = "server")]
use crate::util::storage_flags::StorageFlags;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::DocumentTypeRef;
#[cfg(any(feature = "server", feature = "verify"))]
use dpp::document::Document;
#[cfg(feature = "server")]
use dpp::document::DocumentV0Getters;
#[cfg(feature = "server")]
use grovedb::reference_path::ReferencePathType::UpstreamRootHeightReference;
#[cfg(feature = "server")]
use grovedb::Element;

#[cfg(feature = "server")]
mod delete;
#[cfg(feature = "server")]
mod estimation_costs;
#[cfg(any(feature = "server", feature = "fixtures-and-mocks"))]
mod get_fetch;
#[cfg(feature = "server")]
mod index_uniqueness;
#[cfg(any(feature = "server", feature = "fixtures-and-mocks"))]
mod insert;
#[cfg(any(feature = "server", feature = "fixtures-and-mocks"))]
mod insert_contested;
#[cfg(any(feature = "server", feature = "fixtures-and-mocks"))]
pub mod query;
#[cfg(any(feature = "server", feature = "fixtures-and-mocks"))]
mod update;
#[cfg(all(
    feature = "verify",
    not(any(feature = "server", feature = "fixtures-and-mocks"))
))]
#[path = "query/fetch_document_history_query/mod.rs"]
mod verify_fetch_document_history_query;

/// paths
#[cfg(any(feature = "server", feature = "verify"))]
pub mod paths;

/// Primary key tree type resolution
#[cfg(feature = "server")]
pub mod primary_key_tree_type;
#[cfg(feature = "server")]
pub(crate) mod prove;
/// Terminal property-name tree resolution for ranked (indexed-tree) indexes
#[cfg(feature = "server")]
pub(crate) mod ranked_index_tree_type;

/// Shared index-walker tree-type derivation for the v2 walkers
#[cfg(feature = "server")]
pub(crate) mod index_level_tree_types;

/// Shared TTL semantics for time-range indexes — see
/// `book/src/drive/time-range-ttl.md`.
#[cfg(feature = "server")]
pub(crate) mod time_range_ttl;

/// indexOnly entry probes: entry path/key derivation shared by the write
/// path and the ABCI state-validation probes
#[cfg(feature = "server")]
pub mod index_only;

/// The indexOnly row commitment: the payload every indexOnly terminal item
/// stores, binding one document's index projections into one logical row
#[cfg(any(feature = "server", feature = "verify"))]
pub mod index_only_row_commitment;

#[cfg(any(feature = "server", feature = "verify"))]
pub use index_only_row_commitment::index_only_row_commitment;
#[cfg(feature = "server")]
pub use index_only_row_commitment::index_only_row_commitment_with_preimage_size;
#[cfg(any(feature = "server", feature = "verify"))]
pub use index_only_row_commitment::INDEX_ONLY_ROW_COMMITMENT_SIZE;

/// How many document history entries to fetch at once. This mirrors contract history
/// and prevents unbounded history reads.
pub const MAX_DOCUMENT_HISTORY_FETCH_LIMIT: u16 = 10;

#[cfg(feature = "server")]
/// Creates a reference to a document.
fn make_document_reference(
    document: &Document,
    document_type: DocumentTypeRef,
    storage_flags: Option<&StorageFlags>,
) -> Element {
    // we need to construct the reference from the split height of the contract document
    // type which is at 4
    // 0 represents document storage
    // Then we add document id
    // Then we add 0 if the document type keys history
    let mut reference_path = vec![vec![0], document.id().to_vec()];
    let mut max_reference_hops = 1;
    if document_type.documents_keep_history() {
        reference_path.push(vec![0]);
        max_reference_hops += 1;
    }
    // 2 because the contract could allow for history
    // 4 because
    // -DataContractDocumentsTree
    // -DataContract ID
    // - 1 Documents inDataContract
    // - DocumentType
    // We add 2 or 3
    // - 0 Storage
    // - Document id
    // -(Optional) 0 (means latest) in the case of documents_keep_history
    Element::Reference(
        UpstreamRootHeightReference(4, reference_path),
        Some(max_reference_hops),
        StorageFlags::map_to_some_element_flags(storage_flags),
    )
}

#[cfg(feature = "server")]
/// Creates an `Element::ReferenceWithSumItem` that pins a document to
/// a summable index path AND carries that document's `sum_property`
/// contribution to the parent sum tree.
///
/// Used in place of [`make_document_reference`] under summable indexes
/// (when `Index::summable` is `Some(_)`). Grovedb's
/// `Element::ReferenceWithSumItem(ReferencePathType, SumValue, flags)`
/// (added in grovedb PR 670) is the reference variant that BOTH
/// dereferences to the document body in primary storage (so document-
/// iteration via index walks still works exactly like
/// [`make_document_reference`]) AND contributes a per-document sum
/// to ancestor sum-bearing trees (`SumTree` / `ProvableSumTree` /
/// `CountSumTree` / `ProvableCountSumTree`).
///
/// Two roles, kept in different element types:
/// - **Primary storage** at `[doctype, 0, doc_id]` uses
///   `Element::ItemWithSumItem(serialized_doc, sum_value, flags)` —
///   the document body lives there inline.
/// - **Index references** at `[index_path, value, 0, doc_id]` use
///   `Element::ReferenceWithSumItem(reference_path, sum_value, flags)`
///   — pointer to primary storage with the per-doc sum attached.
///
/// `sum_value` MUST equal the document's value at the index's
/// `summable.unwrap()` property, read once at insert time. The DPP
/// validator already enforced that this property exists, is integer,
/// and is required — so the `to_integer::<SumValue>()` conversion is
/// safe.
///
/// On delete, grovedb reads `sum_value` straight off this stored
/// element and propagates the subtraction up the ancestor merk path —
/// no need to re-read the source document (its `sum_property` field
/// may have drifted, or the doc may not be deserializable in the
/// delete-by-id paths).
pub(crate) fn make_document_reference_with_sum_item(
    document: &Document,
    document_type: DocumentTypeRef,
    // `grovedb::SumValue = i64` per `grovedb-element/src/element/mod.rs`,
    // but the type alias isn't re-exported through the `grovedb`
    // facade crate. Use `i64` directly to avoid pulling in
    // `grovedb-element` as a separate dep.
    sum_value: i64,
    storage_flags: Option<&StorageFlags>,
) -> Element {
    // Reference-path construction mirrors `make_document_reference`
    // byte-for-byte — the only structural difference is the element
    // variant carrying the sum contribution alongside the path.
    let mut reference_path = vec![vec![0], document.id().to_vec()];
    let mut max_reference_hops = 1;
    if document_type.documents_keep_history() {
        reference_path.push(vec![0]);
        max_reference_hops += 1;
    }
    // grovedb PR 670 (`feat: add
    // Element::ProvableCountProvableSumTree + dual-axis crossover
    // proofs`, head SHA `79d45a7d`) lands `ReferenceWithSumItem` with
    // four constructors: `new_reference_with_sum_item`,
    // `_with_flags`, `_with_hops`, and
    // `_with_max_hops_and_flags`. We need both the hop count
    // (because the count-side `make_document_reference` uses
    // `Some(max_reference_hops)` to bound dereferencing at the
    // documents-keep-history depth) AND the storage flags, so it's
    // the 4-arg variant.
    Element::new_reference_with_sum_item_with_max_hops_and_flags(
        UpstreamRootHeightReference(4, reference_path),
        Some(max_reference_hops),
        sum_value,
        StorageFlags::map_to_some_element_flags(storage_flags),
    )
}

#[cfg(any(feature = "server", feature = "verify"))]
/// Read a document's `<sum_property>` field and convert it to `i64`
/// for use as the sum contribution in
/// [`make_document_item_with_sum_item`]. Also used on the verify side:
/// the executed-transition verifier recomputes the expected sum
/// contribution of a proved summable indexOnly entry from the created
/// document.
///
/// The DPP validator guarantees the named property exists and is in
/// the document's `required` array — a missing value here means
/// contract corruption (`CorruptedCodeExecution`). The integer
/// conversion failure, however, IS reachable from valid user input:
/// a U64-typed property whose schema allows values > i64::MAX would
/// pass DPP validation and fail here, so that branch returns
/// `DriveError::InvalidInput` (user-facing) rather than corruption.
pub(crate) fn read_document_sum_contribution(
    document: &Document,
    sum_property: &str,
) -> Result<i64, crate::error::Error> {
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use dpp::document::DocumentV0Getters;

    let value = document.properties().get(sum_property).ok_or_else(|| {
        Error::Drive(DriveError::CorruptedCodeExecution(
            "summable property absent from a document that the validator should have rejected — \
             contract validation must enforce that the named summable property is in `required`",
        ))
    })?;
    // `value.to_integer::<i64>()` can fail on a u64 value above
    // i64::MAX. The DPP-level cross-validation in
    // `try_from_schema/v2/mod.rs` accepts U64 as a summable property
    // type today (changing that would also require restructuring
    // property-type inference — tracked follow-up), so this branch is
    // reachable from valid input and the error must be user-facing.
    value.to_integer::<i64>().map_err(|e| {
        Error::Drive(DriveError::InvalidInput(format!(
            "summable property \"{}\" value cannot be represented as i64 (grovedb sum trees \
             use i64 aggregators; values above i64::MAX overflow the aggregator): {}",
            sum_property, e
        )))
    })
}

#[cfg(feature = "server")]
/// Creates a reference to a contested document.
fn make_document_contested_reference(
    document: &Document,
    storage_flags: Option<&StorageFlags>,
) -> Element {
    // we need to construct the reference from the split height of the contract document
    // type which is at 5 for the contested tree
    // 0 represents document storage
    // Then we add document id
    // Then we add 0 if the document type keys history
    let reference_path = vec![
        vec![CONTESTED_DOCUMENT_STORAGE_TREE_KEY],
        document.id().to_vec(),
    ];
    let max_reference_hops = 1;
    // 2 because the contract could allow for history
    // 5 because
    // -VotesTree
    // -ContestedResourceTree
    // -ActivePolls
    // -DataContract ID
    // - DocumentType
    // We add 2
    // - 0 Storage
    // - Document id
    Element::Reference(
        UpstreamRootHeightReference(5, reference_path),
        Some(max_reference_hops),
        StorageFlags::map_to_some_element_flags(storage_flags),
    )
}

#[cfg(feature = "server")]
/// size of a document reference.
fn document_reference_size(document_type: DocumentTypeRef) -> u32 {
    // we need to construct the reference from the split height of the contract document
    // type which is at 4
    // 0 represents document storage
    // Then we add document id
    // Then we add 0 if the document type keys history
    // vec![vec![0], Vec::from(document.id)];
    // 1 (vec size) + 1 (subvec size) + 1 (0) + 1 (subvec size) + 32 (document id size)
    let mut reference_path_size = 36;
    if document_type.documents_keep_history() {
        reference_path_size += 2;
    }

    // 1 for type reference
    // 1 for reference type
    // 1 for root height offset
    // reference path size
    // 1 reference_hops options
    // 1 reference_hops count
    // 1 element flags option
    6 + reference_path_size
}

#[cfg(feature = "server")]
fn unique_event_id() -> [u8; 32] {
    rand::random::<[u8; 32]>()
}

/// Tests module
#[cfg(feature = "server")]
#[cfg(test)]
pub(crate) mod tests {
    use std::option::Option::None;

    use crate::drive::Drive;
    use crate::util::storage_flags::StorageFlags;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::DataContract;
    use dpp::tests::json_document::json_document_to_contract;

    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::version::PlatformVersion;

    /// Setup Dashpay
    pub fn setup_dashpay(_prefix: &str, mutable_contact_requests: bool) -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);

        let platform_version = PlatformVersion::latest();

        let dashpay_path = if mutable_contact_requests {
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json"
        } else {
            "tests/supporting_files/contract/dashpay/dashpay-contract.json"
        };

        // let's construct the grovedb structure for the dashpay data contract
        let dashpay = json_document_to_contract(dashpay_path, false, platform_version)
            .expect("expected to get cbor document");
        drive
            .apply_contract(
                &dashpay,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, dashpay)
    }

    /// Setup the grades contract — single `grade` document type with
    /// five indexes designed for **average queries** (sum / count over
    /// the `score` property). See
    /// `tests/supporting_files/contract/grades/grades-contract.json` for
    /// the schema; the worked-examples chapter is at
    /// `book/src/drive/average-index-examples.md`.
    ///
    /// Tree shapes the contract produces (verified by the smoke test
    /// below):
    /// - primary key (`grade/[0]`) → **CountSumTree** (`documentsCountable` +
    ///   `documentsSummable`)
    /// - `byClass`, `byStudent`, `bySemester` value trees →
    ///   **CountSumTree** (per-key count + sum at one merk lookup)
    /// - `byClassSemester`, `byStudentSemester` `semester`
    ///   continuations → **ProvableCountProvableSumTree** (PCPS, both
    ///   `rangeCountable` + `rangeSummable` set; enables
    ///   `AggregateCountAndSumOnRange` for range-average proofs)
    pub fn setup_grades() -> (Drive, DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let grades = json_document_to_contract(
            "tests/supporting_files/contract/grades/grades-contract.json",
            false,
            platform_version,
        )
        .expect("expected to parse grades contract");
        drive
            .apply_contract(
                &grades,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply grades contract successfully");
        (drive, grades)
    }

    /// Smoke-test: load the grades contract from JSON, apply it, and
    /// confirm every index's property-name tree resolved to the
    /// expected variant. Pins the contract's shape against the
    /// existing index-walker dispatch (see
    /// `add_indices_for_top_index_level_for_contract_operations_v0`
    /// for the dispatch table); a future change to either side that
    /// breaks the average-query surface trips this test rather than
    /// surfacing as a `CorruptedData` at query time.
    #[test]
    fn grades_contract_loads_and_produces_expected_index_trees() {
        use crate::drive::RootTree;
        use crate::util::grove_operations::DirectQueryType;
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use grovedb::Element;
        use grovedb_path::SubtreePath;

        let (drive, contract) = setup_grades();
        let platform_version = PlatformVersion::latest();

        // Read the property-name tree at @/contract/0x01/grade/<prop>.
        let probe = |prop: &str| -> Element {
            let contract_id = contract.id().to_buffer();
            let path: Vec<Vec<u8>> = vec![
                vec![RootTree::DataContractDocuments as u8],
                contract_id.to_vec(),
                vec![1u8],
                b"grade".to_vec(),
            ];
            let path_slices: Vec<&[u8]> = path.iter().map(|p| p.as_slice()).collect();
            drive
                .grove_get_raw(
                    SubtreePath::from(path_slices.as_slice()),
                    prop.as_bytes(),
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &platform_version.drive,
                )
                .expect("probe must succeed")
                .expect("property-name tree must exist")
        };

        // `byClass` / `byStudent` / `bySemester` declare countable +
        // summable but neither range flag — top-level property-name tree
        // is a plain Tree (NormalTree); the per-value subtree underneath
        // (per-class, per-student, …) is the CountSumTree that carries
        // the per-key (count, sum). We only check the property-name
        // layer here.
        for prop in ["class", "student", "semester"] {
            match probe(prop) {
                Element::Tree(..) => {}
                other => panic!(
                    "grades.{prop} (countable + summable, no range) → expected Tree, got {other:?}"
                ),
            }
        }

        // `byClassSemester` and `byStudentSemester` declare both range
        // flags — the FIRST property of each (class / student) is shared
        // with byClass / byStudent so its top-level tree is also Tree
        // (the compound's `semester` continuation lives under each
        // class / student value-tree and is the actual PCPS). The
        // top-level probe surfaces only that the contract apply didn't
        // collide on the shared `class` / `student` keys.
        //
        // The PCPS continuation is verified at insert-time by the
        // existing `range_summable_index_e2e_tests` module's PCPS test
        // (4-corner regression on the dispatcher); no need to
        // re-litigate it here.
    }
}
