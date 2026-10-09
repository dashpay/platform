use crate::drive::document::layout::{document_type_layout, LayoutKey, LayoutNode};
use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::QueryResultType;
use crate::util::batch::grovedb_op_batch::GroveDbOpBatchV0Methods;
use crate::util::batch::GroveDbOpBatch;
use crate::util::grove_operations::DirectQueryType;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::batch::{QualifiedGroveDbOp, SubelementsDeletionBehavior};
use grovedb::{AggregateData, Element, PathQuery, Query, SizedQuery, Transaction};
use grovedb_merk::element::reconstruct::ElementReconstructExtensions;
use grovedb_merk::element::tree_type::ElementTreeTypeExtensions;
use grovedb_merk::tree_type::TreeType;
use std::collections::BTreeMap;

impl Drive {
    /// Rewrites every index value key that generation 0 of
    /// `serialize_value_for_key` wrote for a property of an unsigned integer
    /// type into the key generation 1 writes for the same value.
    ///
    /// Runs once, on the first block of protocol version 14: from that
    /// version an unsigned value is keyed by its plain big-endian bytes, which
    /// sort as the values do, where generation 0 flipped the top bit as for a
    /// signed value. Generation 1 differs only by that flip, so running the
    /// rewrite again would flip the keys back; `transition_to_version_14`
    /// calls it only on a chain it has not transitioned before.
    ///
    /// Every document type of every contract in state is walked through its
    /// layout (`document_type_layout`). Each first-level index tree that holds
    /// such a key is rebuilt: its children move, a few at a time, to their new
    /// keys, each subtree written again with its elements, flags and tree
    /// types as they are. A key keeps its width, so nothing changes size.
    ///
    /// A rebuilt Merk tree takes another shape, and GroveDB refuses a node
    /// whose subtree sum leaves the signed 64-bit range, even where the whole
    /// tree's sum is in it. So an index tree is rebuilt only when every sum
    /// tree in it keeps its sums in range in any shape; one that could not,
    /// which only sums of values near the limits of the range make, keeps its
    /// earlier keys.
    ///
    /// Only the layouts earlier protocol versions write exist when this runs:
    /// the value keys of ordinary index levels. Contest vote polls keep the
    /// generation-0 keys (`serialize_value_for_vote_poll_key`), so they are not
    /// walked.
    pub fn rekey_unsigned_integer_index_values(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let contract_count = self.for_each_contract_in_state(
            transaction,
            platform_version,
            |contract_id, fetch_info| {
                for document_type in fetch_info.contract.document_types().values() {
                    self.rekey_unsigned_integer_index_values_of_document_type(
                        contract_id,
                        document_type.as_ref(),
                        transaction,
                        platform_version,
                    )?;
                }
                Ok(())
            },
        )?;

        tracing::info!(
            contract_count,
            "rewrote the unsigned integer index value keys of every contract in state"
        );

        Ok(())
    }

    fn rekey_unsigned_integer_index_values_of_document_type(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Most types index no unsigned property: they need no layout
        let indexes_unsigned_property = document_type
            .indexes()
            .values()
            .flat_map(|index| &index.properties)
            .any(|property| document_type.has_unsigned_integer_tree_key(&property.name));
        if !indexes_unsigned_property {
            return Ok(());
        }

        let layout = document_type_layout(document_type, platform_version)?;
        let document_type_path =
            contract_document_type_path_vec(&contract_id, document_type.name());
        for index_tree in &layout.root.children {
            let LayoutKey::Fixed { bytes, .. } = &index_tree.key else {
                continue;
            };
            if rekeys_within(index_tree, document_type) {
                self.rekey_index_tree(
                    &document_type_path,
                    bytes,
                    index_tree,
                    document_type,
                    transaction,
                    platform_version,
                )?;
            }
        }
        Ok(())
    }

