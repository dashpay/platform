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

/// A child of a tree whose key changes: its key, its new key, the element
/// stored there and the layout node it is an instance of.
type Rekeyed<'a> = (Vec<u8>, Vec<u8>, Element, &'a LayoutNode);

impl Drive {
    /// Rewrites every index value key that generation 0 of
    /// `serialize_value_for_key` wrote for a property of an unsigned integer
    /// type into the key generation 1 writes for the same value.
    ///
    /// Runs once, on the first block of protocol version 14: from that
    /// version an unsigned value is keyed by its plain big-endian bytes, which
    /// sort as the values do, where generation 0 flipped the top bit as for a
    /// signed value. Every document type of every contract in state is
    /// walked through its layout (`document_type_layout`), and each subtree
    /// under a value key of such a property moves to the new key with its
    /// elements, flags and tree types as they are. A key keeps its width, so
    /// nothing changes size.
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
        let mut start_at = None;
        let mut contract_count = 0usize;
        let mut rekeyed_count = 0usize;

        loop {
            let page =
                self.fetch_contract_ids(start_at, u16::MAX, Some(transaction), platform_version)?;

            for contract_id in &page {
                rekeyed_count += self.rekey_unsigned_integer_index_values_of_contract(
                    *contract_id,
                    transaction,
                    platform_version,
                )?;
            }
            contract_count += page.len();

            match page.last() {
                Some(last_id) if page.len() == u16::MAX as usize => {
                    start_at = Some((*last_id, false));
                }
                _ => break,
            }
        }

        tracing::info!(
            contract_count,
            rekeyed_count,
            "rewrote the unsigned integer index value keys of every contract in state"
        );

