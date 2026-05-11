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
use dpp::version::drive_versions::DriveVersion;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
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

    /// Generates a grovedb proof of the CountTree elements covering a
    /// fully-covered Equal/`In` count query against a `countable: true`
    /// index. Returns the raw proof bytes; the SDK-side
    /// [`Self::verify_point_lookup_count_proof`] walks the proof and
    /// extracts `count_value_or_default()` from each verified CountTree
    /// element.
    ///
    /// Builds the path query via
    /// [`Self::point_lookup_count_path_query`] (shared with the
    /// verifier so the merk-root recomputation matches). Errors surface
    /// from the builder when the query shape isn't supported — partial
    /// coverage, `In` on a non-last property, etc. — see that builder's
    /// docstring for the exhaustive contract.
    ///
    /// Proof size is O(k × log n) where k is the number of covered
    /// (Equal/In) branches and n is the tree depth: one merk path proof
    /// per CountTree element, not per matching document. Replaces the
    /// pre-this-PR materialize-and-count proof which scaled with
    /// matching docs and was capped at `u16::MAX`.
    pub fn execute_point_lookup_count_with_proof(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        let drive_version = &platform_version.drive;
        let path_query = self.point_lookup_count_path_query(platform_version)?;
        let proof = drive
            .grove
            .get_proved_path_query(&path_query, None, transaction, &drive_version.grove_version)
            .unwrap()
            .map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok(proof)
    }

    /// Executes the count query, returning a single `u64` count.
    ///
    /// Builds the path that lands exactly on the terminal CountTree for the
    /// covered Equal/`In` branches and reads `count_value_or_default()`. The
    /// picker (`find_countable_index_for_where_clauses`) is strict — it only
    /// returns an index when every index property has a matching `Equal`/`In`
    /// clause — so by the time we reach this executor every level has a
    /// resolved key.
    ///
    /// For `In` clauses (set-membership), each value forks a separate path
    /// and the per-branch counts are summed. Duplicate values that share a
    /// canonical encoding collapse to one fork.
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

    /// Walks the index property levels Equal-by-Equal (or forks on `In`),
    /// and reads the terminal CountTree's `count_value`.
    ///
    /// Contract: every index property MUST have a matching `Equal`/`In`
    /// clause. The strict picker
    /// ([`Self::find_countable_index_for_where_clauses`]) guarantees this
    /// upstream; the "missing clause for an index property" branch here is
    /// defensive — it returns
    /// `InvalidWhereClauseComponents` directing the caller at the
    /// index-design fix rather than silently falling through to a
    /// partial-coverage walk.
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
        let clause = self
            .where_clauses
            .iter()
            .find(|wc| wc.field == prop.name)
            .ok_or_else(|| {
                Error::Query(QuerySyntaxError::InvalidWhereClauseComponents(
                    "count query requires the where clauses to fully cover the \
                     countable index; one or more index properties have no \
                     matching `==` or `in` clause — use a more specific index \
                     (define a `countable: true` index whose properties exactly \
                     match the clauses) or set `documentsCountable: true` on the \
                     document type for unfiltered total counts",
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
                // index key and dedupe before forking. Without dedupe, a query
                // like `age in [30, 30]` would visit and sum the same subtree
                // twice — distinct values that share a canonical encoding
                // collapse to one fork.
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
}
