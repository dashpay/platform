//! Activation conversion of the trees that hold document sums.
//!
//! From protocol version 15 a document type whose summed property is not declared within
//! `SystemLimits::max_summed_value_magnitude_in_64_bit_sums` keeps every sum in a tree with a
//! 128-bit total (`DocumentTypeV2Getters::uses_big_sums`). A type stored before that version
//! holds its sums in the trees with a 64-bit total. This walk converts each of those trees to
//! its 128-bit counterpart where it stands (`GroveDb::convert_to_big_sum_tree`): the tree keeps
//! its path, its children, its flags, its count, its sum and its wrapper, so no document, index
//! entry or reference is written again.
//!
//! The trees converted are the ones a write of the type now creates wide: the primary tree of a
//! type that sums its documents, and, for every index summing the property, the tree named
//! after its last property, the tree of each of that property's values and the entries tree
//! (`0`) in it. A tree is converted before the trees that add their sums to it, as GroveDB
//! requires. The trees of an index that sums nothing, and the counters of a
//! `summableOffCountIndex` index, keep the width they have.
//!
//! Two trees are left as stored, and counted: a `ProvableCountSumTree`, which has no
//! counterpart to convert to in place, and a ranked tree GroveDB refuses for a key too long for
//! the big sum axis. The trees adding their sums to one of them are left too, since a 128-bit
//! tree adds nothing to a 64-bit parent.
//!
//! Any other error halts the activation block for every validator, as for the other
//! activation migrations: the writes belong to the activation transaction, and no partial
//! conversion may commit.

use crate::drive::document::paths::contract_document_type_path_vec;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::QueryResultType;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::{is_flat_level_key, IndexLevel};
use dpp::version::PlatformVersion;
use grovedb::{Element, PathQuery, Query, SizedQuery, Transaction};

/// What the conversion of document sums found and did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DocumentSumConversionStats {
    /// All contracts inspected.
    pub contracts: u64,
    /// Document types that keep their sums in 128 bits.
    pub types: u64,
    /// Trees converted to their 128-bit counterpart.
    pub converted_trees: u64,
    /// Trees with a 64-bit sum left as stored: no counterpart converts them in place.
    pub trees_left_as_stored: u64,
}

/// What became of one tree.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// Nothing is stored at the key.
    Absent,
    /// The tree now keeps a 128-bit sum, or holds no 64-bit sum to convert.
    Settled,
    /// The tree keeps its 64-bit sum: the trees adding to it must keep theirs.
    LeftAsStored,
}

/// Whether an index summing a property of a type that keeps big sums ends at `level` or below
/// it: the levels the walk has anything to convert under.
fn holds_big_sums(level: &IndexLevel) -> bool {
    level
        .has_index_with_type()
        .is_some_and(|info| info.big_sums)
        || level.sub_levels().values().any(holds_big_sums)
}

