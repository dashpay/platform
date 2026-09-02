use crate::drive::constants::CONTRACT_DOCUMENTS_PATH_HEIGHT;
use crate::drive::document::index_level_tree_types::{
    index_level_tree_types_with_continuation_demotion, IndexLevelTreeTypes,
};
use crate::drive::document::time_range_ttl::{
    live_time_range_entry_keys, request_expired_time_range_drains, TimeRangeEntryState,
};
use crate::drive::document::{
    make_document_reference, make_document_reference_with_sum_item, read_document_sum_contribution,
};

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{
    BatchDeleteUpTreeApplyType, BatchInsertApplyType, BatchInsertTreeApplyType, DirectQueryType,
    QueryType,
};
use crate::util::object_size_info::DocumentInfo::DocumentOwnedInfo;
use crate::util::object_size_info::DriveKeyInfo::{Key, KeyRef, KeySize};
use crate::util::object_size_info::PathKeyElementInfo::PathKeyRefElement;
use crate::util::object_size_info::{
    DocumentAndContractInfo, DocumentInfo, DocumentInfoV0Methods, DriveKeyInfo, PathKeyInfo,
};
use crate::util::storage_flags::StorageFlags;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;

use dpp::document::document_methods::DocumentMethodsV0;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};

use crate::drive::document::paths::{
    contract_document_type_path,
    contract_documents_keeping_history_primary_key_path_for_document_id,
    contract_documents_primary_key_path,
};
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::data_contract::document_type::{
    DocumentTypeRef, Index, IndexCountability, IndexLevel, TimeRangeTransform,
};
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

/// Whether an old-document indexed-field value read back from storage is
/// null/absent — the counterpart of `document_top_field.is_empty()` on the
/// new-document side (a null or missing indexed property reads back as an
/// empty key). `KeySize` never reaches the stateful update path (worst-case
/// cost estimation is redirected to the insert walker at the top of
/// `update_document_for_contract_operations_v1`), so it is treated as
/// non-null.
fn drive_key_info_is_empty(key_info: &DriveKeyInfo) -> bool {
    match key_info {
        Key(key) => key.is_empty(),
        KeyRef(key_ref) => key_ref.is_empty(),
        KeySize(_) => false,
    }
}

/// `[0]`-key reference-bucket `TreeType` dispatch for the
/// terminator level. Mirrors the dispatch in
/// `add_reference_for_index_level_for_contract_operations_v0` —
/// this table distinguishes `Countable` (→ `CountTree`) from
/// `CountableAllowingOffset` (→ `ProvableCountTree`), where the
/// value-tree dispatch above collapses them via `is_countable()`.
///
/// The `[0]` bucket is the leaf tree under a non-unique terminator
/// value, holding the per-doc references; it must carry the index's
/// count and sum aggregates so `count_value_or_default()` /
/// `sum_value_or_default()` walks at the value tree's parent
/// resolve to the right per-value totals.
fn reference_tree_type_for_index(
    countable: IndexCountability,
    summable: &Option<String>,
    range_summable: bool,
) -> TreeType {
    let count_provable = matches!(countable, IndexCountability::CountableAllowingOffset);
    let count_root_only = matches!(countable, IndexCountability::Countable) && !count_provable;
    let sum_provable = range_summable;
    let sum_root_only = summable.is_some() && !sum_provable;
    match (count_provable, count_root_only, sum_provable, sum_root_only) {
        (false, false, false, false) => TreeType::NormalTree,
        (false, true, false, false) => TreeType::CountTree,
        (true, _, false, false) => TreeType::ProvableCountTree,
        (false, false, false, true) => TreeType::SumTree,
        (false, false, true, _) => TreeType::ProvableSumTree,
        (false, true, false, true) => TreeType::CountSumTree,
        (true, _, false, true) => TreeType::ProvableCountSumTree,
        (true, _, true, _) => TreeType::ProvableCountProvableSumTree,
        (false, true, true, _) => TreeType::ProvableCountProvableSumTree,
    }
}

impl Drive {
    /// Gathers operations for updating a document.
    ///
    /// v1 (platform v14+) applies the shared-prefix aggregate fix to the
    /// branches a key-changing update materializes, keeping them
    /// bit-identical to what the v2 insert walkers would have written:
    ///
    /// - value-tree and property-name tree types derive through the shared
    ///   [`index_level_tree_types_with_continuation_demotion`] helper, so
    ///   provable count-bearing value trees with compound continuations
    ///   demote to `CountSumTree` exactly as on insert;
    /// - continuation property-name trees created under an aggregating
    ///   value tree go through
    ///   [`Drive::batch_insert_empty_tree_contributing_zero_to_aggregating_parent_if_not_exists`],
    ///   so they contribute zero to every axis the parent aggregates. v0
    ///   inserted them unwrapped, which would let a continuation's own
    ///   aggregates leak into the per-value count/sum.
    ///
    /// The same helper resolves the property-name tree through
    /// [`crate::drive::document::ranked_index_tree_type`], so a branch
    /// materialized here for a ranked (meta schema v3) index comes back
    /// as the matching *indexed* tree with its ranking axes, not as a
    /// plain tree whose secondaries nothing would maintain. A
    /// key-changing update can be the operation that re-materializes a
    /// property-name tree an earlier delete drained away, so it has to
    /// reproduce that layout exactly. `ranked_axes` is empty for every
    /// pre-v14 contract.
    ///
    /// v1 also fixes the terminator layout dispatch to agree with the
    /// insert and delete walkers on null-bearing entries. v0
    /// dispatched unique vs
    /// non-unique layout on `all_fields_null` (AND across indexed
    /// fields) — and its old-entry delete on `!index.unique` alone —
    /// while the insert walker's
    /// `add_reference_for_index_level_for_contract_operations_v0` uses
    /// `!is_unique || any_fields_null` plus an all-null skip for
    /// `nullSearchable: false` indexes. Under v0, updating a document
    /// holding a unique index with SOME null fields moved its entry
    /// into the unique layout (bare reference at key `[0]`) where no
    /// query or delete would find it, and deleted the old entry with
    /// the wrong key. v1 uses the insert walker's dispatch on both
    /// sides, computing the old entry's layout from the old document's
    /// nullness and the new entry's from the new document's.
    ///
    /// The `[0]` reference-bucket tree-type dispatch and everything
    /// else match v0.
    pub(in crate::drive::document::update) fn update_document_for_contract_operations_v1(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        block_info: &BlockInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let drive_version = &platform_version.drive;
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];
        if !document_and_contract_info.document_type.requires_revision()
        // if it requires revision then there are reasons for us to be able to update in drive
        {
            return Err(Error::Drive(DriveError::UpdatingReadOnlyImmutableDocument(
                "this document type is not mutable",
            )));
        }