        Ok(())
    }

    /// Rewrites the unsigned integer index value keys of one contract's
    /// document types, returning how many keys moved.
    fn rekey_unsigned_integer_index_values_of_contract(
        &self,
        contract_id: [u8; 32],
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<usize, Error> {
        let fetch_info = self
            .fetch_contract_and_add_operations(
                contract_id,
                None,
                Some(transaction),
                &mut vec![],
                platform_version,
            )?
            .ok_or_else(|| {
                Error::Drive(DriveError::CorruptedDriveState(format!(
                    "contract {} is listed under the contracts root but can not be fetched",
                    hex::encode(contract_id)
                )))
            })?;

        let mut rekeyed_count = 0;
        for document_type in fetch_info.contract.document_types().values() {
            let document_type = document_type.as_ref();
            let layout = document_type_layout(document_type, platform_version)?;
            if !rekeys_within(&layout.root, document_type) {
                continue;
            }
            let path = contract_document_type_path_vec(&contract_id, document_type.name());
            rekeyed_count += self.rekey_children(
                &path,
                &layout.root,
                document_type,
                transaction,
                platform_version,
            )?;
        }
        Ok(rekeyed_count)
    }

    /// Rewrites the keys under the tree at `path`, an instance of `node`:
    /// the children whose keys change move, and the others are walked for
    /// keys below them that change. Returns how many keys moved.
    fn rekey_children(
        &self,
        path: &[Vec<u8>],
        node: &LayoutNode,
        document_type: DocumentTypeRef,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<usize, Error> {
        let mut rekeyed: Vec<Rekeyed> = Vec::new();
        let mut rekeyed_count = 0;
        for (key, element) in self.stored_children(path, transaction, platform_version)? {
            let Some(child_node) = child_node(node, &key) else {
                continue;
            };
            if !rekeys_within(child_node, document_type) {
                continue;
            }
            match rekeyed_key(child_node, &key, document_type, platform_version)? {
                Some(new_key) => rekeyed.push((key, new_key, element, child_node)),
                None if element.is_any_tree() => {
                    let mut child_path = path.to_vec();
                    child_path.push(key);
                    rekeyed_count += self.rekey_children(
                        &child_path,
                        child_node,
                        document_type,
                        transaction,
                        platform_version,
                    )?;
                }
                None => {}
            }
        }
        if rekeyed.is_empty() {
            return Ok(rekeyed_count);
        }
        rekeyed_count += rekeyed.len();

        // Every old key is removed before any new key is written: under
        // generation 0, a key and the same key with its top bit flipped hold
        // two different values, so a new key can be another child's old one
        let mut removals = GroveDbOpBatch::new();
        let mut insertions = GroveDbOpBatch::new();
        for (key, new_key, element, child_node) in rekeyed {
            match element.tree_type() {
                Some(tree_type) => removals.push(
                    QualifiedGroveDbOp::delete_tree_op(
                        path.to_vec(),
                        key.clone(),
                        tree_type,
                        SubelementsDeletionBehavior::DeleteChildren,
                    )
                    .dont_check_for_backwards_references(),
                ),
                None => removals.add_delete(path.to_vec(), key.clone()),
            }
            self.copy_rekeyed(
                path,
                &key,
                element,
                path,
                new_key,
                Some(child_node),
                document_type,
                &mut insertions,
                transaction,
                platform_version,
            )?;
        }
        self.grove_apply_batch(removals, false, Some(transaction), &platform_version.drive)?;
        self.grove_apply_batch(
            insertions,
            false,
            Some(transaction),
            &platform_version.drive,
        )?;
        Ok(rekeyed_count)
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
        || node
            .alternative
            .as_ref()
            .is_some_and(|alternative| rekeys_within(&alternative.node, document_type))
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

/// The empty tree of `element`'s type, with its flags and wrapper: what a
/// batch writes before the children it derives the root key and aggregates
/// from. `None` for an element that is no Merk tree.
fn empty_tree_like(element: &Element) -> Option<Element> {
    match element {
        Element::NonCounted(inner) => {
            empty_tree_like(inner).map(|inner| Element::NonCounted(Box::new(inner)))
        }
        Element::NotSummed(inner) => {
            empty_tree_like(inner).map(|inner| Element::NotSummed(Box::new(inner)))
        }
        Element::NotCountedOrSummed(inner) => {
            empty_tree_like(inner).map(|inner| Element::NotCountedOrSummed(Box::new(inner)))
        }
        Element::ProvableSumIndexedTree(_, _, _, flags) => Some(Element::ProvableSumIndexedTree(
            None,
            None,
            0,
            flags.clone(),
        )),
        Element::ProvableCountIndexedTree(_, _, _, flags) => Some(
            Element::ProvableCountIndexedTree(None, None, 0, flags.clone()),
        ),
        Element::ProvableCountProvableSumIndexedTree(_, _, _, axes, flags) => {
            Some(Element::ProvableCountProvableSumIndexedTree(
                None,
                0,
                0,
                axes.iter().map(|(axis, _)| (*axis, None)).collect(),
                flags.clone(),
            ))
        }
        element if element.uses_non_merk_data_storage() => None,
        element => element.reconstruct_with_root_key(None, AggregateData::NoAggregateData),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::unsigned_index_key_order_tests::{
        above_100, count_grades, insert_grade, query_grades, setup_grades,
    };
    use crate::query::DriveDocumentQuery;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::{DataContract, DataContractFactory};
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::{Document, DocumentV0, DocumentV0Getters};
    use dpp::identifier::Identifier;
    use dpp::platform_value::{platform_value, Value};
    use std::borrow::Cow;
    use std::collections::BTreeMap;

    const HIGH: u64 = 1 << 63;

    fn protocol_version_13() -> &'static PlatformVersion {
        PlatformVersion::get(13).expect("expected protocol version 13")
    }

    /// Every element under `path`, keyed by its path below it, as it is
    /// stored: a tree as its type, flags and wrapper, with its aggregates.
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
                    element.count_value_or_default(),
                    element.sum_value_or_default(),
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

    fn document_type_path(contract: &DataContract, name: &str) -> Vec<Vec<u8>> {
        contract_document_type_path_vec(contract.id().as_bytes(), name)
    }

    #[test]
    fn should_rekey_grades_written_before_protocol_version_14() {
        let (drive, contract) = setup_grades(protocol_version_13());
        let path = document_type_path(&contract, "grade");
        let transaction = drive.grove.start_transaction();
        let before = dump(&drive, &path, &transaction);
        drop(transaction);

        rekey(&drive);

        let transaction = drive.grove.start_transaction();
        let after = dump(&drive, &path, &transaction);
        let expected: BTreeMap<_, _> = before
            .into_iter()
            .map(|(path, summary)| (rekeyed_path(&path, &[("grade", 1)]), summary))
            .collect();
        assert_eq!(after, expected, "only the grade keys change");
        drop(transaction);

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

    /// The `item` contract: a u64 `amount` (`minimum` alone picks u64) under a
    /// string prefix, an optional u16 `rank`, and a unique index led by the
    /// amount.
    fn item_contract(platform_version: &PlatformVersion) -> DataContract {
        let factory = DataContractFactory::new(platform_version.protocol_version)
            .expect("expected a contract factory");
        let item = platform_value!({
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
        });
        factory
            .create_with_value_config(
                Identifier::new([8; 32]),
                0,
                platform_value!({ "item": item }),
                None,
                None,
            )
            .expect("expected the item contract")
            .data_contract_owned()
    }

    fn item(id: u8, category: &str, amount: u64, rank: Option<u16>, label: &str) -> Document {
        let mut properties = BTreeMap::from([
            ("category".to_string(), Value::Text(category.to_string())),
            ("amount".to_string(), Value::U64(amount)),
            ("label".to_string(), Value::Text(label.to_string())),
        ]);
        if let Some(rank) = rank {
            properties.insert("rank".to_string(), Value::U16(rank));
        }
        DocumentV0 {
            id: Identifier::new([id; 32]),
            owner_id: Identifier::new([1; 32]),
            properties,
            ..Default::default()
        }
        .into()
    }

    fn insert_item(
        drive: &Drive,
        contract: &DataContract,
        document: &Document,
        platform_version: &PlatformVersion,
    ) {
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((
                            document,
                            Some(Cow::Owned(StorageFlags::SingleEpoch(0))),
                        )),
                        owner_id: None,
                    },
                    contract,
                    document_type: contract
                        .document_type_for_name("item")
                        .expect("expected the item type"),
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert the item");
    }

    /// The `amount`s, or with `rank` the `rank`s, a query returns, without
    /// and with a proof.
    fn query_items(drive: &Drive, contract: &DataContract, sql: &str, property: &str) -> Vec<u64> {
        let platform_version = PlatformVersion::latest();
        let document_type = contract
            .document_type_for_name("item")
            .expect("expected the item type");
        let query =
            DriveDocumentQuery::from_sql_expr(sql, contract, Some(&drive.config), platform_version)
                .expect("expected the query to parse");
        let (documents, _, _) = query
            .execute_raw_results_no_proof(drive, None, None, platform_version)
            .expect("expected the query to execute");
        let values: Vec<u64> = documents
            .iter()
            .map(|bytes| {
                Document::from_bytes(bytes, document_type, platform_version)
                    .expect("expected a document")
                    .properties()
                    .get(property)
                    .and_then(|value| value.to_integer::<u64>().ok())
                    .expect("expected the property")
            })
            .collect();
        let (proof, _) = query
            .clone()
            .execute_with_proof(drive, None, None, platform_version)
            .expect("expected the query to prove");
        let (_, proved) = query
            .verify_proof(&proof, platform_version)
            .expect("expected the proof to verify");
        let proved: Vec<u64> = proved
            .iter()
            .map(|document| {
                document
                    .properties()
                    .get(property)
                    .and_then(|value| value.to_integer::<u64>().ok())
                    .expect("expected the property")
            })
            .collect();
        assert_eq!(proved, values, "the proof proves the documents");
        values
    }

    #[test]
    fn should_rekey_nested_unique_and_null_layouts_and_swap_colliding_keys() {
        let platform_version = protocol_version_13();
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let contract = item_contract(platform_version);
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply the item contract");
        // 7 and HIGH + 7, and the ranks 3 and 32771, trade keys between the
        // two generations
        for document in [
            item(1, "a", 7, Some(3), "x"),
            item(2, "a", HIGH + 7, Some(32771), "y"),
            item(3, "b", 7, None, "z"),
            item(4, "b", 300, Some(3), "w"),
            item(5, "a", HIGH + 300, None, "v"),
        ] {
            insert_item(&drive, &contract, &document, platform_version);
        }
        let path = document_type_path(&contract, "item");
        let transaction = drive.grove.start_transaction();
        let before = dump(&drive, &path, &transaction);
        drop(transaction);

        rekey(&drive);

        let transaction = drive.grove.start_transaction();
        let after = dump(&drive, &path, &transaction);
        let expected: BTreeMap<_, _> = before
            .into_iter()
            .map(|(path, summary)| (rekeyed_path(&path, &[("amount", 8), ("rank", 2)]), summary))
            .collect();
        assert_eq!(after, expected, "only the amount and rank keys change");
        drop(transaction);

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
        insert_item(
            &drive,
            &contract,
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
}
