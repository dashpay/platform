use crate::drive::constants::CONTRACT_DOCUMENTS_PATH_HEIGHT;
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
    DocumentAndContractInfo, DocumentInfoV0Methods, DriveKeyInfo, PathKeyInfo,
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
use dpp::data_contract::document_type::{IndexCountability, IndexLevel};
use dpp::version::PlatformVersion;
use grovedb::batch::key_info::KeyInfo;
use grovedb::batch::key_info::KeyInfo::KnownKey;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, MaybeTree, TransactionArg, TreeType};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

/// Value-tree `TreeType` dispatch for a given `IndexLevel` node.
///
/// Mirrors the dispatch table inlined in
/// `add_indices_for_index_level_for_contract_operations_v1` (and
/// the matching arm in the top-level helper) so any branch this
/// update path materializes for a key-changing update lands with
/// the exact same TreeType the insert path would have chosen.
///
/// Each `IndexLevel` node represents a property name; its value
/// tree (children of the property-name tree, keyed by the
/// property's distinct values) takes the aggregate variant from
/// the level's `has_index_with_type()` flags — `None` (pure
/// prefix level, no index terminates here) collapses to
/// `NormalTree`.
fn value_tree_type_for_index_level(index_level: &IndexLevel) -> TreeType {
    let info = index_level.has_index_with_type();
    let is_countable_terminator = info.map(|i| i.countable.is_countable()).unwrap_or(false);
    let range_countable = info.map(|i| i.range_countable).unwrap_or(false);
    let is_summable_terminator = info.map(|i| i.summable.is_some()).unwrap_or(false);
    let range_summable = info.map(|i| i.range_summable).unwrap_or(false);
    match (
        is_countable_terminator,
        range_countable,
        is_summable_terminator,
        range_summable,
    ) {
        (true, true, true, true) => TreeType::ProvableCountProvableSumTree,
        (true, false, true, false) => TreeType::CountSumTree,
        (true, true, true, false) => TreeType::ProvableCountSumTree,
        (true, false, true, true) => TreeType::ProvableCountProvableSumTree,
        (true, _, false, false) => TreeType::CountTree,
        (false, false, true, _) => TreeType::SumTree,
        (false, _, false, _) => TreeType::NormalTree,
        _ => TreeType::NormalTree,
    }
}

/// Property-name-tree `TreeType` dispatch for a given `IndexLevel`
/// node. Mirrors the dispatch in
/// `add_indices_for_index_level_for_contract_operations_v1` —
/// only the range-* flags drive this upgrade because the
/// property-name level only matters for the
/// `AggregateCountOnRange` / `AggregateSumOnRange` walks.
fn property_name_tree_type_for_index_level(index_level: &IndexLevel) -> TreeType {
    let info = index_level.has_index_with_type();
    let range_countable = info.map(|i| i.range_countable).unwrap_or(false);
    let range_summable = info.map(|i| i.range_summable).unwrap_or(false);
    match (range_countable, range_summable) {
        (true, true) => TreeType::ProvableCountProvableSumTree,
        (true, false) => TreeType::ProvableCountTree,
        (false, true) => TreeType::ProvableSumTree,
        (false, false) => TreeType::NormalTree,
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
    pub(in crate::drive::document::update) fn update_document_for_contract_operations_v0(
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
            index_path.push(Vec::from(top_index_property.name.as_bytes()));

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
            let mut current_index_level = index_structure
                .sub_levels()
                .get(&top_index_property.name)
                .ok_or(Error::Drive(DriveError::CorruptedContractIndexes(format!(
                    "index structure missing top property '{}' for index '{}' — \
                         doctype's IndexLevel tree must contain every property of every \
                         registered index",
                    top_index_property.name, index.name
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
            let index_document_reference = if let Some(sum_property_name) = &index.summable {
                let sum_value = read_document_sum_contribution(document, sum_property_name)?;
                make_document_reference_with_sum_item(
                    document,
                    document_and_contract_info.document_type,
                    sum_value,
                    storage_flags,
                )
            } else {
                document_reference.clone()
            };

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
                    let value_tree_type = value_tree_type_for_index_level(current_index_level);
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

            let mut all_fields_null = document_top_field.is_empty();

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
                        // Inner property-name tree at depth i+1.
                        // Promotes to `ProvableCountTree` /
                        // `ProvableSumTree` / `ProvableCountProvableSumTree`
                        // when the level below opts into the
                        // range-* variant for the corresponding axis.
                        let property_name_tree_type =
                            property_name_tree_type_for_index_level(current_index_level);
                        let inserted = self.batch_insert_empty_tree_if_not_exists(
                            PathKeyInfo::PathKeyRef::<0>((
                                index_path.clone(),
                                index_property.name.as_bytes(),
                            )),
                            property_name_tree_type,
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
                        // another sharing the prefix).
                        let value_tree_type = value_tree_type_for_index_level(current_index_level);
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

                all_fields_null &= document_index_field.is_empty();

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

                if !index.unique {
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

                // unique indexes will be stored under key "0"
                // non unique indices should have a tree at key "0" that has all elements based off of primary key
                if !index.unique || all_fields_null {
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
                if !index.unique || all_fields_null {
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
        Ok(batch_operations)
    }
}