impl Drive {
    /// Converts, for every document type in state that keeps its sums in 128 bits, the trees
    /// that hold those sums in 64 bits. See the module documentation.
    ///
    /// Runs once, on the first block of protocol version 15, in the activation transaction.
    /// A tree already converted is left alone, so running it again changes nothing.
    pub fn convert_document_sums_to_big_sums(
        &self,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<DocumentSumConversionStats, Error> {
        if platform_version
            .system_limits
            .max_summed_value_magnitude_in_64_bit_sums
            .is_none()
        {
            return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                "document sums are converted under a protocol version that keeps big sums",
            )));
        }
        let mut stats = DocumentSumConversionStats::default();
        let mut cursor: Option<[u8; 32]> = None;
        loop {
            let ids = self.fetch_contract_ids(
                cursor.map(|id| (id, false)),
                u16::MAX,
                Some(transaction),
                platform_version,
            )?;
            if ids.is_empty() {
                break;
            }
            cursor = ids.last().copied();
            for id in ids {
                stats.contracts += 1;
                let fetched = self
                    .fetch_contract(id, None, None, Some(transaction), platform_version)
                    .value?
                    .ok_or_else(|| {
                        Error::Drive(DriveError::CorruptedDriveState(
                            "enumerated contract is missing".to_string(),
                        ))
                    })?;
                for (name, document_type) in fetched.contract.document_types() {
                    let document_type = document_type.as_ref();
                    if !document_type.uses_big_sums() {
                        continue;
                    }
                    stats.types += 1;
                    let type_path = contract_document_type_path_vec(&id, name);
                    // The primary tree holds the type's total. An indexOnly type has none.
                    if document_type.documents_summable().is_some() && !document_type.index_only() {
                        self.convert_sum_tree(
                            &type_path,
                            &[0],
                            &mut stats,
                            transaction,
                            platform_version,
                        )?;
                    }
                    for (level_key, level) in document_type.index_structure().sub_levels() {
                        self.convert_index_level_sums(
                            &type_path,
                            level_key,
                            level,
                            &mut stats,
                            transaction,
                            platform_version,
                        )?;
                    }
                }
            }
        }
        Ok(stats)
    }

    /// Converts the trees of `level`, whose tree is stored at `level_key` under `parent_path`,
    /// and of the levels below it.
    fn convert_index_level_sums(
        &self,
        parent_path: &[Vec<u8>],
        level_key: &str,
        level: &IndexLevel,
        stats: &mut DocumentSumConversionStats,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        if !holds_big_sums(level) {
            return Ok(());
        }
        // Whether an index summing the property ends here: then the level's own tree, the
        // tree of each value and the entries tree in it hold its sums.
        let sums_here = level
            .has_index_with_type()
            .is_some_and(|info| info.big_sums);
        let level_tree = if sums_here {
            self.convert_sum_tree(
                parent_path,
                level_key.as_bytes(),
                stats,
                transaction,
                platform_version,
            )?
        } else if self
            .stored_element(
                parent_path,
                level_key.as_bytes(),
                transaction,
                platform_version,
            )?
            .is_some()
        {
            Outcome::Settled
        } else {
            Outcome::Absent
        };
        // A level below the first is created by the first document that reaches it.
        if level_tree == Outcome::Absent {
            return Ok(());
        }
        let mut level_path = parent_path.to_vec();
        level_path.push(level_key.as_bytes().to_vec());
        let converts_children = sums_here && level_tree == Outcome::Settled;

        // A flat index keeps its entries straight under its level, with no value level.
        if is_flat_level_key(level_key) {
            if converts_children {
                self.convert_sum_tree(&level_path, &[0], stats, transaction, platform_version)?;
            }
            return Ok(());
        }

        let mut query = Query::new();
        query.insert_all();
        let (values, _) = self.grove_get_raw_path_query(
            &PathQuery::new(level_path.clone(), SizedQuery::new(query, None, None)),
            Some(transaction),
            QueryResultType::QueryKeyElementPairResultType,
            &mut vec![],
            &platform_version.drive,
        )?;
        for (value_key, element) in values.to_key_elements() {
            // A `summableOffCountIndex` level holds counters at its value position, not trees.
            if !element.underlying().is_any_tree() {
                continue;
            }
            let mut value_path = level_path.clone();
            value_path.push(value_key.clone());
            if converts_children {
                let value_tree = self.convert_sum_tree(
                    &level_path,
                    &value_key,
                    stats,
                    transaction,
                    platform_version,
                )?;
                if value_tree == Outcome::Settled {
                    self.convert_sum_tree(&value_path, &[0], stats, transaction, platform_version)?;
                }
            }
            // A continuation's tree adds nothing to the value tree it is in (it is stored
            // wrapped, or the value tree sums nothing), so it converts whatever the value
            // tree's outcome.
            for (sub_level_key, sub_level) in level.sub_levels() {
                self.convert_index_level_sums(
                    &value_path,
                    sub_level_key,
                    sub_level,
                    stats,
                    transaction,
                    platform_version,
                )?;
            }
        }
        Ok(())
    }

    /// The element stored at `key` under `path`, as stored.
    fn stored_element(
        &self,
        path: &[Vec<u8>],
        key: &[u8],
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Element>, Error> {
        let path: Vec<&[u8]> = path.iter().map(Vec::as_slice).collect();
        self.grove
            .get_raw_optional(
                path.as_slice().into(),
                key,
                Some(transaction),
                &platform_version.drive.grove_version,
            )
            .value
            .map_err(|error| Error::GroveDB(Box::new(error)))
    }

    /// Converts the tree at `key` under `path` when it keeps a 64-bit sum that has a 128-bit
    /// counterpart, and says what became of it.
    fn convert_sum_tree(
        &self,
        path: &[Vec<u8>],
        key: &[u8],
        stats: &mut DocumentSumConversionStats,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<Outcome, Error> {
        let Some(element) = self.stored_element(path, key, transaction, platform_version)? else {
            return Ok(Outcome::Absent);
        };
        match element.underlying() {
            Element::SumTree(..)
            | Element::CountSumTree(..)
            | Element::ProvableSumTree(..)
            | Element::ProvableCountProvableSumTree(..)
            | Element::ProvableSumIndexedTree(..)
            | Element::ProvableCountProvableSumIndexedTree(..) => {
                let path: Vec<&[u8]> = path.iter().map(Vec::as_slice).collect();
                match self
                    .grove
                    .convert_to_big_sum_tree(
                        path.as_slice(),
                        key,
                        Some(transaction),
                        &platform_version.drive.grove_version,
                    )
                    .value
                {
                    Ok(()) => {
                        stats.converted_trees += 1;
                        Ok(Outcome::Settled)
                    }
                    // A ranked tree holding a key too long for the big sum axis.
                    Err(grovedb::Error::NotSupported(_)) => {
                        stats.trees_left_as_stored += 1;
                        Ok(Outcome::LeftAsStored)
                    }
                    Err(error) => Err(Error::GroveDB(Box::new(error))),
                }
            }
            // No tree with a 128-bit sum commits the count alone to its node hashes, so
            // this one cannot be converted where it stands.
            Element::ProvableCountSumTree(..) => {
                stats.trees_left_as_stored += 1;
                Ok(Outcome::LeftAsStored)
            }
            // Already wide, or an element that holds no sum of its own to widen.
            _ => Ok(Outcome::Settled),
        }
    }
}