        // If we are going for estimated costs do an add instead as it always worse than an update
        if document_and_contract_info
            .owned_document_info
            .document_info
            .is_document_size()
            || estimated_costs_only_with_layer_info.is_some()
        {
            return self.add_document_for_contract_operations(
                document_and_contract_info,
                true, // we say we should override as this skips an unnecessary check
                block_info,
                previous_batch_operations,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            );
        }

        let contract = document_and_contract_info.contract;
        let document_type = document_and_contract_info.document_type;
        let owner_id = document_and_contract_info.owned_document_info.owner_id;
        let Some((document, storage_flags)) = document_and_contract_info
            .owned_document_info
            .document_info
            .get_borrowed_document_and_storage_flags()
        else {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "must have document and storage flags",
            )));
        };
        // we need to construct the path for documents on the contract
        // the path is
        //  * Document andDataContract root tree
        //  *DataContract ID recovered from document
        //  * 0 to signify Documents and notDataContract
        let contract_document_type_path =
            contract_document_type_path(contract.id_ref().as_bytes(), document_type.name());

        let contract_documents_primary_key_path =
            contract_documents_primary_key_path(contract.id_ref().as_bytes(), document_type.name());

        // Per-document reference is built per-index below because
        // summable indexes need `Element::ReferenceWithSumItem` (sum
        // contribution propagates to ancestor sum trees) while plain
        // indexes use `Element::Reference`. The non-sum reference is
        // computed once here for reuse on all non-summable indexes;
        // summable indexes build their own variant inside the loop.
        let document_reference = make_document_reference(
            document,
            document_and_contract_info.document_type,
            storage_flags,
        );

        // next we need to get the old document from storage
        let old_document_element = if document_type.documents_keep_history() {
            let contract_documents_keeping_history_primary_key_path_for_document_id =
                contract_documents_keeping_history_primary_key_path_for_document_id(
                    contract.id_ref().as_bytes(),
                    document_type.name().as_str(),
                    document.id_ref().as_slice(),
                );
            // When keeping document history the 0 is a reference that points to the current value
            // O is just on one byte, so we have at most one hop of size 1 (1 byte)
            self.grove_get(
                (&contract_documents_keeping_history_primary_key_path_for_document_id).into(),
                &[0],
                QueryType::StatefulQuery,
                transaction,
                &mut batch_operations,
                drive_version,
            )?
        } else {
            self.grove_get_raw(
                (&contract_documents_primary_key_path).into(),
                document.id().as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut batch_operations,
                drive_version,
            )?
        };

        // we need to store the document for it's primary key
        // we should be overriding if the document_type does not have history enabled
        self.add_document_to_primary_storage(
            &document_and_contract_info,
            block_info,
            true,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;

        let old_document_info = if let Some(old_document_element) = old_document_element {
            // Accept BOTH plain `Item` (non-summable doctypes) AND
            // `ItemWithSumItem` (summable doctypes — primary storage on
            // doctypes with `documents_summable: Some(_)` is written as
            // ItemWithSumItem by `add_document_to_primary_storage`).
            // The sum_value is discarded here because the reload only
            // needs the document body + flags; the new write below
            // re-computes the sum from the freshly-supplied document.
            let (old_serialized_document, element_flags) = match old_document_element {
                Element::Item(bytes, flags) => (bytes, flags),
                Element::ItemWithSumItem(bytes, _sum_value, flags) => (bytes, flags),
                _ => {
                    return Err(Error::Drive(DriveError::CorruptedDocumentNotItem(
                        "old document is not an item or item-with-sum-item",
                    )))
                }
            };
            let document = Document::from_bytes(
                old_serialized_document.as_slice(),
                document_type,
                platform_version,
            )?;
            let storage_flags = StorageFlags::map_some_element_flags_ref(&element_flags)?;
            DocumentOwnedInfo((document, storage_flags.map(Cow::Owned)))
        } else {
            return Err(Error::Drive(DriveError::UpdatingDocumentThatDoesNotExist(
                "document being updated does not exist",
            )));
        };

        let mut batch_insertion_cache: HashSet<Vec<Vec<u8>>> = HashSet::new();
        // Pre-built tree of every index path in the doctype. Walking
        // this in parallel with each `index.properties` chain below
        // is how we pick the right aggregate `TreeType` at each
        // branch we materialize on a key-changing update — matching
        // exactly what the insert path's
        // `add_indices_for_{top_index_,index_}level_for_contract_operations_v1`
        // helpers would have chosen. Without this walk, an update
        // that moves into a previously-unseen branch under an
        // aggregate index would create the branch as `NormalTree`
        // beneath a `ProvableCount*` / `ProvableSum*` parent —
        // diverging from the insert path (consensus break).
        let index_structure = document_type.index_structure();
        // Operations under TTL'd (ephemeral) levels — one vec for the whole
        // walk, so the emptiness climb of one index sees the removals of
        // another index sharing its level; re-tagged ephemeral after the
        // loop.
        let mut ephemeral_batch_operations: Vec<LowLevelDriveOperation> = vec![];

        // TTL drainage rides every write into a TTL'd index — updates
        // included. One request per deduplicated level (several indexes may
        // share one grid level), run once the transition's batch has
        // applied — see `apply_batch_low_level_drive_operations` — so the
        // removals queued below always target trees that still stand. This
        // path is stateful-only (estimation redirected to the insert walker
        // above), and drainage is unbilled — see the ttl module's Billing
        // section.
        {
            let base_path: Vec<Vec<u8>> = contract_document_type_path
                .iter()
                .map(|&segment| Vec::from(segment))
                .collect();
            request_expired_time_range_drains(
                index_structure,
                &base_path,
                block_info.time_ms,
                platform_version,
                &mut batch_operations,
            );
        }

        // fourth we need to store a reference to the document for each index
        for index in document_type.indexes().values() {
            // at this point the contract path is to the contract documents
            // for each index the top index component will already have been added
            // when the contract itself was created
            let mut index_path: Vec<Vec<u8>> = contract_document_type_path
                .iter()
                .map(|&x| Vec::from(x))
                .collect();
            let top_index_property = index.properties.first().ok_or(Error::Drive(
                DriveError::CorruptedContractIndexes("invalid contract indices".to_string()),
            ))?;
            // A time-range index's top level is keyed by the property name
            // qualified with the grid (`TimeRangeTransform::storage_key`),
            // so different grids over one timestamp live in sibling
            // subtrees; a plain index keeps the bare name. The same key is
            // both the path segment and the `IndexLevel` lookup below.
            let top_level_key = match index.time_range.as_ref() {
                Some(transform) => transform.storage_key(&top_index_property.name),
                None => top_index_property.name.clone(),
            };
            index_path.push(Vec::from(top_level_key.as_bytes()));

            // Mirror the insert path's IndexLevel descent. We
            // start at the top-level property's `IndexLevel` node —
            // the same node `add_indices_for_top_index_level_..._v1`
            // would feed to its `value_tree_type` dispatch — and
            // descend one step per property in `index.properties`
            // below, so at every branch we materialize the matching
            // `IndexLevel` node is in hand.
            //
            // `index_structure` carries the upgrade across ALL
            // indexes that share this path (a level can host both
            // `byRecipient` and `byRecipientSentAt`; the aggregate
            // type at depth 1 must reflect whichever terminator is
            // there). Using `has_index_with_type()` on the descended
            // node ensures we pick that upgrade rather than the
            // currently-iterated index's own per-level flags.
            let mut current_index_level =
                index_structure
                    .sub_levels()
                    .get(&top_level_key)
                    .ok_or(Error::Drive(DriveError::CorruptedContractIndexes(format!(
                        "index structure missing top level '{}' for index '{}' — \
                         doctype's IndexLevel tree must contain every property of every \
                         registered index",
                        top_level_key, index.name
                    ))))?;

            // Per-index reference variant. Mirror of the insert path's
            // dispatch in
            // `add_reference_for_index_level_for_contract_operations` —
            // summable indexes must emit `Element::ReferenceWithSumItem`
            // so the per-document sum propagates into ancestor sum trees
            // on every update. Without this branch, an update would
            // overwrite an existing `ReferenceWithSumItem` with a plain
            // `Reference`, silently dropping the doc's contribution
            // from ancestor sum aggregates (the document body remains
            // queryable but SUM/AVG proofs would exclude it — a soundness
            // bug an attacker could trigger with any benign no-op update).
            //
            // TTL'd (ephemeral) levels ride their own op batch and carry no
            // storage flags — same routing as the insert and delete
            // walkers; see the ttl module's Billing section. The reference
            // is built with the level's flags: an ephemeral reference must
            // be flagless or its later removal turns sectioned (refundable).
            let index_is_ephemeral = current_index_level
                .time_range()
                .is_some_and(|transform| transform.ttl_seconds.is_some());
            let index_storage_flags = if index_is_ephemeral {
                None
            } else {
                storage_flags
            };
            let index_document_reference = if let Some(sum_property_name) = &index.summable {
                let sum_value = read_document_sum_contribution(document, sum_property_name)?;
                make_document_reference_with_sum_item(
                    document,
                    document_and_contract_info.document_type,
                    sum_value,
                    index_storage_flags,
                )
            } else if index_is_ephemeral {
                make_document_reference(document, document_and_contract_info.document_type, None)
            } else {
                document_reference.clone()
            };

            // Time-range indexes store one entry per overlapping range bucket,
            // so they need a set-diff update rather than the single old→new
            // value transition below. `current_index_level` is still the
            // top-level node and `index_path` is the base
            // (…/<document_type>/<grid-qualified source>) at this point. The
            // transform is read off the `IndexLevel` node — the same source
            // the insert and delete walkers branch on — so all three walkers
            // agree even on a document type constructed outside full
            // validation. (A level's key embeds its grid, so the node's
            // transform is exactly the grid every index sharing this level
            // declared.)
            if let Some(transform) = current_index_level.time_range() {
                let index_batch_operations: &mut Vec<LowLevelDriveOperation> = if index_is_ephemeral
                {
                    &mut ephemeral_batch_operations
                } else {
                    &mut batch_operations
                };
                self.update_time_range_index_for_contract_operations_v1(
                    index,
                    transform,
                    document,
                    &old_document_info,
                    owner_id,
                    document_type,
                    &index_path,
                    current_index_level,
                    &index_document_reference,
                    index_storage_flags,
                    &mut batch_insertion_cache,
                    previous_batch_operations,
                    index_batch_operations,
                    block_info.time_ms,
                    transaction,
                    platform_version,
                )?;
                continue;
            }

            // with the example of the dashpay contract's first index
            // the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId
            let document_top_field = document
                .get_raw_for_document_type(
                    &top_index_property.name,
                    document_type,
                    owner_id,
                    platform_version,
                )?
                .unwrap_or_default();

            let old_document_top_field = old_document_info
                .get_raw_for_document_type(
                    &top_index_property.name,
                    document_type,
                    None, // We want to use the old owner id
                    None,
                    platform_version,
                )?
                .unwrap_or_default();

            // if we are not applying that means we are trying to get worst case costs
            // which would entail a change on every index
            let mut change_occurred_on_index = match &old_document_top_field {
                DriveKeyInfo::Key(k) => &document_top_field != k,
                DriveKeyInfo::KeyRef(k) => document_top_field.as_slice() != *k,
                DriveKeyInfo::KeySize(_) => {
                    // we should assume true in this worst case cost scenario
                    true
                }
            };

            // Post-demotion tree types for the top property's level —
            // the exact types the v2 insert walkers would derive. The
            // value-tree type is tracked as `parent_value_tree_type`
            // regardless of whether this level changed: a deeper
            // materialization below still needs to know what it hangs
            // under.
            let top_level_tree_types =
                index_level_tree_types_with_continuation_demotion(current_index_level)?;
            let mut parent_value_tree_type = top_level_tree_types.value_tree_type;
            // A prefix-ranking chain level (`rankedCountable: { at }`)
            // inverts the zero-contribution choice below: its value trees
            // count exactly their single continuation, so the continuation
            // is inserted unwrapped and contributes its subtree count —
            // matching the v2 insert walker's dispatch.
            let mut parent_counts_continuations = current_index_level.ranked_count_grouping()
                || current_index_level.count_propagating();

            if change_occurred_on_index {
                // here we are inserting an empty tree that will have a subtree of all other index properties
                let mut qualified_path = index_path.clone();
                qualified_path.push(document_top_field.clone());

                if !batch_insertion_cache.contains(&qualified_path) {
                    // Top-level value tree: aggregate variant
                    // depending on whether any index terminates at
                    // the top property (e.g., a standalone
                    // `[recipient]` alongside `[recipient, sentAt]`)
                    // and what flags that terminator carries.
                    // Default for pure-prefix levels collapses to
                    // `NormalTree`, matching pre-v12 behavior.
                    let value_tree_type = top_level_tree_types.value_tree_type;
                    let inserted = self.batch_insert_empty_tree_if_not_exists(
                        PathKeyInfo::PathKeyRef::<0>((
                            index_path.clone(),
                            document_top_field.as_slice(),
                        )),
                        value_tree_type,
                        storage_flags,
                        BatchInsertTreeApplyType::StatefulBatchInsertTree,
                        transaction,
                        previous_batch_operations,
                        &mut batch_operations,
                        drive_version,
                    )?;
                    if inserted {
                        batch_insertion_cache.insert(qualified_path);
                    }
                }
            }

            // Null accumulators for the terminator dispatch, tracked
            // separately for the new and the old document: the entry
            // being written lives in the layout the NEW document's
            // nullness selects, while the entry being deleted lives in
            // the layout the insert walker chose from the OLD
            // document's nullness — and the two can differ within one
            // update (e.g. a null field acquiring a value). `any` (OR)
            // picks unique vs non-unique layout, `all` (AND) drives
            // the `nullSearchable: false` skip; both mirror
            // `add_reference_for_index_level_for_contract_operations_v0`
            // exactly. v0 of this walker AND-accumulated a single
            // `all_fields_null` and used it for both sides — so for a
            // unique index with SOME null fields it wrote/deleted the
            // unique layout while the insert walker had used the
            // non-unique one, stranding the entry where no query (and
            // no later delete) would look.
            let old_document_top_field_is_empty = drive_key_info_is_empty(&old_document_top_field);
            let mut any_fields_null = document_top_field.is_empty();
            let mut all_fields_null = document_top_field.is_empty();
            let mut old_any_fields_null = old_document_top_field_is_empty;
            let mut old_all_fields_null = old_document_top_field_is_empty;

            let mut old_index_path: Vec<DriveKeyInfo> = index_path
                .iter()
                .map(|path_item| DriveKeyInfo::Key(path_item.clone()))
                .collect();
            // we push the actual value of the index path
            index_path.push(document_top_field);
            // the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>

            old_index_path.push(old_document_top_field);

            for i in 1..index.properties.len() {
                let index_property = index.properties.get(i).ok_or(Error::Drive(
                    DriveError::CorruptedContractIndexes("invalid contract indices".to_string()),
                ))?;

                // Descend one step in the doctype's `IndexLevel`
                // tree, in lockstep with `index.properties`. Failure
                // here means the IndexLevel tree was built from a
                // different doctype than the one we're iterating —
                // a corruption signal, not a user-input error.
                current_index_level = current_index_level
                    .sub_levels()
                    .get(&index_property.name)
                    .ok_or(Error::Drive(DriveError::CorruptedContractIndexes(format!(
                        "index structure missing sub_level '{}' under index '{}' at depth {}",
                        index_property.name, index.name, i
                    ))))?;

                let sub_level_tree_types =
                    index_level_tree_types_with_continuation_demotion(current_index_level)?;

                let document_index_field = document
                    .get_raw_for_document_type(
                        &index_property.name,
                        document_type,
                        owner_id,
                        platform_version,
                    )?
                    .unwrap_or_default();

                let old_document_index_field = old_document_info
                    .get_raw_for_document_type(
                        &index_property.name,
                        document_type,
                        None, // We want to use the old owner_id
                        None,
                        platform_version,
                    )?
                    .unwrap_or_default();

                // if we are not applying that means we are trying to get worst case costs
                // which would entail a change on every index
                change_occurred_on_index |= match &old_document_index_field {
                    DriveKeyInfo::Key(k) => &document_index_field != k,
                    DriveKeyInfo::KeyRef(k) => document_index_field != *k,
                    DriveKeyInfo::KeySize(_) => {
                        // we should assume true in this worst case cost scenario
                        true
                    }
                };

                if change_occurred_on_index {
                    // here we are inserting an empty tree that will have a subtree of all other index properties

                    let mut qualified_path = index_path.clone();
                    qualified_path.push(index_property.name.as_bytes().to_vec());

                    if !batch_insertion_cache.contains(&qualified_path) {
                        // Inner property-name tree at depth i+1 — a
                        // continuation hanging inside the previous
                        // level's value tree. When that parent
                        // aggregates (count, sum, or both) the
                        // continuation must contribute zero on every
                        // aggregated axis, exactly as on the insert
                        // path; v0 inserted it unwrapped, letting the
                        // continuation's aggregates leak into the
                        // per-value count/sum. A ranked level resolves
                        // to an indexed tree here, which the
                        // non-aggregating branch creates with its axes
                        // and the aggregating branch rejects (an
                        // indexed tree cannot live inside an
                        // aggregating value tree — see
                        // `INDEXED_INNER_UNWRAPPABLE` in `fees::op`).
                        let property_name_tree_type = sub_level_tree_types.property_name_tree_type;
                        let ranked_axes = sub_level_tree_types.ranked_axes.as_slice();
                        // A count-exempt sibling branch (`count_exempt_branch`,
                        // stamped by the IndexLevel derivation) re-inverts the
                        // chain-level choice per child: even though the chain
                        // level counts its own continuation, the sibling's
                        // branch tree must be zero-wrapped so its entries
                        // never pollute the subtree totals — matching the v2
                        // insert walker's dispatch.
                        let inserted = if matches!(parent_value_tree_type, TreeType::NormalTree)
                            || (parent_counts_continuations
                                && !current_index_level.count_exempt_branch())
                        {
                            self.batch_insert_empty_index_tree_if_not_exists(
                                PathKeyInfo::PathKeyRef::<0>((
                                    index_path.clone(),
                                    index_property.name.as_bytes(),
                                )),
                                property_name_tree_type,
                                ranked_axes,
                                storage_flags,
                                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                                transaction,
                                previous_batch_operations,
                                &mut batch_operations,
                                drive_version,
                            )?
                        } else {
                            self.batch_insert_empty_tree_contributing_zero_to_aggregating_parent_if_not_exists(
                                PathKeyInfo::PathKeyRef::<0>((
                                    index_path.clone(),
                                    index_property.name.as_bytes(),
                                )),
                                parent_value_tree_type,
                                property_name_tree_type,
                                ranked_axes,
                                storage_flags,
                                BatchInsertTreeApplyType::StatefulBatchInsertTree,
                                transaction,
                                previous_batch_operations,
                                &mut batch_operations,
                                drive_version,
                            )?
                        };
                        if inserted {
                            batch_insertion_cache.insert(qualified_path);
                        }
                    }
                }

                index_path.push(Vec::from(index_property.name.as_bytes()));
                old_index_path.push(DriveKeyInfo::Key(Vec::from(index_property.name.as_bytes())));

                // Iteration 1. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId
                // Iteration 2. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference

                if change_occurred_on_index {
                    // here we are inserting an empty tree that will have a subtree of all other index properties

                    let mut qualified_path = index_path.clone();
                    qualified_path.push(document_index_field.clone());

                    if !batch_insertion_cache.contains(&qualified_path) {
                        // Inner value tree at depth i+2: same
                        // dispatch as the top-level value tree
                        // above — aggregate variant when any index
                        // terminates at this level (this index or
                        // another sharing the prefix), post-demotion.
                        let value_tree_type = sub_level_tree_types.value_tree_type;
                        let inserted = self.batch_insert_empty_tree_if_not_exists(
                            PathKeyInfo::PathKeyRef::<0>((
                                index_path.clone(),
                                document_index_field.as_slice(),
                            )),
                            value_tree_type,
                            storage_flags,
                            BatchInsertTreeApplyType::StatefulBatchInsertTree,
                            transaction,
                            previous_batch_operations,
                            &mut batch_operations,
                            drive_version,
                        )?;
                        if inserted {
                            batch_insertion_cache.insert(qualified_path);
                        }
                    }
                }

                any_fields_null |= document_index_field.is_empty();
                all_fields_null &= document_index_field.is_empty();
                let old_document_index_field_is_empty =
                    drive_key_info_is_empty(&old_document_index_field);
                old_any_fields_null |= old_document_index_field_is_empty;
                old_all_fields_null &= old_document_index_field_is_empty;

                // The next-deeper continuation (if any) hangs inside
                // this level's value tree.
                parent_value_tree_type = sub_level_tree_types.value_tree_type;
                parent_counts_continuations = current_index_level.ranked_count_grouping()
                    || current_index_level.count_propagating();

                // we push the actual value of the index path, both for the new and the old
                index_path.push(document_index_field);
                old_index_path.push(old_document_index_field);
                // Iteration 1. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/
                // Iteration 2. the index path is now something likeDataContracts/ContractID/Documents(1)/$ownerId/<ownerId>/toUserId/<ToUserId>/accountReference/<accountReference>
            }

            if change_occurred_on_index {
                // we first need to delete the old values
                // unique indexes will be stored under key "0"
                // non unique indices should have a tree at key "0" that has all elements based off of primary key
                //
                // The old entry lives in the layout the insert walker
                // chose from the OLD document's nullness (see the
                // accumulators above): no entry at all when every
                // indexed field was null on a `nullSearchable: false`
                // index, the doc-id-keyed `[0]` bucket when the index
                // is non-unique OR any old field was null, and the
                // bare reference at key `[0]` only for a unique index
                // with no old nulls. Same dispatch as
                // `remove_reference_for_index_level_for_contract_operations_v0`.
                if old_all_fields_null && !index.null_searchable {
                    // the insert walker wrote no entry for the old
                    // document — nothing to delete
                } else {
                    let mut key_info_path = KeyInfoPath::from_vec(
                        old_index_path
                            .into_iter()
                            .map(|key_info| match key_info {
                                Key(key) => KnownKey(key),
                                KeyRef(key_ref) => KnownKey(key_ref.to_vec()),
                                KeySize(key_info) => key_info,
                            })
                            .collect::<Vec<KeyInfo>>(),
                    );

                    if !index.unique || old_any_fields_null {
                        key_info_path.push(KnownKey(vec![0]));

                        // here we should return an error if the element already exists
                        self.batch_delete_up_tree_while_empty(
                            key_info_path,
                            document.id().as_slice(),
                            Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                            BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                            },
                            transaction,
                            previous_batch_operations,
                            &mut batch_operations,
                            drive_version,
                        )?;
                    } else {
                        // here we should return an error if the element already exists
                        self.batch_delete_up_tree_while_empty(
                            key_info_path,
                            &[0],
                            Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                            BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                                is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                            },
                            transaction,
                            previous_batch_operations,
                            &mut batch_operations,
                            drive_version,
                        )?;
                    }
                }

                // unique indexes will be stored under key "0"
                // non unique indices should have a tree at key "0" that has all elements based off of primary key
                if all_fields_null && !index.null_searchable {
                    // The insert walker writes no entry when every
                    // indexed field is null on a `nullSearchable: false`
                    // index — write nothing here either, or the update
                    // would create an entry that insert/delete walkers
                    // never expect to exist.
                } else if !index.unique || any_fields_null {
                    // here we are inserting an empty tree that will have a subtree of all other index properties
                    //
                    // Terminator `[0]` reference bucket: same
                    // dispatch as
                    // `add_reference_for_index_level_for_contract_operations_v0`
                    // — this is the leaf tree the insert path
                    // installs under the terminator value, and it
                    // must carry the index's count + sum aggregates
                    // so per-value `count_value_or_default()` /
                    // `sum_value_or_default()` walks at the parent
                    // value tree resolve to the right totals.
                    //
                    // Unlike the value/property-name dispatches
                    // above, this table distinguishes `Countable`
                    // (→ `CountTree`) from `CountableAllowingOffset`
                    // (→ `ProvableCountTree`), matching the insert
                    // path's terminator bucket exactly.
                    let reference_tree_type = reference_tree_type_for_index(
                        index.countable,
                        &index.summable,
                        index.range_summable,
                    );
                    self.batch_insert_empty_tree_if_not_exists(
                        PathKeyInfo::PathKeyRef::<0>((index_path.clone(), &[0])),
                        reference_tree_type,
                        storage_flags,
                        BatchInsertTreeApplyType::StatefulBatchInsertTree,
                        transaction,
                        previous_batch_operations,
                        &mut batch_operations,
                        drive_version,
                    )?;
                    index_path.push(vec![0]);

                    // here we should return an error if the element already exists
                    self.batch_insert(
                        PathKeyRefElement::<0>((
                            index_path,
                            document.id().as_slice(),
                            index_document_reference.clone(),
                        )),
                        &mut batch_operations,
                        drive_version,
                    )?;
                } else {
                    // in one update you can't insert an element twice, so need to check the cache
                    // here we should return an error if the element already exists
                    let inserted = self.batch_insert_if_not_exists(
                        PathKeyRefElement::<0>((
                            index_path,
                            &[0],
                            index_document_reference.clone(),
                        )),
                        BatchInsertApplyType::StatefulBatchInsert,
                        transaction,
                        &mut batch_operations,
                        drive_version,
                    )?;
                    if !inserted {
                        return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                            "index already exists".to_string(),
                        )));
                    }
                }
            } else {
                // no change occurred on index, we need to refresh the references

                // We can only trust the reference content has not changed if there are no storage flags
                let trust_refresh_reference = storage_flags.is_none();

                // unique indexes will be stored under key "0"
                // non unique indices should have a tree at key "0" that has all elements based off of primary key
                //
                // No change occurred on this index, so the old and new
                // nullness are identical and the new-document
                // accumulators describe the stored entry's layout.
                if all_fields_null && !index.null_searchable {
                    // nothing is stored for this document under this
                    // index — nothing to refresh
                } else if !index.unique || any_fields_null {
                    index_path.push(vec![0]);

                    // here we should return an error if the element already exists
                    self.batch_refresh_reference(
                        index_path,
                        document.id().to_vec(),
                        index_document_reference.clone(),
                        trust_refresh_reference,
                        &mut batch_operations,
                        drive_version,
                    )?;
                } else {
                    self.batch_refresh_reference(
                        index_path,
                        vec![0],
                        index_document_reference.clone(),
                        trust_refresh_reference,
                        &mut batch_operations,
                        drive_version,
                    )?;
                }
            }
        }
        LowLevelDriveOperation::push_retagged_ephemeral(
            &mut batch_operations,
            ephemeral_batch_operations,
        );
        Ok(batch_operations)
    }

    /// Updates the index entries for a single **time-range** index.
    ///
    /// A time-range index stores one entry per overlapping range bucket the
    /// document's timestamp falls into, so an update is a set diff rather than
    /// the single old→new transition the normal-index path performs:
    /// - entry keys present only in the old timestamp's set are removed,
    /// - entry keys present only in the new timestamp's set are inserted,
    /// - keys present in both are refreshed, or (when a later index property
    ///   changed) deleted under the old sub-values and reinserted under the new
    ///   ones.
    ///
    /// The entry-key set for a document mirrors the insert walker exactly: a
    /// decodable timestamp yields one key per containing bucket (none when the
    /// timestamp predates the transform's origin), a null timestamp yields the
    /// single ordinary null key, and a non-null value that fails to decode
    /// yields its raw key — so null→value, value→null and null→null
    /// transitions maintain the same entries insert and delete would.
    ///
    /// Time-range indexes are validated to be non-contested, but they may be
    /// unique when the windows do not overlap (`range == step`) and the source
    /// is `$createdAt`, so both terminator layouts occur here and are chosen
    /// exactly as the insert and delete walkers choose them
    /// (`!unique || any_fields_null`): the non-unique layout stores the
    /// reference under `…/<value>/[0]/<doc_id>`, the unique layout stores it
    /// AT `…/<value>/[0]`. The predicate is evaluated separately for the new
    /// and the old document, because the layout an entry was *written* under
    /// is the layout it must be *deleted* under.
    ///
    /// (With a `$createdAt` source and overlap factor 1 the bucket set can
    /// never change on an update — the timestamp is immutable and yields
    /// exactly one bucket — so on a unique index only the suffix-change arms
    /// below are reachable. The layout dispatch is nonetheless applied to
    /// every arm rather than reasoned away per arm: the arms are shared with
    /// non-unique indexes, and a future relaxation of the uniqueness rules
    /// must not silently resurrect the wrong layout.)
    ///
    /// The estimated-cost / document-size update path never reaches this
    /// method — it delegates to the insert path, which has its own bucket
    /// fan-out.
    #[allow(clippy::too_many_arguments)]
    fn update_time_range_index_for_contract_operations_v1(
        &self,
        index: &Index,
        transform: &TimeRangeTransform,
        document: &Document,
        old_document_info: &DocumentInfo,
        owner_id: Option<[u8; 32]>,
        document_type: DocumentTypeRef,
        base_index_path: &[Vec<u8>],
        top_index_level: &IndexLevel,
        index_document_reference: &Element,
        storage_flags: Option<&StorageFlags>,
        batch_insertion_cache: &mut HashSet<Vec<Vec<u8>>>,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        block_time_ms: u64,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;

        // TTL drainage is requested by the caller once per deduplicated
        // level, before this per-index loop, and runs after the batch
        // applies — so the removable checks below see standing state and no
        // drop can race a queued mutation. Do not request it here — several
        // indexes may share this level.

        // New/old raw values for the bucketed source property → entry key
        // sets, mirroring the insert walker's fan-out (see the doc comment).
        let new_raw = document.get_raw_for_document_type(
            &transform.source,
            document_type,
            owner_id,
            platform_version,
        )?;
        let old_raw = match old_document_info.get_raw_for_document_type(
            &transform.source,
            document_type,
            None, // We want to use the old owner id
            None,
            platform_version,
        )? {
            Some(Key(k)) => Some(k),
            Some(KeyRef(k)) => Some(k.to_vec()),
            _ => None,
        };

        // The entry-key rule (null → single null entry, pre-origin → no
        // entries, undecodable → raw key, decodable → containing buckets) is
        // shared with the insert and delete walkers via
        // `TimeRangeTransform::entry_keys_for_raw` — one definition, so the
        // three walkers can never disagree.
        // TTL: writes never target expired buckets — an update of a document
        // whose windows have all expired leaves it with no entries under
        // this index, and never resurrects a dropped bucket. The old set is
        // NOT filtered: its expired keys flow to the delete loop below,
        // whose per-key removable check skips exactly the buckets the lazy
        // drop already took.
        let new_entry_keys = live_time_range_entry_keys(
            transform,
            transform.entry_keys_for_raw(new_raw.as_deref().unwrap_or_default()),
            block_time_ms,
        );
        let old_entry_keys = transform.entry_keys_for_raw(old_raw.as_deref().unwrap_or_default());

        // Terminator-layout inputs, tracked separately for the new and the old
        // document and accumulated over the suffix properties in the loop
        // below. The insert and delete walkers demote a unique index to the
        // non-unique layout as soon as ANY indexed field is null
        // (`any_fields_null`, accumulated with OR down the level recursion), so
        // an entry's layout is a property of the document that wrote it — the
        // delete below must therefore use the OLD document's verdict, not the
        // new one's. A null source raw value yields the single empty entry key,
        // which is exactly the null case here.
        let mut new_any_fields_null = new_raw.as_deref().unwrap_or_default().is_empty();
        let mut old_any_fields_null = old_raw.as_deref().unwrap_or_default().is_empty();

        // Sub-property suffix (positions 1..) interleaved as [name, value, …]
        // for both the new and old document, plus the IndexLevel node at each
        // depth with its (pure, hoisted) tree-type derivation — the bucket
        // loop below runs up to `overlap_factor` times and must not re-derive
        // per iteration.
        let mut levels: Vec<(&IndexLevel, IndexLevelTreeTypes)> = Vec::new();
        let mut new_suffix: Vec<Vec<u8>> = Vec::new();
        let mut old_suffix: Vec<Vec<u8>> = Vec::new();
        let mut current_level = top_index_level;
        for index_property in index.properties.iter().skip(1) {
            current_level =
                current_level
                    .sub_levels()
                    .get(&index_property.name)
                    .ok_or(Error::Drive(DriveError::CorruptedContractIndexes(format!(
                        "index structure missing sub_level '{}' under time-range index '{}'",
                        index_property.name, index.name
                    ))))?;
            levels.push((
                current_level,
                index_level_tree_types_with_continuation_demotion(current_level)?,
            ));

            let new_val = document
                .get_raw_for_document_type(
                    &index_property.name,
                    document_type,
                    owner_id,
                    platform_version,
                )?
                .unwrap_or_default();
            let old_val = match old_document_info.get_raw_for_document_type(
                &index_property.name,
                document_type,
                None,
                None,
                platform_version,
            )? {
                Some(Key(k)) => k,
                Some(KeyRef(k)) => k.to_vec(),
                _ => Vec::new(),
            };
            new_any_fields_null |= new_val.is_empty();
            old_any_fields_null |= old_val.is_empty();

            new_suffix.push(index_property.name.as_bytes().to_vec());
            new_suffix.push(new_val);
            old_suffix.push(index_property.name.as_bytes().to_vec());
            old_suffix.push(old_val);
        }

        // The two terminator layouts, mirroring
        // `add_reference_for_index_level_for_contract_operations_v0`'s
        // `!index_type.index_type.is_unique() || any_fields_null` dispatch.
        let new_terminator_is_unique = index.unique && !new_any_fields_null;
        let old_terminator_is_unique = index.unique && !old_any_fields_null;

        let suffix_changed = new_suffix != old_suffix;
        let reference_tree_type =
            reference_tree_type_for_index(index.countable, &index.summable, index.range_summable);
        let top_level_tree_types =
            index_level_tree_types_with_continuation_demotion(top_index_level)?;
        let top_value_tree_type = top_level_tree_types.value_tree_type;
        let doc_id = document.id();

        let new_set: HashSet<&Vec<u8>> = new_entry_keys.iter().collect();
        let old_set: HashSet<&Vec<u8>> = old_entry_keys.iter().collect();

        // Insert/refresh new entries FIRST (see the ordering note on the
        // delete loop below): added keys are inserted; common keys are
        // refreshed when unchanged or reinserted when the suffix changed.
        for entry_key in &new_entry_keys {
            if old_set.contains(entry_key) && !suffix_changed {
                // Unchanged entry — refresh the stored reference in place
                // (its content can still differ via storage flags). An entry
                // key can only be in both sets when the source value's
                // null-ness matched (a null raw yields the empty key, a
                // non-null one an 8-byte bucket start), and the suffix is
                // unchanged here, so the old and new layouts coincide and
                // either verdict describes the entry on disk.
                let mut path: Vec<Vec<u8>> = base_index_path.to_vec();
                path.push(entry_key.clone());
                for segment in &new_suffix {
                    path.push(segment.clone());
                }
                let (refresh_key, refresh_path) = if new_terminator_is_unique {
                    // Unique layout: the reference lives AT `[0]`.
                    (vec![0], path)
                } else {
                    // Non-unique layout: `[0]` is a tree of doc-id-keyed
                    // references.
                    path.push(vec![0]);
                    (doc_id.to_vec(), path)
                };
                self.batch_refresh_reference(
                    refresh_path,
                    refresh_key,
                    index_document_reference.clone(),
                    storage_flags.is_none(),
                    batch_operations,
                    drive_version,
                )?;
                continue;
            }

            // Materialize every tree along the entry path — the same
            // post-demotion tree types and zero-contribution wrappers the
            // insert walkers use — then store the reference. The insertion
            // cache prevents duplicate empty-tree operations when several
            // indexes produce the same qualified path. Indexes sharing a
            // first property need NOT share a transform (grids fork into
            // sibling subtrees via their grid-qualified level keys), which
            // is exactly why the cache keys on the full qualified path
            // rather than the property name.
            let mut path: Vec<Vec<u8>> = base_index_path.to_vec();
            let mut qualified_path = path.clone();
            qualified_path.push(entry_key.clone());
            if !batch_insertion_cache.contains(&qualified_path) {
                let inserted = self.batch_insert_empty_tree_if_not_exists(
                    PathKeyInfo::PathKeyRef::<0>((path.clone(), entry_key.as_slice())),
                    top_value_tree_type,
                    storage_flags,
                    BatchInsertTreeApplyType::StatefulBatchInsertTree,
                    transaction,
                    previous_batch_operations,
                    batch_operations,
                    drive_version,
                )?;
                if inserted {
                    batch_insertion_cache.insert(qualified_path);
                }
            }
            path.push(entry_key.clone());

            let mut parent_value_tree_type = top_value_tree_type;
            // A prefix-ranking chain level (`rankedCountable: { at }`)
            // inverts the zero-contribution choice below, exactly as in the
            // non-bucketed branch above and the v2 insert walker: a chain
            // level's value trees count their single continuation, so the
            // continuation is inserted unwrapped and contributes its
            // subtree count. The bucketed level itself can never be a
            // grouping level (validation rejects `at` naming the transform
            // source), but the stamps are read rather than assumed so the
            // three walkers share one rule.
            let mut parent_counts_continuations =
                top_index_level.ranked_count_grouping() || top_index_level.count_propagating();
            for (i, (level, sub_level_tree_types)) in levels.iter().enumerate() {
                let property_name = &new_suffix[i * 2];
                let value = &new_suffix[i * 2 + 1];

                let mut qualified_path = path.clone();
                qualified_path.push(property_name.clone());
                if !batch_insertion_cache.contains(&qualified_path) {
                    // Continuation property-name tree: contributes zero on
                    // every axis its aggregating parent maintains, exactly
                    // as on the insert path — except inside a ranking
                    // chain, whose continuation stays unwrapped, unless
                    // this child is a count-exempt sibling branch
                    // (re-inversion per child; see the non-bucketed
                    // dispatch above).
                    let property_name_tree_type = sub_level_tree_types.property_name_tree_type;
                    let ranked_axes = sub_level_tree_types.ranked_axes.as_slice();
                    let inserted = if matches!(parent_value_tree_type, TreeType::NormalTree)
                        || (parent_counts_continuations && !level.count_exempt_branch())
                    {
                        self.batch_insert_empty_index_tree_if_not_exists(
                            PathKeyInfo::PathKeyRef::<0>((path.clone(), property_name.as_slice())),
                            property_name_tree_type,
                            ranked_axes,
                            storage_flags,
                            BatchInsertTreeApplyType::StatefulBatchInsertTree,
                            transaction,
                            previous_batch_operations,
                            batch_operations,
                            drive_version,
                        )?
                    } else {
                        self.batch_insert_empty_tree_contributing_zero_to_aggregating_parent_if_not_exists(
                            PathKeyInfo::PathKeyRef::<0>((path.clone(), property_name.as_slice())),
                            parent_value_tree_type,
                            property_name_tree_type,
                            ranked_axes,
                            storage_flags,
                            BatchInsertTreeApplyType::StatefulBatchInsertTree,
                            transaction,
                            previous_batch_operations,
                            batch_operations,
                            drive_version,
                        )?
                    };
                    if inserted {
                        batch_insertion_cache.insert(qualified_path);
                    }
                }
                path.push(property_name.clone());

                let mut qualified_path = path.clone();
                qualified_path.push(value.clone());
                if !batch_insertion_cache.contains(&qualified_path) {
                    let inserted = self.batch_insert_empty_tree_if_not_exists(
                        PathKeyInfo::PathKeyRef::<0>((path.clone(), value.as_slice())),
                        sub_level_tree_types.value_tree_type,
                        storage_flags,
                        BatchInsertTreeApplyType::StatefulBatchInsertTree,
                        transaction,
                        previous_batch_operations,
                        batch_operations,
                        drive_version,
                    )?;
                    if inserted {
                        batch_insertion_cache.insert(qualified_path);
                    }
                }
                path.push(value.clone());

                parent_value_tree_type = sub_level_tree_types.value_tree_type;
                parent_counts_continuations =
                    level.ranked_count_grouping() || level.count_propagating();
            }

            if new_terminator_is_unique {
                // Unique layout: no per-document subtree — the reference IS
                // the element at `[0]`, so the slot must be free. Insertion
                // failure means two documents claim the same (bucket, …)
                // tuple, which the uniqueness validation should have caught
                // before we got here; treat it as index corruption exactly as
                // the non-time-range branch does.
                let inserted = self.batch_insert_if_not_exists(
                    PathKeyRefElement::<0>((path, &[0], index_document_reference.clone())),
                    BatchInsertApplyType::StatefulBatchInsert,
                    transaction,
                    batch_operations,
                    drive_version,
                )?;
                if !inserted {
                    return Err(Error::Drive(DriveError::CorruptedContractIndexes(
                        "index already exists".to_string(),
                    )));
                }
            } else {
                // Non-unique layout: materialize the `[0]` reference bucket
                // (it carries the index's count/sum aggregates) and store the
                // reference under the document id.
                self.batch_insert_empty_tree_if_not_exists(
                    PathKeyInfo::PathKeyRef::<0>((path.clone(), &[0])),
                    reference_tree_type,
                    storage_flags,
                    BatchInsertTreeApplyType::StatefulBatchInsertTree,
                    transaction,
                    previous_batch_operations,
                    batch_operations,
                    drive_version,
                )?;
                path.push(vec![0]);

                self.batch_insert(
                    PathKeyRefElement::<0>((
                        path,
                        doc_id.as_slice(),
                        index_document_reference.clone(),
                    )),
                    batch_operations,
                    drive_version,
                )?;
            }
        }
        // Delete old entries LAST: removed keys, plus common keys whose
        // sub-values changed (their old-suffix path no longer matches). The
        // ordering is load-bearing: `batch_delete_up_tree_while_empty` folds
        // the already-accumulated batch operations into its emptiness walk,
        // so with the inserts above in place it stops before deleting a
        // shared ancestor tree (bucket / property-name) that this same batch
        // re-populates — e.g. on a suffix change at an unchanged timestamp.
        // Deletes first would emit a delete of that ancestor alongside
        // inserts into it, which grovedb's batch consistency check rejects.
        for entry_key in &old_entry_keys {
            if new_set.contains(entry_key) && !suffix_changed {
                continue; // unchanged entry — already refreshed by the insert loop above
            }
            // TTL: an expired bucket the drain already took entirely has no
            // entry left to delete; one that still stands may be PARTIALLY
            // drained (whole `[0]` and group value trees go before the
            // bucket), so the entry is checked at full-path granularity
            // below and skipped when its deeper trees are gone. A live
            // bucket behaves exactly as before. This path is stateful-only
            // (estimation redirects to the insert walker at the top of the
            // v1 update), so the existence reads are always legal here.
            match self.time_range_entry_state(
                transform,
                entry_key,
                block_time_ms,
                base_index_path,
                transaction,
                platform_version,
            )? {
                TimeRangeEntryState::Live => {}
                TimeRangeEntryState::ExpiredGone => continue,
                TimeRangeEntryState::ExpiredStanding => {
                    let mut entry_path_segments: Vec<Vec<u8>> = base_index_path.to_vec();
                    entry_path_segments.push(entry_key.clone());
                    for segment in &old_suffix {
                        entry_path_segments.push(segment.clone());
                    }
                    if !old_terminator_is_unique {
                        entry_path_segments.push(vec![0]);
                    }
                    // The bucket itself was just found standing, so the
                    // walk starts below it.
                    if !self.expired_entry_path_exists(
                        &entry_path_segments,
                        base_index_path.len() + 1,
                        transaction,
                        platform_version,
                    )? {
                        continue;
                    }
                }
            }
            let mut key_info_path: Vec<KeyInfo> = base_index_path
                .iter()
                .map(|s| KnownKey(s.clone()))
                .collect();
            key_info_path.push(KnownKey(entry_key.clone()));
            for segment in &old_suffix {
                key_info_path.push(KnownKey(segment.clone()));
            }
            // The entry is removed under the layout the OLD document wrote it
            // with: under the unique layout `[0]` IS the reference and is the
            // key being deleted, under the non-unique layout `[0]` is the
            // enclosing tree and the document id is the key. The stop height
            // is unaffected — the emptiness walk still climbs the suffix
            // values, the property-name trees and the bucket, and still stops
            // at the document-type level, whose per-index property-name trees
            // belong to contract registration rather than to any document.
            let delete_key: Vec<u8> = if old_terminator_is_unique {
                vec![0]
            } else {
                key_info_path.push(KnownKey(vec![0]));
                doc_id.to_vec()
            };
            self.batch_delete_up_tree_while_empty(
                KeyInfoPath::from_vec(key_info_path),
                delete_key.as_slice(),
                Some(CONTRACT_DOCUMENTS_PATH_HEIGHT),
                BatchDeleteUpTreeApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                },
                transaction,
                previous_batch_operations,
                batch_operations,
                drive_version,
            )?;
        }

        Ok(())
    }
}
