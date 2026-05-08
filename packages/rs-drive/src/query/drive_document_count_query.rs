use std::collections::{BTreeMap, BTreeSet};

#[cfg(feature = "server")]
use crate::drive::Drive;
#[cfg(feature = "server")]
use crate::error::query::QuerySyntaxError;
#[cfg(feature = "server")]
use crate::error::Error;
#[cfg(feature = "server")]
use crate::util::grove_operations::DirectQueryType;
#[cfg(feature = "server")]
use dpp::version::drive_versions::DriveVersion;
#[cfg(feature = "server")]
use grovedb::query_result_type::QueryResultType;
#[cfg(feature = "server")]
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
#[cfg(feature = "server")]
use grovedb_path::SubtreePath;

#[cfg(feature = "server")]
use crate::drive::RootTree;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
#[cfg(feature = "server")]
use dpp::data_contract::document_type::IndexProperty;
use dpp::data_contract::document_type::{DocumentTypeRef, Index};
#[cfg(feature = "server")]
use dpp::version::PlatformVersion;

use super::conditions::{WhereClause, WhereOperator};

/// A query to count documents using CountTree elements in the index path.
///
/// This struct encapsulates all the information needed to perform a count
/// query on a document type's countable index, including optional split-by
/// functionality for getting per-value counts.
#[derive(Debug, Clone)]
pub struct DriveDocumentCountQuery<'a> {
    /// The document type to count
    pub document_type: DocumentTypeRef<'a>,
    /// The contract id (32 bytes)
    pub contract_id: [u8; 32],
    /// The document type name
    pub document_type_name: String,
    /// The countable index to use
    pub index: &'a Index,
    /// The equality where clauses that match index prefix properties
    pub where_clauses: Vec<WhereClause>,
    /// Optional property to split counts by. When set, returns per-value
    /// counts for this property instead of a single total count.
    pub split_by_property: Option<String>,
}

/// An entry in a split count result, containing the serialized key
/// and the count of documents matching that key value.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitCountEntry {
    /// The serialized key bytes for this value
    pub key: Vec<u8>,
    /// The count of documents matching this key value
    pub count: u64,
}

impl<'a> DriveDocumentCountQuery<'a> {
    /// Returns `true` if the where-clause operator is one the count fast path
    /// can serve via point-lookups in a CountTree.
    ///
    /// Today that's `Equal` (one path) and `In` (cartesian fork over the listed
    /// values). Range operators (`>`, `<`, `Between*`, `StartsWith`) need a
    /// boundary walk that the current PathQuery infrastructure cannot express;
    /// callers detect those via [`Self::has_unsupported_operator`] and surface
    /// an error instead of silently returning a wrong count.
    fn is_indexable_for_count(op: WhereOperator) -> bool {
        matches!(op, WhereOperator::Equal | WhereOperator::In)
    }

    /// Returns `true` if any where clause uses an operator the count fast path
    /// cannot serve. Callers should treat this as a query-rejection signal.
    pub fn has_unsupported_operator(where_clauses: &[WhereClause]) -> bool {
        where_clauses
            .iter()
            .any(|wc| !Self::is_indexable_for_count(wc.operator))
    }