    /// Rebuilds the first-level index tree `index_tree_key` of the document
    /// type at `document_type_path`, an instance of `node`, with every key
    /// below it that changes rewritten.
    ///
    /// Its children are moved in groups: the old keys of a group are removed
    /// in one batch, then the copies are inserted in another, and a group
    /// closes once it holds `SystemLimits::max_rekey_batch_insertions`
    /// elements. Under generation 0 a key and the same key with its top bit
    /// flipped hold two values that trade keys, so the two always move in the
    /// same group. The document type's tree sums nothing, so taking children
    /// out of this tree and putting them back changes no aggregate above it.
    fn rekey_index_tree(
        &self,
        document_type_path: &[Vec<u8>],
        index_tree_key: &[u8],
        node: &LayoutNode,
        document_type: DocumentTypeRef,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let Some(index_tree) = self.grove_get_raw_optional(
            document_type_path.into(),
            index_tree_key,
            DirectQueryType::StatefulDirectQuery,
            Some(transaction),
            &mut vec![],
            &platform_version.drive,
        )?
        else {
            return Ok(());
        };
        let Some(tree_type) = index_tree.tree_type() else {
            return Ok(());
        };
        let mut path = document_type_path.to_vec();
        path.push(index_tree_key.to_vec());

        if !self.sums_stay_in_range_in_any_shape(&path, tree_type, transaction, platform_version)? {
            tracing::warn!(
                path = %path.iter().map(hex::encode).collect::<Vec<_>>().join("/"),
                "kept the earlier keys of an index tree whose sums could leave the signed \
                 64-bit range when it is rebuilt"
            );
            return Ok(());
        }

        let max_batch_insertions = platform_version
            .system_limits
            .max_rekey_batch_insertions
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "the protocol version that rewrites unsigned index value keys sets \
                 max_rekey_batch_insertions",
            )))? as usize;

        let children: BTreeMap<Vec<u8>, Element> = self
            .stored_children(&path, transaction, platform_version)?
            .into_iter()
            .collect();
        let mut removals = GroveDbOpBatch::new();
        let mut insertions = GroveDbOpBatch::new();
        for (key, element) in &children {
            let Some(child_node) = child_node(node, key) else {
                continue;
            };
            let moves: Vec<(&Vec<u8>, &Element, Vec<u8>)> =
                match rekeyed_key(child_node, key, document_type, platform_version)? {
                    Some(new_key) => match children.get_key_value(&new_key) {
                        // The pair moves when the lower key of the two is reached
                        Some((partner_key, _)) if partner_key < key => continue,
                        Some((partner_key, partner)) => {
                            let partner_new_key = rekeyed_key(
                                child_node,
                                partner_key,
                                document_type,
                                platform_version,
                            )?
                            .unwrap_or_else(|| partner_key.clone());
                            vec![
                                (key, element, new_key),
                                (partner_key, partner, partner_new_key),
                            ]
                        }
                        None => vec![(key, element, new_key)],
                    },
                    // The key stays and keys below it change: the subtree is
                    // written again under it
                    None if child_node
                        .children
                        .iter()
                        .any(|below| rekeys_within(below, document_type)) =>
                    {
                        vec![(key, element, key.clone())]
                    }
                    None => continue,
                };
            for (key, element, new_key) in moves {
                match element.tree_type() {
                    Some(tree_type) => removals.push(
                        QualifiedGroveDbOp::delete_tree_op(
                            path.clone(),
                            key.clone(),
                            tree_type,
                            SubelementsDeletionBehavior::DeleteChildren,
                        )
                        .dont_check_for_backwards_references(),
                    ),
                    None => removals.add_delete(path.clone(), key.clone()),
                }
                self.copy_rekeyed(
                    &path,
                    key,
                    element.clone(),
                    &path,
                    new_key,
                    Some(child_node),
                    document_type,
                    &mut insertions,
                    transaction,
                    platform_version,
                )?;
            }
            if insertions.len() >= max_batch_insertions {
                self.apply_rekey_batches(
                    &mut removals,
                    &mut insertions,
                    transaction,
                    platform_version,
                )?;
            }
        }
        self.apply_rekey_batches(
            &mut removals,
            &mut insertions,
            transaction,
            platform_version,
        )
    }

    /// Applies `removals`, then `insertions`, leaving both empty.
    fn apply_rekey_batches(
        &self,
        removals: &mut GroveDbOpBatch,
        insertions: &mut GroveDbOpBatch,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if removals.is_empty() {
            return Ok(());
        }
        self.grove_apply_batch(
            std::mem::take(removals),
            false,
            Some(transaction),
            &platform_version.drive,
        )?;
        self.grove_apply_batch(
            std::mem::take(insertions),
            false,
            Some(transaction),
            &platform_version.drive,
        )
    }

    /// Whether every tree at or below `path` (a `tree_type` tree), itself
    /// included, keeps its sums in the signed 64-bit range whatever shape it is
    /// rebuilt in: the magnitudes of its children's sums add up to at most
    /// `i64::MAX`, so no part of them, and no partial sum a Merk node computes
    /// while rebuilding, can leave the range. Counts only grow up to the
    /// stored total, so they always stay in range.
    fn sums_stay_in_range_in_any_shape(
        &self,
        path: &[Vec<u8>],
        tree_type: TreeType,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let mut pending = vec![(path.to_vec(), tree_type)];
        while let Some((path, tree_type)) = pending.pop() {
            let children = self.stored_children(&path, transaction, platform_version)?;
            if sums_in_i64(tree_type) {
                let magnitude = children.iter().fold(0u128, |total, (_, child)| {
                    total.saturating_add(u128::from(child.sum_value_or_default().unsigned_abs()))
                });
                if magnitude > i64::MAX as u128 {
                    return Ok(false);
                }
            }
            for (key, child) in children {
                if let Some(child_tree_type) = child.tree_type() {
                    let mut child_path = path.clone();
                    child_path.push(key);
                    pending.push((child_path, child_tree_type));
                }
            }
        }
        Ok(true)
    }

    /// Adds to `insertions` the element stored at `parent` / `key`, written
    /// at `new_parent` / `new_key`, and everything below it, with the keys
    /// below that change rewritten. A tree is written empty, of its own type
    /// and with its flags (and wrapper), and its children are written under
    /// it; the batch derives its root key and aggregates from them.
    #[allow(clippy::too_many_arguments)]
    fn copy_rekeyed(
        &self,
        parent: &[Vec<u8>],
        key: &[u8],
        element: Element,
        new_parent: &[Vec<u8>],
        new_key: Vec<u8>,
        node: Option<&LayoutNode>,
        document_type: DocumentTypeRef,
        insertions: &mut GroveDbOpBatch,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if element.is_any_tree() {
            let empty = empty_tree_like(&element).ok_or(Error::Drive(
                DriveError::CorruptedCodeExecution(
                    "a document type holds only Merk trees, which can be moved",
                ),
            ))?;
            insertions.add_insert(new_parent.to_vec(), new_key.clone(), empty);
            let mut path = parent.to_vec();
            path.push(key.to_vec());
            let mut new_path = new_parent.to_vec();
            new_path.push(new_key);
            for (child_key, child_element) in
                self.stored_children(&path, transaction, platform_version)?
            {
                let child_node = node.and_then(|node| child_node(node, &child_key));
                let new_child_key = match child_node {
                    Some(child_node) => {
                        rekeyed_key(child_node, &child_key, document_type, platform_version)?
                    }
                    None => None,
                }
                .unwrap_or_else(|| child_key.clone());
                self.copy_rekeyed(
                    &path,
                    &child_key,
                    child_element,
                    &new_path,
                    new_child_key,
                    child_node,
                    document_type,
                    insertions,
                    transaction,
                    platform_version,
                )?;
            }
            return Ok(());
        }
        let element = self.stored_leaf(parent, key, element, transaction, platform_version)?;
        insertions.add_insert(new_parent.to_vec(), new_key, element);
        Ok(())
    }

    /// Every child of the tree at `path`, with the element stored there.
    fn stored_children(
        &self,
        path: &[Vec<u8>],
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Vec<u8>, Element)>, Error> {
        let path_query = PathQuery::new(
            path.to_vec(),
            SizedQuery::new(Query::new_range_full(), None, None),
        );
        let (children, _) = self.grove_get_raw_path_query(
            &path_query,
            Some(transaction),
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;
        Ok(children.to_key_elements())
    }

    /// A leaf as it is stored. A query returns a reference with its path made
    /// absolute, so a reference is read again on its own.
    fn stored_leaf(
        &self,
        parent: &[Vec<u8>],
        key: &[u8],
        element: Element,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<Element, Error> {
        if !element.is_reference() {
            return Ok(element);
        }
        self.grove_get_raw(
            parent.into(),
            key,
            DirectQueryType::StatefulDirectQuery,
            Some(transaction),
            &mut vec![],
            &platform_version.drive,
        )?
        .ok_or(Error::Drive(DriveError::CorruptedDriveState(
            "a reference a query returned can not be read".to_string(),
        )))
    }
}

/// The layout node a child of an instance of `node` keyed `key` is an
/// instance of: the fixed child with that key, or else the family of keys
/// the node holds. `None` for a key the layout does not describe, which
/// is moved as it is.
fn child_node<'a>(node: &'a LayoutNode, key: &[u8]) -> Option<&'a LayoutNode> {
    node.children
        .iter()
        .find(|child| matches!(&child.key, LayoutKey::Fixed { bytes, .. } if bytes == key))
        .or_else(|| {
            node.children
                .iter()
                .find(|child| !matches!(child.key, LayoutKey::Fixed { .. }))
        })
}

