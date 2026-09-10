//! Activation migration of persisted document histories.
//!
//! Any error deliberately halts the activation block for every validator. All
//! batches belong to the activation transaction; no partial migration may commit.
//! Every `corrupt()` path is unreachable for valid pre-14 state: legacy writers
//! and contract-update validation preserve the storage invariants checked here.

use crate::drive::document::paths::{contract_document_type_path_vec, DOCUMENT_HISTORY_TREE_KEY};
use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};
use crate::fees::op::LowLevelDriveOperation;
use crate::query::QueryResultType;
use crate::util::common::encode::encode_u64;
use crate::util::storage_flags::StorageFlags;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentPropertyType;
use dpp::document::serialization_traits::DocumentPlatformConversionMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::version::PlatformVersion;
use grovedb::batch::{QualifiedGroveDbOp, SubelementsDeletionBehavior};
use grovedb::reference_path::ReferencePathType::{SiblingReference, UpstreamRootHeightReference};
use grovedb::{Element, PathQuery, Query, SizedQuery, Transaction, TreeType};
use grovedb_costs::OperationCost;
use std::collections::BTreeMap;

/// Inventory and measured GroveDB work for a history-layout migration.
#[derive(Debug, Default)]
pub struct DocumentHistoryMigrationStats {
    /// All contracts inspected, including contracts with no documents.
    pub contracts: u64,
    /// History-keeping document types, including empty types.
    pub types: u64,
    /// Current documents inspected.
    pub documents: u64,
    /// Retained revision items inspected.
    pub revisions: u64,
    /// Serialized revision payload bytes.
    pub revision_bytes: u64,
    /// Maximum retained revisions in one document.
    pub maximum_document_revisions: u64,
    /// Document index references inspected.
    pub index_entries: u64,
    /// Index references successfully rewritten.
    pub rewritten_index_entries: u64,
    /// Largest index-reference count retained for one document type.
    pub maximum_type_index_entries: u64,
    /// Conservative owned-buffer bound for one type index inventory, excluding allocator overhead.
    pub maximum_type_index_buffer_bound: u64,
    /// Documents changed by this invocation.
    pub migrated_documents: u64,
    /// Applied batches, excluding reads.
    pub batches: u64,
    /// Read and write costs reported by GroveDB.
    pub cost: OperationCost,
}

type IndexReference = (Vec<Vec<u8>>, Vec<u8>, Element);
type IndexEntries = BTreeMap<Vec<u8>, Vec<IndexReference>>;

#[cfg(test)]
mod index_tests;
#[cfg(test)]
mod tests;

fn corrupt(message: impl Into<String>) -> Error {
    Error::Drive(DriveError::CorruptedDriveState(message.into()))
}

fn index_inventory_buffer_bound(entries: &IndexEntries) -> u64 {
    // A B-tree node stores at most eleven key/value pairs and twelve child edges.
    // Reserving two KiB per key plus the root bounds its inline node storage.
    let mut bytes = 2048 * (entries.len() as u64 + 1);
    for (id, references) in entries {
        bytes += id.capacity() as u64;
        bytes += (references.capacity() * std::mem::size_of::<(Vec<Vec<u8>>, Vec<u8>, Element)>())
            as u64;
        for (path, key, element) in references {
            bytes += (path.capacity() * std::mem::size_of::<Vec<u8>>()) as u64;
            bytes += path.iter().map(|part| part.capacity() as u64).sum::<u64>();
            bytes += key.capacity() as u64;
            if let Element::Reference(UpstreamRootHeightReference(_, target), _, flags)
            | Element::ReferenceWithSumItem(
                UpstreamRootHeightReference(_, target),
                _,
                _,
                flags,
            ) = element
            {
                bytes += (target.capacity() * std::mem::size_of::<Vec<u8>>()) as u64;
                bytes += target
                    .iter()
                    .map(|part| part.capacity() as u64)
                    .sum::<u64>();
                bytes += flags.as_ref().map_or(0, |flags| flags.capacity() as u64);
            }
        }
    }
    bytes
}