    /// Finds a countable index whose properties form a prefix that matches the
    /// indexable (Equal / In) where-clause fields. For a count query:
    /// - All indexable where-clause fields must appear as a prefix of the index properties
    /// - The index must have `countable = true`
    /// - Returns `None` if any where clause uses an operator other than `Equal` / `In`
    /// - Among matching indexes, we prefer the one with the most properties
    ///   matched by where clauses (most specific)
    pub fn find_countable_index_for_where_clauses<'b>(
        indexes: &'b BTreeMap<String, Index>,
        where_clauses: &[WhereClause],
    ) -> Option<&'b Index> {
        if Self::has_unsupported_operator(where_clauses) {
            return None;
        }

        let indexable_fields: BTreeSet<&str> = where_clauses
            .iter()
            .filter(|wc| Self::is_indexable_for_count(wc.operator))
            .map(|wc| wc.field.as_str())
            .collect();

        let mut best_match: Option<(&Index, usize)> = None;

        for index in indexes.values() {
            if !index.countable {
                continue;
            }

            // Check that the indexable where-clause fields form a prefix of
            // the index properties.
            let mut prefix_len = 0;
            for prop in &index.properties {
                if indexable_fields.contains(prop.name.as_str()) {
                    prefix_len += 1;
                } else {
                    break;
                }
            }

            // All indexable where-clause fields must be consumed as a prefix.
            if prefix_len < indexable_fields.len() {
                continue;
            }

            // Prefer the index with the longest matching prefix (most specific).
            match &best_match {
                None => best_match = Some((index, prefix_len)),
                Some((_, best_len)) if prefix_len > *best_len => {
                    best_match = Some((index, prefix_len));
                }
                _ => {}
            }
        }

        best_match.map(|(index, _)| index)
    }

    /// Finds a countable index where:
    /// - The indexable (Equal / In) where-clause fields form a prefix of the index properties
    /// - The `split_property` is the next property after the covered prefix
    /// - The index has `countable = true`
    /// - Returns `None` if any where clause uses an operator other than `Equal` / `In`
    pub fn find_countable_index_for_split<'b>(
        indexes: &'b BTreeMap<String, Index>,
        where_clauses: &[WhereClause],
        split_property: &str,
    ) -> Option<&'b Index> {
        if Self::has_unsupported_operator(where_clauses) {
            return None;
        }

        let indexable_fields: BTreeSet<&str> = where_clauses
            .iter()
            .filter(|wc| Self::is_indexable_for_count(wc.operator))
            .map(|wc| wc.field.as_str())
            .collect();

        for index in indexes.values() {
            if !index.countable {
                continue;
            }

            // Check that indexable where-clause fields form a prefix.
            let mut prefix_len = 0;
            for prop in &index.properties {
                if indexable_fields.contains(prop.name.as_str()) {
                    prefix_len += 1;
                } else {
                    break;
                }
            }

            if prefix_len < indexable_fields.len() {
                continue;
            }

            // The split property must be the next property after the prefix.
            if let Some(next_prop) = index.properties.get(prefix_len) {
                if next_prop.name == split_property {
                    return Some(index);
                }
            }
        }

        None
    }

    /// Executes the count query without generating a proof.
    ///
    /// When `split_by_property` is `None`, returns the total count as a single
    /// `SplitCountEntry` with an empty key.
    ///
    /// When `split_by_property` is `Some`, returns per-value counts for the
    /// split property.
    #[cfg(feature = "server")]
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        if self.split_by_property.is_some() {
            self.execute_split_count(drive, transaction, platform_version)
        } else {
            let count = self.execute_total_count(drive, transaction, platform_version)?;
            Ok(vec![SplitCountEntry { key: vec![], count }])
        }
    }

    /// Executes the count query and generates a GroveDB proof.
    ///
    /// Returns the raw proof bytes. The caller is responsible for verifying
    /// the proof and extracting the count from the verified result.
    #[cfg(feature = "server")]
    pub fn execute_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;

        // Build the same path as execute_no_proof
        let mut path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        // Walk the index properties, pushing property keys and equality values
        for prop in &self.index.properties {
            let matching_clause = self
                .where_clauses
                .iter()
                .find(|wc| wc.field == prop.name && wc.operator == WhereOperator::Equal);

            if let Some(clause) = matching_clause {
                path.push(prop.name.as_bytes().to_vec());
                let serialized_value = self.document_type.serialize_value_for_key(
                    prop.name.as_str(),
                    &clause.value,
                    platform_version,
                )?;
                path.push(serialized_value);
            } else {
                break;
            }
        }

        // Build a path query that covers the count tree and its contents
        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(path, SizedQuery::new(query, None, None));

        let proof = drive
            .grove
            .get_proved_path_query(&path_query, None, transaction, &drive_version.grove_version)
            .unwrap()
            .map_err(|e| Error::GroveDB(Box::new(e)))?;

        Ok(proof)
    }

    /// Executes the total count query, returning a single u64 count.
    ///
    /// Walks the index level-by-level, branching on `In` clauses (each value
    /// adds a path) and falling through to [`Self::count_recursive`] for any
    /// trailing index properties that have no matching where clause.
    #[cfg(feature = "server")]
    fn execute_total_count(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        // Build the base path: [DataContractDocuments, contract_id, 1, doc_type_name]
        let base_path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        self.expand_paths_and_count(drive, base_path, 0, transaction, platform_version)
    }

    /// Recursive helper for [`Self::execute_total_count`].
    ///
    /// Visits the index property at `prop_idx`. If a matching where clause is
    /// found:
    ///   - `Equal`  → extend the current path with `(prop_name, value)` and recurse.
    ///   - `In`     → for each value in the clause's array, clone the path,
    ///                extend with that value, recurse, and sum the per-branch
    ///                counts. This is the cartesian fork.
    ///   - anything else → unreachable; the index picker rejects the query.
    /// If no clause matches the current property, hand off to
    /// [`Self::count_recursive`] which sums all sub-counts at the remaining
    /// levels.
    #[cfg(feature = "server")]
    fn expand_paths_and_count(
        &self,
        drive: &Drive,
        current_path: Vec<Vec<u8>>,
        prop_idx: usize,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<u64, Error> {
        let drive_version = &platform_version.drive;

        if prop_idx == self.index.properties.len() {
            // All index properties resolved to a fixed key — O(1) read.
            return Self::fetch_count_at_path(drive, &current_path, transaction, drive_version);
        }

        let prop = &self.index.properties[prop_idx];
        let matching_clause = self.where_clauses.iter().find(|wc| wc.field == prop.name);

        let Some(clause) = matching_clause else {
            // No clause for this property. Walk all values at the remaining
            // levels and sum.
            let remaining = &self.index.properties[prop_idx..];
            return Self::count_recursive(
                drive,
                current_path,
                remaining,
                transaction,
                drive_version,
            );
        };

        match clause.operator {
            WhereOperator::Equal => {
                let mut new_path = current_path;
                new_path.push(prop.name.as_bytes().to_vec());
                new_path.push(self.document_type.serialize_value_for_key(
                    prop.name.as_str(),
                    &clause.value,
                    platform_version,
                )?);
                self.expand_paths_and_count(
                    drive,
                    new_path,
                    prop_idx + 1,
                    transaction,
                    platform_version,
                )
            }
            WhereOperator::In => {
                let values = clause.value.as_array().ok_or_else(|| {
                    Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                        "In where-clause value must be an array",
                    ))
                })?;

                let mut total: u64 = 0;
                for v in values {
                    let mut new_path = current_path.clone();
                    new_path.push(prop.name.as_bytes().to_vec());
                    new_path.push(self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        v,
                        platform_version,
                    )?);
                    total = total.saturating_add(self.expand_paths_and_count(
                        drive,
                        new_path,
                        prop_idx + 1,
                        transaction,
                        platform_version,
                    )?);
                }
                Ok(total)
            }
            _ => Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "count fast path supports only Equal and In where-clause operators",
                ),
            )),
        }
    }

    /// Executes a split count query, returning per-value counts for the
    /// split property.
    ///
    /// Walks the index prefix that precedes `split_by_property` level by
    /// level, branching on `In` clauses. For each fully-resolved prefix,
    /// runs the per-split-value sub-query (see [`Self::collect_split_at_prefix`])
    /// and merges the results by split key, summing counts.
    #[cfg(feature = "server")]
    fn execute_split_count(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let split_property = self
            .split_by_property
            .as_deref()
            .expect("split_by_property must be Some when calling execute_split_count");

        let split_prop_idx = self
            .index
            .properties
            .iter()
            .position(|p| p.name == split_property)
            .unwrap_or(0);

        let base_path = vec![
            vec![RootTree::DataContractDocuments as u8],
            self.contract_id.to_vec(),
            vec![1u8],
            self.document_type_name.as_bytes().to_vec(),
        ];

        let mut merged: BTreeMap<Vec<u8>, u64> = BTreeMap::new();
        self.expand_split_prefix_paths(
            drive,
            base_path,
            0,
            split_prop_idx,
            split_property,
            transaction,
            platform_version,
            &mut merged,
        )?;

        Ok(merged
            .into_iter()
            .filter(|(_, count)| *count > 0)
            .map(|(key, count)| SplitCountEntry { key, count })
            .collect())
    }

    /// Walks the index up to `split_prop_idx`, branching on `In`. At each
    /// fully-resolved prefix, calls [`Self::collect_split_at_prefix`] to
    /// gather the per-split-value counts, and accumulates them into `merged`.
    #[cfg(feature = "server")]
    #[allow(clippy::too_many_arguments)]
    fn expand_split_prefix_paths(
        &self,
        drive: &Drive,
        current_path: Vec<Vec<u8>>,
        prop_idx: usize,
        split_prop_idx: usize,
        split_property: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        merged: &mut BTreeMap<Vec<u8>, u64>,
    ) -> Result<(), Error> {
        if prop_idx == split_prop_idx {
            // Reached the split property level under this prefix. Run the
            // per-split-value sub-query and merge entries by key.
            return self.collect_split_at_prefix(
                drive,
                current_path,
                split_prop_idx,
                split_property,
                transaction,
                platform_version,
                merged,
            );
        }

        let prop = &self.index.properties[prop_idx];
        let clause = self
            .where_clauses
            .iter()
            .find(|wc| wc.field == prop.name)
            .ok_or_else(|| {
                // The index picker guarantees every property before the split
                // property has a matching clause; missing one indicates a
                // mis-picked index.
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "split count: missing where clause for an index property preceding the split property",
                ))
            })?;

        match clause.operator {
            WhereOperator::Equal => {
                let mut new_path = current_path;
                new_path.push(prop.name.as_bytes().to_vec());
                new_path.push(self.document_type.serialize_value_for_key(
                    prop.name.as_str(),
                    &clause.value,
                    platform_version,
                )?);
                self.expand_split_prefix_paths(
                    drive,
                    new_path,
                    prop_idx + 1,
                    split_prop_idx,
                    split_property,
                    transaction,
                    platform_version,
                    merged,
                )
            }
            WhereOperator::In => {
                let values = clause.value.as_array().ok_or_else(|| {
                    Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                        "In where-clause value must be an array",
                    ))
                })?;

                for v in values {
                    let mut new_path = current_path.clone();
                    new_path.push(prop.name.as_bytes().to_vec());
                    new_path.push(self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        v,
                        platform_version,
                    )?);
                    self.expand_split_prefix_paths(
                        drive,
                        new_path,
                        prop_idx + 1,
                        split_prop_idx,
                        split_property,
                        transaction,
                        platform_version,
                        merged,
                    )?;
                }
                Ok(())
            }
            _ => Err(Error::Query(
                QuerySyntaxError::InvalidWhereClauseComponents(
                    "split count fast path supports only Equal and In where-clause operators",
                ),
            )),
        }
    }

    /// Reads all per-value sub-counts for `split_property` under
    /// `prefix_path`, summing per-key counts into `merged`. Mirrors the
    /// original (pre-`In`-support) loop; factored out so the prefix-walk
    /// recursion can call it once per resolved prefix.
    #[cfg(feature = "server")]
    #[allow(clippy::too_many_arguments)]
    fn collect_split_at_prefix(
        &self,
        drive: &Drive,
        prefix_path: Vec<Vec<u8>>,
        split_prop_idx: usize,
        split_property: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
        merged: &mut BTreeMap<Vec<u8>, u64>,
    ) -> Result<(), Error> {
        let drive_version = &platform_version.drive;

        // Push the split-property key onto the prefix to address the per-value
        // subtree level.
        let mut path = prefix_path;
        path.push(split_property.as_bytes().to_vec());

        let mut query = Query::new();
        query.insert_all();
        let path_query = PathQuery::new(path.clone(), SizedQuery::new(query, None, None));

        let mut drive_operations = vec![];
        let result = drive.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut drive_operations,
            drive_version,
        );

        let (elements, _) = match result {
            Ok(result) => result,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                ) =>
            {
                // No documents under this prefix; nothing to merge.
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        let key_elements = elements.to_key_elements();
        if key_elements.is_empty() {
            return Ok(());
        }

        let remaining_properties = &self.index.properties[split_prop_idx + 1..];

        for (key, _element) in key_elements {
            let mut value_path = path.clone();
            value_path.push(key.clone());

            let count = if remaining_properties.is_empty() {
                Self::fetch_count_at_path(drive, &value_path, transaction, drive_version)?
            } else {
                Self::count_recursive(
                    drive,
                    value_path,
                    remaining_properties,
                    transaction,
                    drive_version,
                )?
            };

            if count == 0 {
                continue;
            }
            *merged.entry(key).or_insert(0) += count;
        }

        Ok(())
    }

    /// Fetches the CountTree element count at the given path.
    /// The CountTree element is at key [0] under the path.
    #[cfg(feature = "server")]
    fn fetch_count_at_path(
        drive: &Drive,
        path: &[Vec<u8>],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<u64, Error> {
        let mut drive_operations = vec![];
        let path_refs: Vec<&[u8]> = path.iter().map(|p| p.as_slice()).collect();
        let element = drive.grove_get_raw_optional(
            SubtreePath::from(path_refs.as_slice()),
            &[0],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;

        Ok(element.map_or(0, |e| e.count_value_or_default()))
    }

    /// Recursively descends through remaining index property levels,
    /// iterating over all values at each level, and sums the CountTree
    /// counts at the terminal level.
    #[cfg(feature = "server")]
    fn count_recursive(
        drive: &Drive,
        current_path: Vec<Vec<u8>>,
        remaining_properties: &[IndexProperty],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<u64, Error> {
        if remaining_properties.is_empty() {
            return Self::fetch_count_at_path(drive, &current_path, transaction, drive_version);
        }

        let prop = &remaining_properties[0];
        let rest = &remaining_properties[1..];

        // Push the index property key to descend into that level
        let mut property_path = current_path;
        property_path.push(prop.name.as_bytes().to_vec());

        // Query all children (value subtrees) at this property level
        let mut query = Query::new();
        query.insert_all();

        let path_query = PathQuery::new(property_path.clone(), SizedQuery::new(query, None, None));

        let mut drive_operations = vec![];
        let result = drive.grove_get_raw_path_query(
            &path_query,
            transaction,
            QueryResultType::QueryKeyElementPairResultType,
            &mut drive_operations,
            drive_version,
        );

        let (elements, _) = match result {
            Ok(result) => result,
            Err(Error::GroveDB(e))
                if matches!(
                    e.as_ref(),
                    grovedb::Error::PathNotFound(_)
                        | grovedb::Error::PathParentLayerNotFound(_)
                        | grovedb::Error::PathKeyNotFound(_)
                ) =>
            {
                return Ok(0);
            }
            Err(e) => return Err(e),
        };

        let key_elements = elements.to_key_elements();

        if key_elements.is_empty() {
            return Ok(0);
        }

        let mut total_count: u64 = 0;

        for (key, _element) in key_elements {
            let mut value_path = property_path.clone();
            value_path.push(key);

            let sub_count =
                Self::count_recursive(drive, value_path, rest, transaction, drive_version)?;
            total_count = total_count.saturating_add(sub_count);
        }

        Ok(total_count)
    }
}

#[cfg(feature = "server")]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::drive::Drive;
    use crate::util::object_size_info::DocumentInfo::DocumentRefInfo;
    use crate::util::object_size_info::{DocumentAndContractInfo, OwnedDocumentInfo};
    use crate::util::storage_flags::StorageFlags;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
    use dpp::data_contract::document_type::random_document::CreateRandomDocument;
    use dpp::document::{Document, DocumentV0};
    use dpp::identifier::Identifier;
    use dpp::platform_value::Value;
    use dpp::tests::json_document::json_document_to_contract_with_ids;
    use dpp::version::PlatformVersion;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::borrow::Cow;
    use std::collections::BTreeMap as StdBTreeMap;

    fn setup_drive_and_contract() -> (Drive, dpp::prelude::DataContract) {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        drive
            .apply_contract(
                &data_contract,
                BlockInfo::default(),
                true,
                StorageFlags::optional_default_as_cow(),
                None,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        (drive, data_contract)
    }

    fn insert_random_documents(
        drive: &Drive,
        data_contract: &dpp::prelude::DataContract,
        document_type_name: &str,
        count: usize,
        seed: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let document_type = data_contract
            .document_type_for_name(document_type_name)
            .expect("expected document type");

        let mut std_rng = StdRng::seed_from_u64(seed);
        for _ in 0..count {
            let random_document = document_type
                .random_document_with_rng(&mut std_rng, platform_version)
                .expect("expected to get random document");

            let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentRefInfo((&random_document, storage_flags)),
                            owner_id: None,
                        },
                        contract: data_contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to insert document");
        }
    }

    /// Inserts a person document with a controlled set of property values,
    /// so tests can drive the count fast path with known firstName / age
    /// values rather than relying on the random-document generator.
    fn insert_person_doc(
        drive: &Drive,
        data_contract: &dpp::prelude::DataContract,
        id: [u8; 32],
        first_name: &str,
        middle_name: &str,
        last_name: &str,
        age: u64,
    ) {
        let platform_version = PlatformVersion::latest();
        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let mut properties = StdBTreeMap::new();
        properties.insert("firstName".to_string(), Value::Text(first_name.to_string()));
        properties.insert(
            "middleName".to_string(),
            Value::Text(middle_name.to_string()),
        );
        properties.insert("lastName".to_string(), Value::Text(last_name.to_string()));
        properties.insert("age".to_string(), Value::U64(age));

        let document: Document = DocumentV0 {
            id: Identifier::from(id),
            owner_id: Identifier::from([0u8; 32]),
            properties,
            revision: None,
            created_at: None,
            updated_at: None,
            transferred_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            transferred_at_block_height: None,
            created_at_core_block_height: None,
            updated_at_core_block_height: None,
            transferred_at_core_block_height: None,
            creator_id: None,
        }
        .into();

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        drive
            .add_document_for_contract(
                DocumentAndContractInfo {
                    owned_document_info: OwnedDocumentInfo {
                        document_info: DocumentRefInfo((&document, storage_flags)),
                        owner_id: None,
                    },
                    contract: data_contract,
                    document_type,
                },
                false,
                BlockInfo::default(),
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to insert document");
    }

    #[test]
    fn test_count_query_total_count_with_documents() {
        let (drive, data_contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();

        insert_random_documents(&drive, &data_contract, "person", 5, 500);

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &[],
        )
        .expect("expected to find countable index");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "person".to_string(),
            index,
            where_clauses: vec![],
            split_by_property: None,
        };

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("expected query to succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].count, 5, "expected count of 5 documents");
        assert!(
            results[0].key.is_empty(),
            "expected empty key for total count"
        );

        // Also verify proof generation works
        let proof = query
            .execute_with_proof(&drive, None, platform_version)
            .expect("expected proof generation to succeed");
        assert!(!proof.is_empty(), "expected non-empty proof");
    }

    #[test]
    fn test_count_query_total_count_empty() {
        let (drive, data_contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &[],
        )
        .expect("expected to find countable index");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "person".to_string(),
            index,
            where_clauses: vec![],
            split_by_property: None,
        };

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("expected query to succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].count, 0, "expected count of 0 documents");

        // Also verify proof generation works on empty index
        let proof = query
            .execute_with_proof(&drive, None, platform_version)
            .expect("expected proof generation to succeed");
        assert!(!proof.is_empty(), "expected non-empty proof");
    }

    #[test]
    fn test_count_query_split_by_property() {
        let (drive, data_contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();

        insert_random_documents(&drive, &data_contract, "person", 5, 600);

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let index = DriveDocumentCountQuery::find_countable_index_for_split(
            document_type.indexes(),
            &[],
            "firstName",
        )
        .expect("expected to find countable index for split");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "person".to_string(),
            index,
            where_clauses: vec![],
            split_by_property: Some("firstName".to_string()),
        };

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("expected query to succeed");

        let total: u64 = results.iter().map(|e| e.count).sum();
        assert_eq!(total, 5, "expected total split count of 5 documents");

        for entry in &results {
            assert!(!entry.key.is_empty(), "expected non-empty split key");
            assert!(entry.count > 0, "expected positive count per split");
        }

        // Also verify proof generation works for split query
        let proof = query
            .execute_with_proof(&drive, None, platform_version)
            .expect("expected proof generation to succeed");
        assert!(!proof.is_empty(), "expected non-empty proof");
    }

    #[test]
    fn test_find_countable_index_for_where_clauses_no_match() {
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        // Create a where clause for a field that doesn't appear as a prefix of any index
        let where_clause = WhereClause {
            field: "nonExistentField".to_string(),
            operator: WhereOperator::Equal,
            value: Value::Text("test".to_string()),
        };

        let result = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            &[where_clause],
        );

        assert!(
            result.is_none(),
            "expected no countable index for non-existent field"
        );
    }

    #[test]
    fn test_find_countable_index_for_split_no_match() {
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let result = DriveDocumentCountQuery::find_countable_index_for_split(
            document_type.indexes(),
            &[],
            "nonExistentField",
        );

        assert!(
            result.is_none(),
            "expected no countable index for non-existent split field"
        );
    }

    #[test]
    fn test_has_unsupported_operator() {
        let eq_clause = WhereClause {
            field: "age".to_string(),
            operator: WhereOperator::Equal,
            value: Value::U64(30),
        };
        let in_clause = WhereClause {
            field: "age".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![Value::U64(30), Value::U64(40)]),
        };
        let gt_clause = WhereClause {
            field: "age".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::U64(20),
        };

        assert!(!DriveDocumentCountQuery::has_unsupported_operator(&[]));
        assert!(!DriveDocumentCountQuery::has_unsupported_operator(&[
            eq_clause.clone()
        ]));
        assert!(!DriveDocumentCountQuery::has_unsupported_operator(&[
            in_clause.clone()
        ]));
        assert!(!DriveDocumentCountQuery::has_unsupported_operator(&[
            eq_clause.clone(),
            in_clause.clone()
        ]));
        assert!(DriveDocumentCountQuery::has_unsupported_operator(&[
            gt_clause.clone()
        ]));
        assert!(DriveDocumentCountQuery::has_unsupported_operator(&[
            eq_clause, gt_clause
        ]));
    }

    #[test]
    fn test_find_countable_index_rejects_unsupported_operator() {
        let platform_version = PlatformVersion::latest();

        let data_contract = json_document_to_contract_with_ids(
            "tests/supporting_files/contract/family/family-contract-countable.json",
            None,
            None,
            false,
            platform_version,
        )
        .expect("expected to get json based contract");

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let gt_clause = WhereClause {
            field: "age".to_string(),
            operator: WhereOperator::GreaterThan,
            value: Value::U64(20),
        };

        // Even though `age` exists as a countable index, GreaterThan disqualifies it
        // for the count fast path; the picker must report this as "no usable index"
        // so the handler turns it into a clear error.
        assert!(
            DriveDocumentCountQuery::find_countable_index_for_where_clauses(
                document_type.indexes(),
                &[gt_clause.clone()],
            )
            .is_none()
        );
        assert!(DriveDocumentCountQuery::find_countable_index_for_split(
            document_type.indexes(),
            &[gt_clause],
            "firstName",
        )
        .is_none());
    }

    #[test]
    fn test_count_query_total_count_with_in_operator() {
        let (drive, data_contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();

        // Three docs with age=30, two with age=40, one with age=50.
        insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "M", "Smith", 30);
        insert_person_doc(&drive, &data_contract, [2u8; 32], "Bob", "M", "Smith", 30);
        insert_person_doc(&drive, &data_contract, [3u8; 32], "Carol", "M", "Smith", 30);
        insert_person_doc(&drive, &data_contract, [4u8; 32], "Dave", "M", "Smith", 40);
        insert_person_doc(&drive, &data_contract, [5u8; 32], "Eve", "M", "Smith", 40);
        insert_person_doc(&drive, &data_contract, [6u8; 32], "Frank", "M", "Smith", 50);

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        // age IN [30, 40] should count 5 documents (3 + 2).
        let in_clause = WhereClause {
            field: "age".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![Value::U64(30), Value::U64(40)]),
        };

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            std::slice::from_ref(&in_clause),
        )
        .expect("expected to find countable index for In on age");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "person".to_string(),
            index,
            where_clauses: vec![in_clause],
            split_by_property: None,
        };

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("expected query to succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].count, 5,
            "expected count of 5 (age=30 has 3, age=40 has 2)"
        );
    }

    #[test]
    fn test_count_query_total_count_with_in_operator_no_matches() {
        let (drive, data_contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();

        insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "M", "Smith", 30);
        insert_person_doc(&drive, &data_contract, [2u8; 32], "Bob", "M", "Smith", 30);

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        // age IN [99, 100] - no matches.
        let in_clause = WhereClause {
            field: "age".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![Value::U64(99), Value::U64(100)]),
        };

        let index = DriveDocumentCountQuery::find_countable_index_for_where_clauses(
            document_type.indexes(),
            std::slice::from_ref(&in_clause),
        )
        .expect("expected to find countable index for In on age");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "person".to_string(),
            index,
            where_clauses: vec![in_clause],
            split_by_property: None,
        };

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("expected query to succeed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].count, 0, "expected count of 0 for unmatched In");
    }

    #[test]
    fn test_count_query_split_with_in_prefix() {
        let (drive, data_contract) = setup_drive_and_contract();
        let platform_version = PlatformVersion::latest();

        // firstName IN ["Alice", "Bob"] split by lastName
        // Expected: Smith=3 (Alice+Alice+Bob), Jones=2 (Alice+Bob), Doe=1 (Carol — excluded)
        insert_person_doc(&drive, &data_contract, [1u8; 32], "Alice", "M", "Smith", 30);
        insert_person_doc(&drive, &data_contract, [2u8; 32], "Alice", "N", "Smith", 31);
        insert_person_doc(&drive, &data_contract, [3u8; 32], "Bob", "M", "Smith", 32);
        insert_person_doc(&drive, &data_contract, [4u8; 32], "Alice", "M", "Jones", 33);
        insert_person_doc(&drive, &data_contract, [5u8; 32], "Bob", "M", "Jones", 34);
        insert_person_doc(&drive, &data_contract, [6u8; 32], "Carol", "M", "Doe", 35);

        let document_type = data_contract
            .document_type_for_name("person")
            .expect("expected document type");

        let in_clause = WhereClause {
            field: "firstName".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(vec![
                Value::Text("Alice".to_string()),
                Value::Text("Bob".to_string()),
            ]),
        };

        let index = DriveDocumentCountQuery::find_countable_index_for_split(
            document_type.indexes(),
            std::slice::from_ref(&in_clause),
            "lastName",
        )
        .expect("expected to find countable index for In + split lastName");

        let query = DriveDocumentCountQuery {
            document_type,
            contract_id: data_contract.id().to_buffer(),
            document_type_name: "person".to_string(),
            index,
            where_clauses: vec![in_clause],
            split_by_property: Some("lastName".to_string()),
        };

        let results = query
            .execute_no_proof(&drive, None, platform_version)
            .expect("expected query to succeed");

        let total: u64 = results.iter().map(|e| e.count).sum();
        assert_eq!(
            total, 5,
            "expected total of 5 (3 Smith + 2 Jones, Carol/Doe excluded)"
        );
        assert_eq!(
            results.len(),
            2,
            "expected 2 split entries (Smith and Jones)"
        );
        for entry in &results {
            assert!(entry.count > 0, "filtered split entries should be > 0");
        }
    }
}