/// Whether the keys of `node`'s family change: the value keys of an
/// unsigned integer property.
fn rekeys(node: &LayoutNode, document_type: DocumentTypeRef) -> bool {
    match &node.key {
        LayoutKey::PropertyValue { property } => {
            document_type.has_unsigned_integer_tree_key(property)
        }
        _ => false,
    }
}

/// Whether any key of `node`'s family or below it changes.
fn rekeys_within(node: &LayoutNode, document_type: DocumentTypeRef) -> bool {
    rekeys(node, document_type)
        || node
            .children
            .iter()
            .any(|child| rekeys_within(child, document_type))
}

/// The new key of a child keyed `key`, an instance of `node`, when it
/// changes.
fn rekeyed_key(
    node: &LayoutNode,
    key: &[u8],
    document_type: DocumentTypeRef,
    platform_version: &PlatformVersion,
) -> Result<Option<Vec<u8>>, Error> {
    let LayoutKey::PropertyValue { property } = &node.key else {
        return Ok(None);
    };
    let new_key = document_type.tree_key_from_generation_0(property, key, platform_version)?;
    Ok((new_key != key).then_some(new_key))
}

/// Whether a tree of this type sums its children in a signed 64-bit value.
fn sums_in_i64(tree_type: TreeType) -> bool {
    matches!(
        tree_type,
        TreeType::SumTree
            | TreeType::CountSumTree
            | TreeType::ProvableCountSumTree
            | TreeType::ProvableSumTree
            | TreeType::ProvableCountProvableSumTree
            | TreeType::ProvableSumIndexedTree
            | TreeType::ProvableCountProvableSumIndexedTree
    )
}