impl Drive {
    /// Moves timestamp-keyed document histories out of primary storage.
    /// The caller owns the transaction and must roll it back on any error.
    pub fn migrate_document_history_storage(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentHistoryMigrationStats, Error> {
        if platform_version
            .drive
            .methods
            .document
            .insert
            .add_document_to_primary_storage
            != 1
        {
            return Err(corrupt(
                "document history migration requires the composite-key layout",
            ));
        }
        let mut stats = DocumentHistoryMigrationStats::default();
        let mut cursor: Option<[u8; 32]> = None;
        loop {
            let mut operations = vec![];
            let ids = self.fetch_contract_ids_with_operations(
                cursor.map(|id| (id, false)),
                u16::MAX,
                Some(transaction),
                &mut operations,
                platform_version,
            )?;
            for operation in operations {
                if let LowLevelDriveOperation::CalculatedCostOperation(cost) = operation {
                    stats.cost += cost;
                }
            }
            if ids.is_empty() {
                break;
            }
            cursor = ids.last().copied();
            for id in ids {
                stats.contracts += 1;
                let fetched =
                    self.fetch_contract(id, None, None, Some(transaction), platform_version);
                stats.cost += fetched.cost;
                let fetched = fetched
                    .value?
                    .ok_or_else(|| corrupt("enumerated contract is missing"))?;
                let contract_flags =
                    StorageFlags::map_to_some_element_flags(fetched.storage_flags.as_ref());
                for (name, document_type) in fetched.contract.document_types() {
                    if !document_type.as_ref().documents_keep_history() {
                        continue;
                    }
                    stats.types += 1;
                    let type_path = contract_document_type_path_vec(&id, name);
                    let type_entries = self.history_migration_entries(
                        &type_path,
                        transaction,
                        platform_version,
                        &mut stats,
                    )?;
                    let mut index_entries = BTreeMap::new();
                    for (key, element) in &type_entries {
                        match key.as_slice() {
                            [0] | [DOCUMENT_HISTORY_TREE_KEY] if element.is_any_tree() => {}
                            [0] | [1] | [DOCUMENT_HISTORY_TREE_KEY] => {
                                return Err(corrupt("unexpected reserved document type child"))
                            }
                            _ => {
                                if !element.is_any_tree()
                                    || !document_type
                                        .as_ref()
                                        .index_structure()
                                        .sub_levels()
                                        .keys()
                                        .any(|name| name.as_bytes() == key)
                                {
                                    return Err(corrupt(
                                        "unrecognised document type child during history inventory",
                                    ));
                                }
                                let mut path = type_path.clone();
                                path.push(key.clone());
                                self.history_migration_index_entries(
                                    path,
                                    transaction,
                                    platform_version,
                                    &mut stats,
                                    &mut index_entries,
                                )?;
                            }
                        }
                    }
                    stats.maximum_type_index_entries = stats.maximum_type_index_entries.max(
                        index_entries
                            .values()
                            .map(|entries| entries.len() as u64)
                            .sum(),
                    );
                    stats.maximum_type_index_buffer_bound = stats
                        .maximum_type_index_buffer_bound
                        .max(index_inventory_buffer_bound(&index_entries));
                    if !type_entries
                        .iter()
                        .any(|(key, _)| key == &[DOCUMENT_HISTORY_TREE_KEY])
                    {
                        self.history_migration_batch(
                            vec![QualifiedGroveDbOp::insert_or_replace_op(
                                type_path.clone(),
                                vec![DOCUMENT_HISTORY_TREE_KEY],
                                Element::empty_tree_with_flags(contract_flags.clone()),
                            )],
                            transaction,
                            platform_version,
                            &mut stats,
                        )?;
                    }
                    let mut primary_path = type_path.clone();
                    primary_path.push(vec![0]);
                    for (document_id, element) in self.history_migration_entries(
                        &primary_path,
                        transaction,
                        platform_version,
                        &mut stats,
                    )? {
                        stats.documents += 1;
                        let mut history_path = type_path.clone();
                        history_path.extend([vec![DOCUMENT_HISTORY_TREE_KEY], document_id.clone()]);
                        if matches!(
                            element,
                            Element::Reference(..) | Element::ReferenceWithSumItem(..)
                        ) {
                            let entries = self.history_migration_entries(
                                &history_path,
                                transaction,
                                platform_version,
                                &mut stats,
                            )?;
                            Self::history_migration_count_revisions(&entries, &mut stats)?;
                            index_entries.remove(&document_id);
                            continue;
                        }
                        let old_tree_type = match element {
                            Element::Tree(..) => TreeType::NormalTree,
                            Element::SumTree(..) => TreeType::SumTree,
                            _ => {
                                return Err(corrupt(
                                    "unexpected primary entry in historical document type",
                                ))
                            }
                        };
                        let mut old_path = primary_path.clone();
                        old_path.push(document_id.clone());
                        let entries = self.history_migration_entries(
                            &old_path,
                            transaction,
                            platform_version,
                            &mut stats,
                        )?;
                        let mut pointer = None;
                        let mut revisions = BTreeMap::new();
                        let mut copies = vec![QualifiedGroveDbOp::insert_or_replace_op(
                            history_path[..history_path.len() - 1].to_vec(),
                            document_id.clone(),
                            Element::empty_provable_count_tree_with_flags(contract_flags.clone()),
                        )];
                        for (key, element) in &entries {
                            if key == &[0] {
                                pointer = Some(element.clone());
                                continue;
                            }
                            if key.len() != 8 {
                                return Err(corrupt("legacy history key must be eight bytes"));
                            }
                            let Element::Item(bytes, _) = element else {
                                return Err(corrupt("legacy history revision must be an item"));
                            };
                            let document = Document::from_bytes(
                                bytes,
                                document_type.as_ref(),
                                platform_version,
                            )?;
                            if document.id().as_slice() != document_id {
                                return Err(corrupt(
                                    "history document id does not match its primary key",
                                ));
                            }
                            let time = DocumentPropertyType::decode_date_timestamp(key)
                                .ok_or_else(|| corrupt("invalid history timestamp"))?;
                            let mut composite = encode_u64(time);
                            composite.extend(encode_u64(document.revision().unwrap_or(1)));
                            revisions.insert(key.clone(), composite.clone());
                            copies.push(QualifiedGroveDbOp::insert_or_replace_op(
                                history_path.clone(),
                                composite,
                                element.clone(),
                            ));
                        }
                        let revision_entries = entries
                            .iter()
                            .filter(|(key, _)| key != &[0])
                            .cloned()
                            .collect::<Vec<_>>();
                        Self::history_migration_count_revisions(&revision_entries, &mut stats)?;
                        let pointer = Self::history_migration_pointer(
                            pointer.ok_or_else(|| corrupt("history has no current pointer"))?,
                            &document_id,
                            &revisions,
                        )?;
                        self.history_migration_batch(
                            copies,
                            transaction,
                            platform_version,
                            &mut stats,
                        )?;
                        let mut deletes = entries
                            .into_iter()
                            .map(|(key, _)| QualifiedGroveDbOp::delete_op(old_path.clone(), key))
                            .collect::<Vec<_>>();
                        deletes.push(QualifiedGroveDbOp::delete_tree_op(
                            primary_path.clone(),
                            document_id.clone(),
                            old_tree_type,
                            SubelementsDeletionBehavior::Error,
                        ));
                        self.history_migration_batch(
                            deletes,
                            transaction,
                            platform_version,
                            &mut stats,
                        )?;
                        self.history_migration_batch(
                            vec![QualifiedGroveDbOp::insert_or_replace_op(
                                primary_path.clone(),
                                document_id.clone(),
                                pointer,
                            )],
                            transaction,
                            platform_version,
                            &mut stats,
                        )?;
                        if let Some(references) = index_entries.remove(&document_id) {
                            let rewrites: Vec<_> = references
                                .iter()
                                .cloned()
                                .map(|(path, key, mut element)| {
                                    match &mut element {
                                        Element::Reference(reference, hops, _)
                                        | Element::ReferenceWithSumItem(reference, hops, _, _) => {
                                            *reference = UpstreamRootHeightReference(
                                                4,
                                                vec![vec![0], document_id.clone()],
                                            );
                                            *hops = Some(2);
                                        }
                                        _ => unreachable!("only references are collected"),
                                    }
                                    QualifiedGroveDbOp::insert_or_replace_op(path, key, element)
                                })
                                .collect();
                            self.history_migration_batch(
                                rewrites,
                                transaction,
                                platform_version,
                                &mut stats,
                            )?;
                            self.history_migration_check_index_rewrites(
                                &references,
                                &document_id,
                                transaction,
                                platform_version,
                                &mut stats,
                            )?;
                        }
                        stats.migrated_documents += 1;
                    }
                    if !index_entries.is_empty() {
                        return Err(corrupt("history index references a missing document"));
                    }
                }
            }
        }
        Ok(stats)
    }

    fn history_migration_check_index_rewrites(
        &self,
        references: &[IndexReference],
        document_id: &[u8],
        transaction: &Transaction,
        version: &PlatformVersion,
        stats: &mut DocumentHistoryMigrationStats,
    ) -> Result<(), Error> {
        let mut rewritten = 0;
        for (path, key, original) in references {
            let mut expected = original.clone();
            match &mut expected {
                Element::Reference(reference, hops, _)
                | Element::ReferenceWithSumItem(reference, hops, _, _) => {
                    *reference =
                        UpstreamRootHeightReference(4, vec![vec![0], document_id.to_vec()]);
                    *hops = Some(2);
                }
                _ => continue,
            }
            let stored = self.grove.get_raw(
                path.as_slice().into(),
                key,
                Some(transaction),
                &version.drive.grove_version,
            );
            stats.cost += stored.cost;
            if stored.value? == expected {
                rewritten += 1;
            }
        }
        if rewritten != references.len() {
            return Err(corrupt(
                "history index rewrite count differs from its inventory",
            ));
        }
        stats.rewritten_index_entries += rewritten as u64;
        Ok(())
    }

    fn history_migration_count_revisions(
        entries: &[(Vec<u8>, Element)],
        stats: &mut DocumentHistoryMigrationStats,
    ) -> Result<(), Error> {
        stats.maximum_document_revisions =
            stats.maximum_document_revisions.max(entries.len() as u64);
        for (_, element) in entries {
            let Element::Item(bytes, _) = element else {
                return Err(corrupt("history contains a non-item revision"));
            };
            stats.revisions += 1;
            stats.revision_bytes += bytes.len() as u64;
        }
        Ok(())
    }

    fn history_migration_pointer(
        mut pointer: Element,
        id: &[u8],
        revisions: &BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> Result<Element, Error> {
        let (reference, hops) = match &mut pointer {
            Element::Reference(reference, hops, _)
            | Element::ReferenceWithSumItem(reference, hops, _, _) => (reference, hops),
            _ => return Err(corrupt("current history pointer is not a reference")),
        };
        let SiblingReference(time) = reference else {
            return Err(corrupt(
                "current history pointer is not a sibling reference",
            ));
        };
        let composite = revisions
            .get(time)
            .ok_or_else(|| corrupt("current pointer references a missing revision"))?
            .clone();
        *reference = UpstreamRootHeightReference(
            4,
            vec![vec![DOCUMENT_HISTORY_TREE_KEY], id.to_vec(), composite],
        );
        *hops = Some(1);
        Ok(pointer)
    }

    fn history_migration_entries(
        &self,
        path: &[Vec<u8>],
        transaction: &Transaction,
        version: &PlatformVersion,
        stats: &mut DocumentHistoryMigrationStats,
    ) -> Result<Vec<(Vec<u8>, Element)>, Error> {
        let mut query = Query::new();
        query.insert_all();
        let mut operations = vec![];
        let result = self.grove_get_raw_path_query(
            &PathQuery::new(path.to_vec(), SizedQuery::new(query, None, None)),
            Some(transaction),
            QueryResultType::QueryKeyElementPairResultType,
            &mut operations,
            &version.drive,
        )?;
        for operation in operations {
            if let LowLevelDriveOperation::CalculatedCostOperation(cost) = operation {
                stats.cost += cost;
            }
        }
        let mut entries = result.0.to_key_elements();
        // Raw range queries normalize references to absolute paths. Read the
        // stored reference itself so migration preserves its flags and shape.
        for (key, element) in &mut entries {
            if matches!(
                element,
                Element::Reference(..) | Element::ReferenceWithSumItem(..)
            ) {
                let raw = self.grove.get_raw(
                    path.iter()
                        .map(Vec::as_slice)
                        .collect::<Vec<_>>()
                        .as_slice()
                        .into(),
                    key,
                    Some(transaction),
                    &version.drive.grove_version,
                );
                stats.cost += raw.cost;
                *element = raw.value?;
            }
        }
        Ok(entries)
    }

    fn history_migration_index_entries(
        &self,
        path: Vec<Vec<u8>>,
        transaction: &Transaction,
        version: &PlatformVersion,
        stats: &mut DocumentHistoryMigrationStats,
        references: &mut IndexEntries,
    ) -> Result<(), Error> {
        for (key, element) in self.history_migration_entries(&path, transaction, version, stats)? {
            if element.is_any_tree() {
                let mut child = path.clone();
                child.push(key);
                self.history_migration_index_entries(
                    child,
                    transaction,
                    version,
                    stats,
                    references,
                )?;
            } else if let Element::Reference(UpstreamRootHeightReference(4, target), ..)
            | Element::ReferenceWithSumItem(
                UpstreamRootHeightReference(4, target),
                ..,
            ) = &element
            {
                if target.first().map(Vec::as_slice) != Some(&[0])
                    || !(target.len() == 2 || target.len() == 3 && target[2] == [0])
                {
                    return Err(corrupt("unexpected document index reference target"));
                }
                stats.index_entries += 1;
                references
                    .entry(target[1].clone())
                    .or_default()
                    .push((path.clone(), key, element));
            } else {
                return Err(corrupt("unexpected leaf in a historical document index"));
            }
        }
        Ok(())
    }

    fn history_migration_batch(
        &self,
        operations: Vec<QualifiedGroveDbOp>,
        transaction: &Transaction,
        version: &PlatformVersion,
        stats: &mut DocumentHistoryMigrationStats,
    ) -> Result<(), Error> {
        let result = self.grove.apply_batch(
            operations,
            None,
            Some(transaction),
            &version.drive.grove_version,
        );
        stats.cost += result.cost;
        result.value?;
        stats.batches += 1;
        Ok(())
    }
}
