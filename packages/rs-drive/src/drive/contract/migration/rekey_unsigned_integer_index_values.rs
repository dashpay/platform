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
use grovedb::batch::QualifiedGroveDbOp;
use grovedb::{PathQuery, Query, QueryItem, SizedQuery, Transaction};
use std::collections::BTreeSet;
use std::ops::Bound;

impl Drive {
    /// Moves every index value key that generation 0 of
    /// `serialize_value_for_key` wrote for a property of an unsigned integer
    /// type to the key generation 1 writes for the same value.
    ///
    /// Runs once, on the first block of protocol version 14: from that
    /// version an unsigned value is keyed by its plain big-endian bytes, which
    /// sort as the values do, where generation 0 flipped the top bit as for a
    /// signed value. Generation 1 differs only by that flip, so running the
    /// rewrite again would flip the keys back; `transition_to_version_14`
    /// calls it, which runs once per chain.
    ///
    /// Every document type of every contract in state is walked through its
    /// layout (`document_type_layout`) down to the trees whose children are
    /// such keys, and each child is moved to its new key with GroveDB's
    /// `move_op`: the subtree under it keeps its stored bytes, so its hashes,
    /// aggregates and Merk shapes are those it had. A key and the same key
    /// with its top bit flipped hold two values that trade keys, and the two
    /// moves go in the same batch. Levels whose keys stay are not rewritten.
    ///
    /// Only the tree holding the moved keys takes another shape. A property
    /// that ends a `rangeSummable` index keeps generation 0's keys
    /// (`serialize_value_for_key` 1) in every index of its type, so no tree
    /// whose nodes keep sums is reordered, and no node sum can leave the
    /// signed 64-bit range.
    ///
    /// Memory: Drive holds at most two pages of keys, and their moves, per
    /// tree level on the way down, and one batch of moves gathered across
    /// trees (`max_rekey_batch_moves`), but GroveDB
    /// copies every record of a moved subtree to its new prefixes, and the
    /// block's transaction holds every write until it commits. So the
    /// activation block's memory grows with the bytes the moved subtrees hold,
    /// which is accepted: it is bounded by the transaction the block writes
    /// anyway, and what earlier protocol versions stored under unsigned index
    /// values is small.
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
        let max_batch_moves =
            platform_version
                .system_limits
                .max_rekey_batch_moves
                .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                    "the protocol version that rewrites unsigned index value keys sets \
                 max_rekey_batch_moves",
                )))? as usize;
        let mut moves = PendingMoves {
            batch: GroveDbOpBatch::new(),
            paths: BTreeSet::new(),
            max_batch_moves,
        };
        let contract_count = self.for_each_contract_in_state(
            |contract_id, fetch_info| {
                for document_type in fetch_info.contract.document_types().values() {
                    self.rekey_unsigned_integer_index_values_of_document_type(
                        contract_id,
                        document_type.as_ref(),
                        &mut moves,
                        transaction,
                        platform_version,
                    )?;
                }
                Ok(())
            },
            transaction,
            platform_version,
        )?;
        self.apply_pending_moves(&mut moves, transaction, platform_version)?;

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
        moves: &mut PendingMoves,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Most types index no property whose keys change: they need no layout
        let indexes_rekeyed_property = document_type
            .indexes()
            .values()
            .flat_map(|index| &index.properties)
            .any(|property| document_type.tree_keys_differ_between_generations(&property.name));
        if !indexes_rekeyed_property {
            return Ok(());
        }

        let layout = document_type_layout(document_type, platform_version)?;
        let document_type_path =
            contract_document_type_path_vec(&contract_id, document_type.name());
        self.rekey_below(
            &document_type_path,
            &layout.root,
            document_type,
            moves,
            transaction,
            platform_version,
        )
    }

    /// Moves the keys that change below the tree at `path`, an instance of
    /// `node`: those deeper down first, under the keys as they are stored,
    /// then those of the tree's own children.
    ///
    /// A tree whose children all have fixed keys moves none of them: only the
    /// children the layout leads down through are read, one by one. Otherwise
    /// the children are read whole when they fit a page, and else in slices:
    /// up to a page of keys whose top bit is clear, and up to a page of keys
    /// whose top bit is set, from where the last slice ended. A slice ends
    /// where the first full page ends, its end mapped onto the other half by
    /// the flip, so every key in it has its twin (the key with the top bit
    /// flipped) in the same slice, and moved keys land in ranges the slices
    /// already passed. Keys of the other page past the end are read again
    /// with the next slice.
    fn rekey_below(
        &self,
        path: &[Vec<u8>],
        node: &LayoutNode,
        document_type: DocumentTypeRef,
        moves: &mut PendingMoves,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if !node
            .children
            .iter()
            .any(|child| rekeys_within(child, document_type))
        {
            return Ok(());
        }
        if node
            .children
            .iter()
            .all(|child| matches!(child.key, LayoutKey::Fixed { .. }))
        {
            for child in &node.children {
                let LayoutKey::Fixed { bytes, .. } = &child.key else {
                    continue;
                };
                if !rekeys_within(child, document_type) {
                    continue;
                }
                let is_tree = self
                    .grove_get_raw_optional(
                        path.into(),
                        bytes,
                        DirectQueryType::StatefulDirectQuery,
                        Some(transaction),
                        &mut vec![],
                        &platform_version.drive,
                    )?
                    .is_some_and(|element| element.is_any_tree());
                if is_tree {
                    let mut child_path = path.to_vec();
                    child_path.push(bytes.clone());
                    self.rekey_below(
                        &child_path,
                        child,
                        document_type,
                        moves,
                        transaction,
                        platform_version,
                    )?;
                }
            }
            return Ok(());
        }

        let page_size = u16::try_from(moves.max_batch_moves)
            .unwrap_or(u16::MAX)
            .max(1);
        let full = |page: &[(Vec<u8>, bool)]| page.len() == usize::from(page_size);
        let children = self.child_keys(
            path,
            QueryItem::RangeFull(..),
            page_size,
            transaction,
            platform_version,
        )?;
        if !full(&children) {
            return self.rekey_slice(
                path,
                node,
                &children,
                document_type,
                moves,
                transaction,
                platform_version,
            );
        }

        // Every key up to here (top bit clear) and its twin are done
        let mut done_through: Option<Vec<u8>> = None;
        loop {
            let low_item = match &done_through {
                Some(key) => QueryItem::RangeAfterTo(key.clone()..TOP_BIT_SET.to_vec()),
                None => QueryItem::RangeTo(..TOP_BIT_SET.to_vec()),
            };
            let high_item = match done_through.as_deref() {
                Some(key) if !key.is_empty() => QueryItem::RangeAfter(flipped(key)..),
                _ => QueryItem::RangeFrom(TOP_BIT_SET.to_vec()..),
            };
            let low = self.child_keys(path, low_item, page_size, transaction, platform_version)?;
            let high =
                self.child_keys(path, high_item, page_size, transaction, platform_version)?;
            let low_end = low
                .last()
                .filter(|_| full(&low))
                .map(|(key, _)| key.clone());
            let high_end = high
                .last()
                .filter(|_| full(&high))
                .map(|(key, _)| flipped(key));
            let end = low_end.into_iter().chain(high_end).min();
            let in_slice = |key: &Vec<u8>, top_bit_set: bool| {
                end.as_ref().is_none_or(|end| {
                    if top_bit_set {
                        flipped(key) <= *end
                    } else {
                        key <= end
                    }
                })
            };
            let slice: Vec<(Vec<u8>, bool)> = low
                .into_iter()
                .filter(|(key, _)| in_slice(key, false))
                .chain(high.into_iter().filter(|(key, _)| in_slice(key, true)))
                .collect();
            self.rekey_slice(
                path,
                node,
                &slice,
                document_type,
                moves,
                transaction,
                platform_version,
            )?;
            match end {
                Some(end) => done_through = Some(end),
                None => break,
            }
        }
        Ok(())
    }

    /// Rewrites the keys below each child in `slice` (children of the tree at
    /// `path`, an instance of `node`), then adds the moves of the children's
    /// own keys to `moves`. A key and its twin trade keys in one batch.
    #[allow(clippy::too_many_arguments)]
    fn rekey_slice(
        &self,
        path: &[Vec<u8>],
        node: &LayoutNode,
        slice: &[(Vec<u8>, bool)],
        document_type: DocumentTypeRef,
        moves: &mut PendingMoves,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for (key, is_tree) in slice {
            let Some(child_node) = child_node(node, key) else {
                continue;
            };
            if *is_tree
                && child_node
                    .children
                    .iter()
                    .any(|below| rekeys_within(below, document_type))
            {
                let mut child_path = path.to_vec();
                child_path.push(key.clone());
                self.rekey_below(
                    &child_path,
                    child_node,
                    document_type,
                    moves,
                    transaction,
                    platform_version,
                )?;
            }
        }

        let present: BTreeSet<&[u8]> = slice.iter().map(|(key, _)| key.as_slice()).collect();
        let mut key_moves = Vec::new();
        for (key, _) in slice {
            let Some(child_node) = child_node(node, key) else {
                continue;
            };
            let Some(new_key) = rekeyed_key(child_node, key, document_type, platform_version)?
            else {
                continue;
            };
            if present.contains(new_key.as_slice()) {
                // The two values trade keys: both moves go in one batch, added
                // when the lower key of the two is reached
                if new_key < *key {
                    continue;
                }
                key_moves.push(vec![
                    QualifiedGroveDbOp::move_op(path.to_vec(), new_key.clone(), key.clone()),
                    QualifiedGroveDbOp::move_op(path.to_vec(), key.clone(), new_key),
                ]);
            } else {
                key_moves.push(vec![QualifiedGroveDbOp::move_op(
                    path.to_vec(),
                    key.clone(),
                    new_key,
                )]);
            }
        }
        self.push_moves(moves, path, key_moves, transaction, platform_version)
    }

    /// Adds `key_moves`, each the move of one key of the tree at `path` or
    /// the two moves of a twin pair, to `moves`, applying the batch whenever
    /// `max_batch_moves` gather, and first when it holds moves below the
    /// tree: a batch refuses to work under a key it moves.
    fn push_moves(
        &self,
        moves: &mut PendingMoves,
        path: &[Vec<u8>],
        key_moves: Vec<Vec<QualifiedGroveDbOp>>,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if key_moves.is_empty() {
            return Ok(());
        }
        // The paths extending `path` sort right after it
        if moves
            .paths
            .range::<[Vec<u8>], _>((Bound::Excluded(path), Bound::Unbounded))
            .next()
            .is_some_and(|pending| pending.starts_with(path))
        {
            self.apply_pending_moves(moves, transaction, platform_version)?;
        }
        for key_move in key_moves {
            if !moves.paths.contains(path) {
                moves.paths.insert(path.to_vec());
            }
            moves.batch.extend(key_move);
            if moves.batch.len() >= moves.max_batch_moves {
                self.apply_pending_moves(moves, transaction, platform_version)?;
            }
        }
        Ok(())
    }

    fn apply_pending_moves(
        &self,
        moves: &mut PendingMoves,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        moves.paths.clear();
        if moves.batch.is_empty() {
            return Ok(());
        }
        self.grove_apply_batch(
            std::mem::take(&mut moves.batch),
            false,
            Some(transaction),
            &platform_version.drive,
        )
    }

    /// The keys of up to `limit` children of the tree at `path` in `item`,
    /// in key order, and whether each is a tree.
    fn child_keys(
        &self,
        path: &[Vec<u8>],
        item: QueryItem,
        limit: u16,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<(Vec<u8>, bool)>, Error> {
        let path_query = PathQuery::new(
            path.to_vec(),
            SizedQuery::new(Query::new_single_query_item(item), Some(limit), None),
        );
        let (page, _) = self.grove_get_raw_path_query(
            &path_query,
            Some(transaction),
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;
        Ok(page
            .to_key_elements()
            .into_iter()
            .map(|(key, element)| (key, element.is_any_tree()))
            .collect())
    }
}

/// The moves gathered so far, across trees, document types and contracts, and
/// the paths of the trees they are in.
struct PendingMoves {
    batch: GroveDbOpBatch,
    paths: BTreeSet<Vec<Vec<u8>>>,
    max_batch_moves: usize,
}

/// The smallest key whose top bit is set: every key below it has the top
/// bit clear (the empty key included), every key from it on has it set.
const TOP_BIT_SET: [u8; 1] = [0b1000_0000];

/// `key` with the top bit of its first byte flipped: the key its value takes
/// in the other generation, when it is an unsigned value's key. Empty for the
/// empty key.
fn flipped(key: &[u8]) -> Vec<u8> {
    let mut key = key.to_vec();
    if let Some(top) = key.first_mut() {
        *top ^= 0b1000_0000;
    }
    key
}

/// The layout node a child of an instance of `node` keyed `key` is an
/// instance of: the fixed child with that key, or else the family of keys
/// the node holds. `None` for a key the layout does not describe, which
/// stays where it is.
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

/// Whether any key of `node`'s family or below it changes: the value keys of
/// a property generation 1 keys as an unsigned integer.
fn rekeys_within(node: &LayoutNode, document_type: DocumentTypeRef) -> bool {
    matches!(
        &node.key,
        LayoutKey::PropertyValue { property }
            if document_type.tree_keys_differ_between_generations(property)
    ) || node
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DriveConfig;
    use crate::drive::votes::paths::VotePollPaths;
    use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
    use crate::query::contested_resource_votes_given_by_identity_query::ContestedResourceVotesGivenByIdentityQuery;
    use crate::query::unsigned_index_key_order_tests::{
        above_100, count_grades, insert_document, insert_grade, query_grades, query_values,
        setup_grades,
    };
    use crate::query::vote_poll_vote_state_query::{
        ContestedDocumentVotePollDriveQuery, ContestedDocumentVotePollDriveQueryResultType,
        FinalizedContestedDocumentVotePollDriveQueryExecutionResult,
    };
    use crate::query::vote_polls_by_document_type_query::VotePollsByDocumentTypeQuery;
    use crate::query::vote_polls_by_end_date_query::VotePollsByEndDateDriveQuery;
    use crate::util::object_size_info::DocumentInfo::{DocumentAndSerialization, DocumentRefInfo};
    use crate::util::object_size_info::{
        DataContractOwnedResolvedInfo, DocumentAndContractInfo, OwnedDocumentInfo,
    };
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::{setup_drive, setup_drive_with_initial_state_structure};
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Setters;
    use dpp::data_contract::{DataContract, DataContractFactory};
    use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
    use dpp::document::{Document, DocumentV0, DocumentV0Setters};
    use dpp::identifier::Identifier;
    use dpp::platform_value::string_encoding::Encoding;
    use dpp::platform_value::{platform_value, Value};
    use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
    use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
    use dpp::voting::vote_polls::VotePoll;
    use dpp::voting::votes::resource_vote::accessors::v0::ResourceVoteGettersV0;
    use dpp::voting::votes::resource_vote::ResourceVote;
    use grovedb::{AggregateData, Element};
    use grovedb_merk::element::reconstruct::ElementReconstructExtensions;
    use std::borrow::Cow;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    const HIGH: u64 = 1 << 63;

    fn protocol_version_13() -> &'static PlatformVersion {
        PlatformVersion::get(13).expect("expected protocol version 13")
    }

    /// Every child of the tree at `path`, as it is stored. A query returns a
    /// reference with its path made absolute, so a reference is read again on
    /// its own.
    fn stored_children(
        drive: &Drive,
        path: &[Vec<u8>],
        transaction: &Transaction,
    ) -> Vec<(Vec<u8>, Element)> {
        let platform_version = PlatformVersion::latest();
        let path_query = PathQuery::new(
            path.to_vec(),
            SizedQuery::new(Query::new_range_full(), None, None),
        );
        let (children, _) = drive
            .grove_get_raw_path_query(
                &path_query,
                Some(transaction),
                QueryResultType::QueryKeyElementPairResultType,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the children");
        children
            .to_key_elements()
            .into_iter()
            .map(|(key, element)| {
                if !element.is_reference() {
                    return (key, element);
                }
                let stored = drive
                    .grove_get_raw(
                        path.into(),
                        &key,
                        DirectQueryType::StatefulDirectQuery,
                        Some(transaction),
                        &mut vec![],
                        &platform_version.drive,
                    )
                    .expect("expected the reference")
                    .expect("a stored reference");
                (key, stored)
            })
            .collect()
    }

    /// Every element under `path`, keyed by its path below it, as it is
    /// stored: a tree as its type, flags and wrapper, with the aggregates of
    /// the tree inside any wrapper.
    fn dump(
        drive: &Drive,
        path: &[Vec<u8>],
        transaction: &Transaction,
    ) -> BTreeMap<Vec<Vec<u8>>, (Element, u64, i64)> {
        let mut out = BTreeMap::new();
        for (key, element) in stored_children(drive, path, transaction) {
            let summary = if element.is_any_tree() {
                (
                    element
                        .reconstruct_with_root_key(None, AggregateData::NoAggregateData)
                        .expect("a Merk tree"),
                    element.underlying().count_value_or_default(),
                    element.underlying().sum_value_or_default(),
                )
            } else {
                (element, 0, 0)
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

    /// A moved value tree keeps the subtree under it as it was stored: the
    /// tree of its documents keeps its root key and count, so its Merk tree
    /// keeps its shape and hashes.
    #[test]
    fn should_keep_the_merk_tree_under_a_moved_key() {
        let (drive, contract) = setup_grades(protocol_version_13());
        for id in 20..40 {
            insert_grade(&drive, &contract, id, 200, protocol_version_13());
        }
        let documents_of = |key: u8| {
            let mut path = contract_document_type_path_vec(contract.id().as_bytes(), "grade");
            path.extend([b"grade".to_vec(), vec![key]]);
            drive
                .grove_get_raw(
                    path.as_slice().into(),
                    &[0],
                    DirectQueryType::StatefulDirectQuery,
                    None,
                    &mut vec![],
                    &PlatformVersion::latest().drive,
                )
                .expect("expected to read")
                .expect("expected the documents of grade 200")
        };
        let before = documents_of(0x48);

        rekey(&drive);

        assert_eq!(documents_of(0xC8), before);
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

    /// A `score` type summing `weight` by a u8 `grade`, both on its own
    /// (`byGrade`, range-summable when `range_summable`) and under a
    /// range-summable `[grade, tag]` continuation, which a summing value tree
    /// holds behind a wrapper that keeps it out of the sum.
    fn score_contract(weight: Value, range_summable: bool) -> DataContract {
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
                        "rangeSummable": range_summable,
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
        let contract = score_contract(
            platform_value!({"type": "integer", "minimum": -1000, "maximum": 1000, "position": 1}),
            false,
        );
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

    /// A summed property an update before protocol version 14 made unsigned
    /// (removing its `minimum`) while the range-summable index held negative
    /// values of it: the grades keep their earlier keys. Moved, grade 2 would
    /// sit over grade 1 in value order, a node adding two values of
    /// -(2^62 + 1), and the activation block would fail.
    #[test]
    fn should_keep_the_keys_of_an_index_range_summing_values_an_update_made_unsigned() {
        let signed = score_contract(
            platform_value!({
                "type": "integer",
                "minimum": -5_000_000_000_000_000_000i64,
                "maximum": 100,
                "position": 1
            }),
            true,
        );
        let drive = setup_contract_at_13(&signed);
        let low = -(1i64 << 62) - 1;
        for (id, grade, weight) in [(1, 2, low), (2, 200, 2), (3, 1, low)] {
            insert_document(
                &drive,
                &signed,
                "score",
                &score(id, grade, weight, "a"),
                protocol_version_13(),
            );
        }
        let mut unsigned = score_contract(
            platform_value!({"type": "integer", "maximum": 100, "position": 1}),
            true,
        );
        unsigned.set_version(2);
        assert!(unsigned
            .document_type_for_name("score")
            .expect("expected the score type")
            .flattened_properties()
            .get("weight")
            .expect("expected the weight")
            .property_type
            .has_unsigned_integer_tree_key());
        drive
            .apply_contract(
                &unsigned,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                protocol_version_13(),
            )
            .expect("expected to update the contract");
        let before = dump_document_type(&drive, &unsigned, "score");

        rekey(&drive);

        assert_eq!(
            dump_document_type(&drive, &unsigned, "score"),
            before,
            "the grades keep their earlier keys"
        );
    }

    /// A range-summable index keeps its earlier keys: these sums of both
    /// signs fit in the old order, but every balanced tree of the new order
    /// holds a node summing past `i64::MAX`. Protocol version 14 keys its
    /// grades as generation 0 too, so reads and writes find them.
    #[test]
    fn should_keep_and_use_the_keys_of_an_index_range_summing_signed_values() {
        const M: i64 = 1 << 62;
        let contract = score_contract(platform_value!({"type": "integer", "position": 1}), true);
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

        let platform_version = PlatformVersion::latest();
        let grades =
            |sql: &str| query_values(&drive, &contract, "score", sql, "grade", platform_version);
        assert_eq!(grades("select * from score where grade == 128"), vec![128]);
        assert_eq!(grades("select * from score where grade == 1"), vec![1]);

        drive
            .delete_document_for_contract(
                Identifier::new([1; 32]),
                &contract,
                "score",
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to delete a document written at protocol version 13");
        insert_document(
            &drive,
            &contract,
            "score",
            &score(5, 129, 0, "a"),
            platform_version,
        );
        assert_eq!(
            grades("select * from score where grade == 1"),
            Vec::<u64>::new()
        );
        assert_eq!(grades("select * from score where grade == 129"), vec![129]);
    }

    /// Moves applied one batch at a time still keep each pair of keys that
    /// trade places in one batch, and leave the records and aggregates
    /// batches of the usual size leave (the dumps leave out the Merk shapes,
    /// which batch boundaries can change).
    #[test]
    fn should_rekey_the_same_in_batches_of_one_move() {
        let (drive, contract) = setup_grades(protocol_version_13());
        let (reference, _) = setup_grades(protocol_version_13());
        rekey(&reference);

        let mut platform_version = PlatformVersion::latest().clone();
        platform_version.system_limits.max_rekey_batch_moves = Some(1);
        let transaction = drive.grove.start_transaction();
        drive
            .rekey_unsigned_integer_index_values(&transaction, &platform_version)
            .expect("expected the rekey to run one move at a time");
        drive
            .grove
            .commit_transaction(transaction)
            .unwrap()
            .expect("expected to commit");

        assert_eq!(
            dump_document_type(&drive, &contract, "grade"),
            dump_document_type(&reference, &contract, "grade"),
        );
        assert_eq!(
            count_grades(&drive, &contract, above_100(), PlatformVersion::latest())
                .expect("expected the count"),
            5
        );
    }

    /// A `pair` type: a u8 `a` and a u16 `b`, indexed together (`[a, b]`,
    /// two levels whose keys change) and `b` alone in a unique index, whose
    /// value trees hold the reference to the document directly.
    fn pair_contract() -> DataContract {
        contract_at_13(
            12,
            "pair",
            platform_value!({
                "type": "object",
                "properties": {
                    "a": {"type": "integer", "minimum": 0, "maximum": 255, "position": 0},
                    "b": {"type": "integer", "minimum": 0, "maximum": 65535, "position": 1},
                },
                "required": ["a", "b"],
                "indices": [
                    {
                        "name": "byAB",
                        "properties": [{"a": "asc"}, {"b": "asc"}],
                        "countable": "countable",
                    },
                    {"name": "byB", "properties": [{"b": "asc"}], "unique": true},
                ],
                "additionalProperties": false,
            }),
        )
    }

    /// A drive at protocol version 13 holding [`pair_contract`] and pairs on
    /// both sides of the top bit of each property, many of them twins.
    fn setup_pairs(contract: &DataContract) -> Drive {
        let drive = setup_contract_at_13(contract);
        for (id, a, b) in [
            (1, 1, 2),
            (2, 129, 32770),
            (3, 2, 7),
            (4, 130, 32775),
            (5, 200, 300),
            (6, 72, 33068),
            (7, 60, 9),
            (8, 1, 40000),
            (9, 1, 7232),
            (10, 129, 5),
            (11, 129, 32773),
            (12, 255, 65535),
            (13, 127, 32767),
        ] {
            insert_document(
                &drive,
                contract,
                "pair",
                &document(id, vec![("a", Value::U8(a)), ("b", Value::U16(b))]),
                protocol_version_13(),
            );
        }
        drive
    }

    /// Two levels whose keys change, read in slices of a few keys and moved
    /// in batches of a few moves, so moves wait across slices and trees and a
    /// level's moves wait for the levels below; and the references a unique
    /// index holds directly under the moved value trees.
    #[test]
    fn should_rekey_two_levels_and_unique_references_in_slices_and_small_batches() {
        let contract = pair_contract();
        let reference = setup_pairs(&contract);
        let before = dump_document_type(&reference, &contract, "pair");
        rekey(&reference);
        let expected = rekeyed_dump(before, &[("a", 1), ("b", 2)]);
        assert_eq!(dump_document_type(&reference, &contract, "pair"), expected);

        for max_rekey_batch_moves in [2, 3] {
            let drive = setup_pairs(&contract);
            let mut platform_version = PlatformVersion::latest().clone();
            platform_version.system_limits.max_rekey_batch_moves = Some(max_rekey_batch_moves);
            let transaction = drive.grove.start_transaction();
            drive
                .rekey_unsigned_integer_index_values(&transaction, &platform_version)
                .expect("expected the rekey to run in small batches");
            drive
                .grove
                .commit_transaction(transaction)
                .unwrap()
                .expect("expected to commit");
            assert_eq!(
                dump_document_type(&drive, &contract, "pair"),
                expected,
                "batches of {max_rekey_batch_moves}"
            );

            let values = |sql: &str, property: &str| {
                query_values(
                    &drive,
                    &contract,
                    "pair",
                    sql,
                    property,
                    PlatformVersion::latest(),
                )
            };
            assert_eq!(
                values("select * from pair where a > 100 order by a asc", "a"),
                vec![127, 129, 129, 129, 130, 200, 255]
            );
            assert_eq!(
                values(
                    "select * from pair where a == 1 and b > 100 order by b asc",
                    "b"
                ),
                vec![7232, 40000]
            );
            assert_eq!(
                values("select * from pair where b > 32768 order by b asc", "b"),
                vec![32770, 32773, 32775, 33068, 40000, 65535]
            );
        }
    }

    /// A token history shaped type: indexes led by an identifier, one of them
    /// continuing with an unsigned `amount`. Only the amount keys move; the
    /// sibling branches under each token stay as they are.
    #[test]
    fn should_move_only_the_levels_whose_keys_change() {
        let contract = contract_at_13(
            11,
            "transfer",
            platform_value!({
                "type": "object",
                "documentsMutable": false,
                "properties": {
                    "tokenId": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 0,
                    },
                    "amount": {"type": "integer", "minimum": 0, "position": 1},
                    "recipient": {
                        "type": "array",
                        "byteArray": true,
                        "minItems": 32,
                        "maxItems": 32,
                        "contentMediaType": "application/x.dash.dpp.identifier",
                        "position": 2,
                    },
                },
                "required": ["tokenId", "amount", "recipient", "$createdAt"],
                "indices": [
                    {"name": "byDate", "properties": [{"tokenId": "asc"}, {"$createdAt": "asc"}]},
                    {"name": "byAmount", "properties": [{"tokenId": "asc"}, {"amount": "asc"}]},
                    {
                        "name": "to",
                        "properties": [{"tokenId": "asc"}, {"recipient": "asc"}, {"$createdAt": "asc"}],
                    },
                ],
                "additionalProperties": false,
            }),
        );
        let drive = setup_contract_at_13(&contract);
        for (id, token, amount) in [(1, 1, 5u64), (2, 1, HIGH + 5), (3, 2, 70), (4, 2, 5)] {
            let mut transfer = document(
                id,
                vec![
                    ("tokenId", Value::Identifier([token; 32])),
                    ("amount", Value::U64(amount)),
                    ("recipient", Value::Identifier([id + 20; 32])),
                ],
            );
            transfer.set_created_at(Some(1_000 + u64::from(id)));
            insert_document(
                &drive,
                &contract,
                "transfer",
                &transfer,
                protocol_version_13(),
            );
        }
        let before = dump_document_type(&drive, &contract, "transfer");

        rekey(&drive);

        assert_eq!(
            dump_document_type(&drive, &contract, "transfer"),
            rekeyed_dump(before, &[("amount", 8)]),
            "only the amount keys change"
        );
        assert_eq!(
            query_values(
                &drive,
                &contract,
                "transfer",
                &format!(
                    "select * from transfer where tokenId == '{}' and amount > 4 order by amount asc",
                    Identifier::new([1; 32]).to_string(Encoding::Base58)
                ),
                "amount",
                PlatformVersion::latest(),
            ),
            vec![5, HIGH + 5]
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

        // The contested values listing and its proof, edited in place to key
        // the poll values with generation 0, read the same at protocol version 13
        let listing_at_13 = VotePollsByDocumentTypeQuery {
            contract_id: contract.id(),
            document_type_name: "seat".to_string(),
            index_name: "byNumber".to_string(),
            start_index_values: vec![],
            end_index_values: vec![],
            start_at_value: None,
            limit: None,
            order_ascending: true,
        };
        assert_eq!(
            listing_at_13
                .execute_no_proof(&drive, None, &mut vec![], protocol_version_13())
                .expect("expected the contested values at protocol version 13"),
            vec![Value::U8(200)]
        );
        let proof_at_13 = listing_at_13
            .clone()
            .execute_with_proof(&drive, None, &mut vec![], protocol_version_13())
            .expect("expected the contested values proof at protocol version 13");
        let (_, proved_at_13) = listing_at_13
            .resolve_with_provided_borrowed_contract(&contract)
            .expect("expected to resolve the listing")
            .verify_contests_proof(&proof_at_13, protocol_version_13())
            .expect("expected the proof to verify at protocol version 13");
        assert_eq!(proved_at_13, vec![Value::U8(200)]);

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
        let contenders: Vec<Vec<u8>> = stored_children(&drive, &contenders_path, &transaction)
            .into_iter()
            .map(|(key, _)| key)
            .collect();
        assert!(contenders.contains(&vec![21; 32]), "the first contender");
        assert!(contenders.contains(&vec![22; 32]), "the second contender");
        let voting_path = vote_poll
            .contender_voting_path(&towards_first, platform_version)
            .expect("expected the voting path");
        let voters: Vec<Vec<u8>> = stored_children(&drive, &voting_path, &transaction)
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
    /// A contest on an unsigned value started, and voted on, before protocol
    /// version 14 ends after it: the poll is found by its end date, its tally
    /// names the winner, whose document is awarded into the index under the
    /// value's new key, and the cleanup removes the end date, the contenders
    /// and the vote at the paths they have had since the contest started.
    #[test]
    fn should_award_and_clean_up_a_contest_on_an_unsigned_value_started_before_the_upgrade() {
        let contract = seat_contract();
        // Configured as a shipped node is: the end of a poll applies a batch
        // that the test harness's consistency check refuses at any version
        let drive = setup_drive(Some(DriveConfig {
            batching_consistency_verification: false,
            ..Default::default()
        }));
        drive
            .create_initial_state_structure(None, protocol_version_13())
            .expect("expected the initial state");
        drive
            .apply_contract(
                &contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                protocol_version_13(),
            )
            .expect("expected to apply the contract");
        let vote_poll = ContestedDocumentResourceVotePollWithContractInfo {
            contract: DataContractOwnedResolvedInfo::OwnedDataContract(contract.clone()),
            document_type_name: "seat".to_string(),
            index_name: "byNumber".to_string(),
            index_values: vec![Value::U8(200)],
        };
        add_seat_contender(&drive, &vote_poll, 21, true, protocol_version_13());
        add_seat_contender(&drive, &vote_poll, 22, false, protocol_version_13());
        let voter = Identifier::new([9; 32]);
        let towards_first = ResourceVoteChoice::TowardsIdentity(Identifier::new([21; 32]));
        drive
            .register_contested_resource_identity_vote(
                voter.to_buffer(),
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
        let ended = VotePollsByEndDateDriveQuery {
            start_time: None,
            end_time: None,
            limit: None,
            offset: None,
            order_ascending: true,
        }
        .execute_no_proof(&drive, None, &mut vec![], platform_version)
        .expect("expected the polls by end date");
        let [(end_date, polls)] = ended
            .into_iter()
            .collect::<Vec<_>>()
            .try_into()
            .expect("expected one end date");
        let [VotePoll::ContestedDocumentResourceVotePoll(poll)]: [VotePoll; 1] =
            polls.try_into().expect("expected one poll");
        assert_eq!(poll.index_values, vec![Value::U8(200)]);

        let tally: FinalizedContestedDocumentVotePollDriveQueryExecutionResult =
            ContestedDocumentVotePollDriveQuery {
                vote_poll: poll,
                result_type: ContestedDocumentVotePollDriveQueryResultType::DocumentsAndVoteTally,
                offset: None,
                limit: Some(10),
                start_at: None,
                allow_include_locked_and_abstaining_vote_tally: true,
            }
            .execute_no_proof(&drive, None, &mut vec![], platform_version)
            .expect("expected the tally")
            .try_into()
            .expect("expected a finalized tally");
        let winner = tally
            .contenders
            .iter()
            .max_by_key(|contender| contender.final_vote_tally)
            .expect("expected contenders");
        assert_eq!(winner.identity_id, Identifier::new([21; 32]));
        let document_type = contract
            .document_type_for_name("seat")
            .expect("expected the seat type");
        let document =
            Document::from_bytes(&winner.serialized_document, document_type, platform_version)
                .expect("expected the winner's document");
        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentAndSerialization((
                            document,
                            winner.serialized_document.clone(),
                            None,
                        )),
                        owner_id: Some(winner.identity_id.to_buffer()),
                    },
                    contract: &contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to award the document");

        // The voters of each choice, as the end of a poll gathers them
        let mut voters = drive
            .fetch_identities_voting_for_contenders(
                &vote_poll,
                tally
                    .contenders
                    .iter()
                    .map(|contender| contender.identity_id)
                    .collect(),
                true,
                None,
                platform_version,
            )
            .expect("expected the voters");
        for contender in &tally.contenders {
            if contender.final_vote_tally == 0 {
                voters.insert(
                    ResourceVoteChoice::TowardsIdentity(contender.identity_id),
                    vec![],
                );
            }
        }
        if tally.locked_vote_tally == 0 {
            voters.insert(ResourceVoteChoice::Lock, vec![]);
        }
        if tally.abstaining_vote_tally == 0 {
            voters.insert(ResourceVoteChoice::Abstain, vec![]);
        }
        assert_eq!(voters.get(&towards_first), Some(&vec![voter]));
        let ended_polls = [(&vote_poll, &end_date, &voters)];
        let mut operations = vec![];
        drive
            .remove_contested_resource_vote_poll_end_date_query_operations(
                &ended_polls,
                &mut operations,
                None,
                platform_version,
            )
            .expect("expected to remove the end date");
        drive
            .remove_contested_resource_vote_poll_votes_operations(
                &ended_polls,
                true,
                &mut operations,
                None,
                platform_version,
            )
            .expect("expected to remove the votes");
        drive
            .remove_contested_resource_vote_poll_documents_operations(
                &ended_polls,
                false,
                &mut operations,
                None,
                platform_version,
            )
            .expect("expected to remove the contenders' documents");
        drive
            .remove_contested_resource_vote_poll_contenders_operations(
                &ended_polls,
                &mut operations,
                None,
                platform_version,
            )
            .expect("expected to remove the contenders");
        let vote_id = vote_poll.unique_id().expect("expected the poll's id");
        drive
            .remove_specific_vote_references_given_by_identity(
                &voter,
                &[&vote_id],
                &mut operations,
                None,
                platform_version,
            )
            .expect("expected to remove the voter's record");
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected the cleanup to apply");

        // The awarded seat is where protocol version 14 looks for it
        assert_eq!(
            query_values(
                &drive,
                &contract,
                "seat",
                "select * from seat where number > 100 order by number asc",
                "number",
                platform_version,
            ),
            vec![200]
        );
        let transaction = drive.grove.start_transaction();
        let numbers: Vec<Vec<u8>> = stored_children(
            &drive,
            &[
                contract_document_type_path_vec(contract.id().as_bytes(), "seat"),
                vec![b"number".to_vec()],
            ]
            .concat(),
            &transaction,
        )
        .into_iter()
        .map(|(key, _)| key)
        .collect();
        assert_eq!(numbers, vec![vec![0xC8]]);

        // Nothing of the contest is left
        let contenders: Vec<Vec<u8>> = stored_children(
            &drive,
            &vote_poll
                .contenders_path(platform_version)
                .expect("expected the contenders path"),
            &transaction,
        )
        .into_iter()
        .map(|(key, _)| key)
        .collect();
        assert!(!contenders.contains(&vec![21; 32]));
        assert!(!contenders.contains(&vec![22; 32]));
        drop(transaction);
        assert!(VotePollsByEndDateDriveQuery {
            start_time: None,
            end_time: None,
            limit: None,
            offset: None,
            order_ascending: true,
        }
        .execute_no_proof(&drive, None, &mut vec![], platform_version)
        .expect("expected the polls by end date")
        .is_empty());
        let votes_given = ContestedResourceVotesGivenByIdentityQuery {
            identity_id: voter,
            offset: None,
            limit: None,
            start_at: None,
            order_ascending: true,
        }
        .execute_no_proof(&drive, None, &mut vec![], platform_version)
        .expect("expected the votes given");
        assert!(votes_given.is_empty(), "the voter's record is removed");
    }
}