/// The empty tree of `element`'s type, with its flags and wrapper: what a
/// batch writes before the children it derives the root key and aggregates
/// from. `None` for an element that is no Merk tree.
fn empty_tree_like(element: &Element) -> Option<Element> {
    if element.uses_non_merk_data_storage() {
        return None;
    }
    element
        .reconstruct_with_root_key(None, AggregateData::NoAggregateData)
        .or_else(|| {
            element.reconstruct_with_two_root_keys(None, None, AggregateData::NoAggregateData)
        })
        .or_else(|| {
            let axes = element
                .axes()?
                .iter()
                .map(|(axis, _)| (*axis, None))
                .collect();
            element.reconstruct_with_axes(None, AggregateData::NoAggregateData, axes)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::votes::paths::VotePollPaths;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
    use crate::query::unsigned_index_key_order_tests::{
        above_100, count_grades, insert_document, insert_grade, query_grades, query_values,
        setup_grades,
    };
    use crate::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DataContractOwnedResolvedInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::{DataContract, DataContractFactory};
    use dpp::document::{Document, DocumentV0};
    use dpp::identifier::Identifier;
    use dpp::platform_value::{platform_value, Value};
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
    use dpp::voting::vote_polls::VotePoll;
    use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
    use dpp::voting::votes::resource_vote::ResourceVote;
    use std::borrow::Cow;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const HIGH: u64 = 1 << 63;

    fn protocol_version_13() -> &'static PlatformVersion {
        PlatformVersion::get(13).expect("expected protocol version 13")
    }

    /// Every element under `path`, keyed by its path below it, as it is
    /// stored: a tree as its type, flags and wrapper, with the aggregates of
    /// the tree inside any wrapper.
    fn dump(
        drive: &Drive,
        path: &[Vec<u8>],
        transaction: &Transaction,
    ) -> BTreeMap<Vec<Vec<u8>>, (Element, u64, i64)> {
        let platform_version = PlatformVersion::latest();
        let mut out = BTreeMap::new();
        for (key, element) in drive
            .stored_children(path, transaction, platform_version)
            .expect("expected the children")
        {
            let summary = if element.is_any_tree() {
                (
                    empty_tree_like(&element).expect("a Merk tree"),
                    element.underlying().count_value_or_default(),
                    element.underlying().sum_value_or_default(),
                )
            } else {
                (
                    drive
                        .stored_leaf(path, &key, element, transaction, platform_version)
                        .expect("expected the leaf"),
                    0,
                    0,
                )
            };
            let is_tree = summary.0.is_any_tree();
            out.insert(vec![key.clone()], summary);
            if is_tree {
                let mut child_path = path.to_vec();
                child_path.push(key.clone());
                for (below, summary) in dump(drive, &child_path, transaction) {
                    let mut full = vec![key.clone()];
                    full.extend(below);
                    out.insert(full, summary);
                }
            }
        }
        out
    }

    /// The dump of the document type `name` of `contract`.
    fn dump_document_type(
        drive: &Drive,
        contract: &DataContract,
        name: &str,
    ) -> BTreeMap<Vec<Vec<u8>>, (Element, u64, i64)> {
        let transaction = drive.grove.start_transaction();
        dump(
            drive,
            &contract_document_type_path_vec(contract.id().as_bytes(), name),
            &transaction,
        )
    }

    /// `dump`'s path with the value keys of `unsigned` properties (name,
    /// width) given their top bit back: the path generation 1 keys the same
    /// entry under. Index paths alternate property names and values from the
    /// document type down, up to the `[0]` terminal; `[0]` first is the
    /// primary key tree.
    fn rekeyed_path(path: &[Vec<u8>], unsigned: &[(&str, usize)]) -> Vec<Vec<u8>> {
        let mut path = path.to_vec();
        let mut position = 0;
        while position + 1 < path.len() && path[position] != vec![0] {
            let name = String::from_utf8(path[position].clone()).expect("a property name");
            let value = &mut path[position + 1];
            if unsigned
                .iter()
                .any(|(property, width)| *property == name && value.len() == *width)
            {
                value[0] ^= 0x80;
            }
            position += 2;
        }
        path
    }

    /// The dump expected after the rewrite: `before` with the value keys of
    /// `unsigned` moved.
    fn rekeyed_dump(
        before: BTreeMap<Vec<Vec<u8>>, (Element, u64, i64)>,
        unsigned: &[(&str, usize)],
    ) -> BTreeMap<Vec<Vec<u8>>, (Element, u64, i64)> {
        before
            .into_iter()
            .map(|(path, summary)| (rekeyed_path(&path, unsigned), summary))
            .collect()
    }

    fn rekey(drive: &Drive) {
        let transaction = drive.grove.start_transaction();
        drive
            .rekey_unsigned_integer_index_values(&transaction, PlatformVersion::latest())
            .expect("expected the rekey to run");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");
    }

    /// A drive at protocol version 13 holding `contract`.
    fn setup_contract_at_13(contract: &DataContract) -> Drive {
        let platform_version = protocol_version_13();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        drive
            .apply_contract(
                contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the contract");
        drive
    }

    /// A contract with one document type `name`, built at protocol version 13.
    fn contract_at_13(owner: u8, name: &str, document_type: Value) -> DataContract {
        DataContractFactory::new(13)
            .expect("expected a contract factory")
            .create_with_value_config(
                Identifier::new([owner; 32]),
                0,
                Value::Map(vec![(Value::Text(name.to_string()), document_type)]),
                None,
                None,
            )
            .expect("expected the contract")
            .data_contract_owned()
    }

    fn document(id: u8, properties: Vec<(&str, Value)>) -> Document {
        DocumentV0 {
            id: Identifier::new([id; 32]),
            owner_id: Identifier::new([1; 32]),
            properties: properties
                .into_iter()
                .map(|(name, value)| (name.to_string(), value))
                .collect(),
            ..Default::default()
        }
        .into()
    }

    #[test]
    fn should_rekey_grades_written_before_protocol_version_14() {
        let (drive, contract) = setup_grades(protocol_version_13());
        let before = dump_document_type(&drive, &contract, "grade");

        rekey(&drive);

        assert_eq!(
            dump_document_type(&drive, &contract, "grade"),
            rekeyed_dump(before, &[("grade", 1)]),
            "only the grade keys change"
        );
        let platform_version = PlatformVersion::latest();
        assert_eq!(
            count_grades(&drive, &contract, above_100(), platform_version)
                .expect("expected the count"),
            5
        );
        assert_eq!(
            query_grades(
                &drive,
                &contract,
                "select * from grade where grade > 100 order by grade asc",
                platform_version,
            ),
            vec![127, 128, 150, 200, 255]
        );

        // A grade written after the rewrite sits among the rewritten ones
        insert_grade(&drive, &contract, 100, 129, platform_version);
        assert_eq!(
            query_grades(
                &drive,
                &contract,
                "select * from grade where grade > 126 order by grade asc limit 3",
                platform_version,
            ),
            vec![127, 128, 129]
        );
    }

    /// The `item` type: a u64 `amount` (`minimum` alone picks u64) under a
    /// string prefix, an optional u16 `rank`, and a unique index led by the
    /// amount.
    fn item_contract() -> DataContract {
        contract_at_13(
            8,
            "item",
            platform_value!({
                "type": "object",
                "properties": {
                    "category": {"type": "string", "maxLength": 10, "position": 0},
                    "amount": {"type": "integer", "minimum": 0, "position": 1},
                    "rank": {"type": "integer", "minimum": 0, "maximum": 60000, "position": 2},
                    "label": {"type": "string", "maxLength": 10, "position": 3},
                },
                "required": ["category", "amount", "label"],
                "indices": [
                    {
                        "name": "byCategoryAmount",
                        "properties": [{"category": "asc"}, {"amount": "asc"}],
                        "countable": "countable",
                    },
                    {"name": "byRank", "properties": [{"rank": "asc"}]},
                    {
                        "name": "byAmountLabel",
                        "properties": [{"amount": "asc"}, {"label": "asc"}],
                        "unique": true,
                    },
                ],
                "additionalProperties": false,
            }),
        )
    }

    fn item(id: u8, category: &str, amount: u64, rank: Option<u16>, label: &str) -> Document {
        let mut properties = vec![
            ("category", Value::Text(category.to_string())),
            ("amount", Value::U64(amount)),
            ("label", Value::Text(label.to_string())),
        ];
        if let Some(rank) = rank {
            properties.push(("rank", Value::U16(rank)));
        }
        document(id, properties)
    }

    fn query_items(drive: &Drive, contract: &DataContract, sql: &str, property: &str) -> Vec<u64> {
        query_values(
            drive,
            contract,
            "item",
            sql,
            property,
            PlatformVersion::latest(),
        )
    }

    #[test]
    fn should_rekey_nested_unique_and_null_layouts_and_swap_colliding_keys() {
        let contract = item_contract();
        let drive = setup_contract_at_13(&contract);
        // 7 and HIGH + 7, and the ranks 3 and 32771, trade keys between the
        // two generations
        for document in [
            item(1, "a", 7, Some(3), "x"),
            item(2, "a", HIGH + 7, Some(32771), "y"),
            item(3, "b", 7, None, "z"),
            item(4, "b", 300, Some(3), "w"),
            item(5, "a", HIGH + 300, None, "v"),
        ] {
            insert_document(&drive, &contract, "item", &document, protocol_version_13());
        }
        let before = dump_document_type(&drive, &contract, "item");

        rekey(&drive);

        assert_eq!(
            dump_document_type(&drive, &contract, "item"),
            rekeyed_dump(before, &[("amount", 8), ("rank", 2)]),
            "only the amount and rank keys change"
        );
        assert_eq!(
            query_items(
                &drive,
                &contract,
                "select * from item where category = 'a' and amount > 5 order by amount asc",
                "amount",
            ),
            vec![7, HIGH + 7, HIGH + 300]
        );
        assert_eq!(
            query_items(
                &drive,
                &contract,
                "select * from item where amount > 0 order by amount desc",
                "amount",
            ),
            vec![HIGH + 300, HIGH + 7, 300, 7, 7]
        );
        assert_eq!(
            query_items(
                &drive,
                &contract,
                "select * from item where rank > 1 order by rank asc",
                "rank",
            ),
            vec![3, 3, 32771]
        );

        // A rewritten entry is found where protocol version 14 looks for it
        let platform_version = PlatformVersion::latest();
        let document_type = contract
            .document_type_for_name("item")
            .expect("expected the item type");
        drive
            .delete_document_for_contract(
                Identifier::new([2; 32]),
                &contract,
                document_type.name(),
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to delete a rewritten item");
        insert_document(
            &drive,
            &contract,
            "item",
            &item(6, "a", HIGH, Some(32768), "u"),
            platform_version,
        );
        assert_eq!(
            query_items(
                &drive,
                &contract,
                "select * from item where category = 'a' and amount > 5 order by amount asc",
                "amount",
            ),
            vec![7, HIGH, HIGH + 300]
        );
        assert_eq!(
            query_items(
                &drive,
                &contract,
                "select * from item where rank > 1 order by rank asc",
                "rank",
            ),
            vec![3, 3, 32768]
        );

        // A rewritten document is updated where protocol version 14 keys it
        drive
            .update_document_for_contract(
                &item(1, "a", HIGH + 1, Some(40000), "x"),
                &contract,
                document_type,
                None,
                BlockInfo::default(),
                true,
                Some(Cow::Owned(StorageFlags::SingleEpoch(0))),
                None,
                platform_version,
                None,
            )
            .expect("expected to update a rewritten item");
        assert_eq!(
            query_items(
                &drive,
                &contract,
                "select * from item where category = 'a' and amount > 5 order by amount asc",
                "amount",
            ),
            vec![HIGH, HIGH + 1, HIGH + 300]
        );
        assert_eq!(
            query_items(
                &drive,
                &contract,
                "select * from item where rank > 1 order by rank asc",
                "rank",
            ),
            vec![3, 32768, 40000]
        );
    }

    /// A `score` type summing a signed `weight` by a u8 `grade`, both on its
    /// own (`byGrade`) and under a range-summable `[grade, tag]` continuation,
    /// which a summing value tree holds behind a wrapper that keeps it out of
    /// the sum.
    fn score_contract(weight: Value) -> DataContract {
        contract_at_13(
            9,
            "score",
            platform_value!({
                "type": "object",
                "properties": {
                    "grade": {"type": "integer", "minimum": 0, "maximum": 255, "position": 0},
                    "weight": weight,
                    "tag": {"type": "string", "maxLength": 10, "position": 2},
                },
                "required": ["grade", "weight", "tag"],
                "indices": [
                    {
                        "name": "byGrade",
                        "properties": [{"grade": "asc"}],
                        "summable": "weight",
                        "rangeSummable": true,
                    },
                    {
                        "name": "byGradeTag",
                        "properties": [{"grade": "asc"}, {"tag": "asc"}],
                        "summable": "weight",
                        "rangeSummable": true,
                    },
                ],
                "additionalProperties": false,
            }),
        )
    }

    fn score(id: u8, grade: u8, weight: i64, tag: &str) -> Document {
        document(
            id,
            vec![
                ("grade", Value::U8(grade)),
                ("weight", Value::I64(weight)),
                ("tag", Value::Text(tag.to_string())),
            ],
        )
    }

    #[test]
    fn should_rekey_sum_trees_and_the_wrapped_continuations_under_them() {
        let contract = score_contract(platform_value!(
            {"type": "integer", "minimum": -1000, "maximum": 1000, "position": 1}
        ));
        let drive = setup_contract_at_13(&contract);
        for (id, grade, weight, tag) in [
            (1, 5, 10, "a"),
            (2, 100, -20, "b"),
            (3, 200, 30, "a"),
            (4, 255, -40, "c"),
            (5, 200, 50, "d"),
        ] {
            insert_document(
                &drive,
                &contract,
                "score",
                &score(id, grade, weight, tag),
                protocol_version_13(),
            );
        }
        let before = dump_document_type(&drive, &contract, "score");
        assert!(
            before.values().any(|(element, ..)| element.is_wrapped()),
            "the continuations sit behind a wrapper"
        );

        rekey(&drive);

        assert_eq!(
            dump_document_type(&drive, &contract, "score"),
            rekeyed_dump(before, &[("grade", 1)]),
            "only the grade keys change, sums and wrappers included"
        );
        assert_eq!(
            query_values(
                &drive,
                &contract,
                "score",
                "select * from score where grade > 100 order by grade asc",
                "grade",
                PlatformVersion::latest(),
            ),
            vec![200, 200, 255]
        );
    }

    /// The sums of these grades fit, but a tree rebuilt in the new key order
    /// would hold grades 1 and 2 in one subtree summing past `i64::MAX`.
    #[test]
    fn should_keep_the_keys_of_an_index_whose_sums_could_leave_the_range_when_rebuilt() {
        const M: i64 = 1 << 62;
        let contract = score_contract(platform_value!({"type": "integer", "position": 1}));
        let drive = setup_contract_at_13(&contract);
        for (id, grade, weight) in [(1, 1, M), (2, 128, -M), (3, 2, M), (4, 3, -M)] {
            insert_document(
                &drive,
                &contract,
                "score",
                &score(id, grade, weight, "a"),
                protocol_version_13(),
            );
        }
        let before = dump_document_type(&drive, &contract, "score");

        rekey(&drive);

        assert_eq!(
            dump_document_type(&drive, &contract, "score"),
            before,
            "the index keeps its earlier keys"
        );
    }

    /// A `seat` type whose numbers (a u8) are contested.
    fn seat_contract() -> DataContract {
        contract_at_13(
            10,
            "seat",
            platform_value!({
                "type": "object",
                "documentsMutable": false,
                "properties": {
                    "number": {"type": "integer", "minimum": 0, "maximum": 255, "position": 0},
                },
                "required": ["number"],
                "indices": [{
                    "name": "byNumber",
                    "properties": [{"number": "asc"}],
                    "unique": true,
                    "contested": {"resolution": 0},
                }],
                "additionalProperties": false,
            }),
        )
    }

    fn add_seat_contender(
        drive: &Drive,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        owner: u8,
        starts_the_contest: bool,
        platform_version: &PlatformVersion,
    ) {
        let owner_id = Identifier::new([owner; 32]);
        let document: Document = DocumentV0 {
            id: Identifier::new([owner; 32]),
            owner_id,
            properties: BTreeMap::from([("number".to_string(), Value::U8(200))]),
            ..Default::default()
        }
        .into();
        let block_info = BlockInfo::default();
        drive
            .add_contested_document(
                OwnedDocumentInfo {
                    document_info: DocumentRefInfo((
                        &document,
                        StorageFlags::optional_default_as_cow(),
                    )),
                    owner_id: Some(owner_id.to_buffer()),
                },
                vote_poll.clone(),
                false,
                starts_the_contest.then(|| {
                    ContestedDocumentVotePollStoredInfo::new(block_info, platform_version)
                        .expect("expected the poll's stored info")
                }),
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to add the contender");
    }

    /// A contest on an unsigned value started, and voted on, before protocol
    /// version 14 is joined after it at the same poll, keeps the vote, and
    /// lists and proves its value as it is.
    #[test]
    fn should_keep_a_contest_on_an_unsigned_value_at_its_poll_across_the_upgrade() {
        let contract = seat_contract();
        let drive = setup_contract_at_13(&contract);
        let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
            document_type_name: "seat".to_string(),
            index_name: "byNumber".to_string(),
            index_values: vec![Value::U8(200)],
        };
        add_seat_contender(&drive, &vote_poll, 21, true, protocol_version_13());
        let voter = [9; 32];
        let towards_first = ResourceVoteChoice::TowardsIdentity(Identifier::new([21; 32]));
        drive
            .register_contested_resource_identity_vote(
                voter,
                1,
                vote_poll.clone(),
                towards_first,
                None,
                &BlockInfo::default(),
                None,
                protocol_version_13(),
            )
            .expect("expected to record the vote");

        rekey(&drive);

        let platform_version = PlatformVersion::latest();
        add_seat_contender(&drive, &vote_poll, 22, false, platform_version);
        let contenders_path = vote_poll
            .contenders_path(platform_version)
            .expect("expected the contenders path");
        assert_eq!(
            contenders_path,
            vote_poll
                .contenders_path(protocol_version_13())
                .expect("expected the contenders path"),
            "the poll keeps its path"
        );
        let transaction = drive.grove.start_transaction();
        let contenders: Vec<Vec<u8>> = drive
            .stored_children(&contenders_path, &transaction, platform_version)
            .expect("expected the contenders")
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        assert!(contenders.contains(&vec![21; 32]), "the first contender");
        assert!(contenders.contains(&vec![22; 32]), "the second contender");
        let voting_path = vote_poll
            .contender_voting_path(&towards_first, platform_version)
            .expect("expected the voting path");
        let voters: Vec<Vec<u8>> = drive
            .stored_children(&voting_path, &transaction, platform_version)
            .expect("expected the voters")
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        assert_eq!(voters, vec![voter.to_vec()], "the vote cast before");
        drop(transaction);

        let listing = VotePollsByDocumentTypeQuery {
            contract_id: contract.id(),
            document_type_name: "seat".to_string(),
            index_name: "byNumber".to_string(),
            start_index_values: vec![],
            end_index_values: vec![],
            start_at_value: None,
            limit: None,
            order_ascending: true,
        };
        let contested_values = listing
            .execute_no_proof(&drive, None, &mut vec![], platform_version)
            .expect("expected the contested values");
        assert_eq!(contested_values, vec![Value::U8(200)]);
        let proof = listing
            .clone()
            .execute_with_proof(&drive, None, &mut vec![], platform_version)
            .expect("expected the contested values proof");
        let (_, proved_values) = listing
            .resolve_with_provided_borrowed_contract(&contract)
            .expect("expected to resolve the listing")
            .verify_contests_proof(&proof, platform_version)
            .expect("expected the proof to verify");
        assert_eq!(proved_values, vec![Value::U8(200)]);

        // The voter's record of the vote still names the poll's value
        let votes_given = ContestedResourceVotesGivenByIdentityQuery {
            identity_id: Identifier::new(voter),
            offset: None,
            limit: None,
            start_at: None,
            order_ascending: true,
        };
        let (proof, _) = votes_given
            .clone()
            .execute_with_proof(&drive, None, None, platform_version)
            .expect("expected the votes given proof");
        let known_contract = Arc::new(contract.clone());
        let (_, votes): (_, Vec<(Identifier, ResourceVote)>) = votes_given
            .verify_identity_votes_given_proof(
                &proof,
                &|_| Ok(Some(known_contract.clone())),
                platform_version,
            )
            .expect("expected the proof to verify");
        let [(_, vote)] = votes.as_slice() else {
            panic!("expected the one vote, got {votes:?}");
        };
        let VotePoll::ContestedDocumentResourceVotePoll(poll) = vote.vote_poll();
        assert_eq!(poll.index_values, vec![Value::U8(200)]);
        assert_eq!(vote.resource_vote_choice(), towards_first);
    }
}
