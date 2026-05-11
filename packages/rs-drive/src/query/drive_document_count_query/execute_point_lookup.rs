//! Equal/In point-lookup execution paths for the count query.
//!
//! No-proof and proof executors that walk the primary-key CountTree
//! at fully-resolved or partially-resolved index paths. The walk uses
//! O(1) CountTree reads at fixed-key paths and falls through to a
//! per-level sum for any trailing index properties without a where
//! clause.
//!
//! Range-mode executors live in
//! [`super::execute_range_count`](super::execute_range_count); this
//! file is the Equal/In half of the dispatch surface.
//!
//! Whole module is gated `feature = "server"` via the parent's
//! `pub mod execute_point_lookup;` declaration.

use super::super::conditions::WhereOperator;
use super::{DriveDocumentCountQuery, SplitCountEntry};
use crate::drive::{Drive, RootTree};
use crate::error::query::QuerySyntaxError;
use crate::error::Error;
use crate::util::grove_operations::DirectQueryType;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::document_type::IndexProperty;
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, Query, SizedQuery, TransactionArg};
use grovedb_path::SubtreePath;
use std::collections::BTreeSet;

impl DriveDocumentCountQuery<'_> {
    /// Executes the count query without generating a proof.
    ///
    /// Returns the total count as a single `SplitCountEntry` with an empty key.
    pub fn execute_no_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<SplitCountEntry>, Error> {
        let count = self.execute_total_count(drive, transaction, platform_version)?;
        Ok(vec![SplitCountEntry {
            in_key: None,
            key: vec![],
            count,
        }])
    }

    /// Executes the count query and generates a GroveDB proof.
    ///
    /// Returns the raw proof bytes. The caller is responsible for verifying
    /// the proof and extracting the count from the verified result.
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
    /// - `Equal` → extend the current path with `(prop_name, value)` and recurse.
    /// - `In` → for each value in the clause's array, clone the path, extend
    ///   with that value, recurse, and sum the per-branch counts. This is the
    ///   cartesian fork.
    /// - anything else → unreachable; the index picker rejects the query.
    ///
    /// If no clause matches the current property, hand off to
    /// [`Self::count_recursive`] which sums all sub-counts at the remaining
    /// levels.
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

                // `In` is set-membership: serialize each value to the canonical
                // index key and dedupe before forking. Without this, a query
                // like `age in [30, 30]` would visit and sum the same subtree
                // twice (Codex review finding #3).
                let mut seen_keys: BTreeSet<Vec<u8>> = BTreeSet::new();
                let mut total: u64 = 0;
                for v in values {
                    let serialized = self.document_type.serialize_value_for_key(
                        prop.name.as_str(),
                        v,
                        platform_version,
                    )?;
                    if !seen_keys.insert(serialized.clone()) {
                        continue;
                    }
                    let mut new_path = current_path.clone();
                    new_path.push(prop.name.as_bytes().to_vec());
                    new_path.push(serialized);
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

    /// Fetches the CountTree element count at the given path.
    /// The CountTree element is at key [0] under the path.
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
