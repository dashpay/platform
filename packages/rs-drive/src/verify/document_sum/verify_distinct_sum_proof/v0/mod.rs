use crate::error::Error;
use crate::query::drive_document_sum_query::{DriveDocumentSumQuery, SumEntry};
use crate::query::WhereOperator;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentSumQuery<'_> {
    /// v0 of [`Self::verify_distinct_sum_proof`].
    ///
    /// Mirror of count's
    /// [`super::super::super::document_count::verify_distinct_count_proof`]
    /// — same path-query rebuild + element walk, but extracts
    /// `sum_value_or_default()` from each verified SumTree element
    /// instead of `count_value_or_default()`.
    ///
    /// For compound queries (`In` on prefix) the In value sits at
    /// `path[base_path_len]` (the first extra path segment beyond
    /// the path query's `path`); for flat queries the emitted
    /// path equals `path_query.path`, so `in_key` stays `None`.
    #[inline(always)]
    pub(super) fn verify_distinct_sum_proof_v0(
        &self,
        proof: &[u8],
        limit: u16,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<SumEntry>), Error> {
        let path_query =
            self.distinct_sum_path_query(Some(limit), left_to_right, platform_version)?;
        let base_path_len = path_query.path.len();
        let has_in_on_prefix = self
            .where_clauses
            .iter()
            .any(|wc| wc.operator == WhereOperator::In);
        let (root_hash, elements) =
            GroveDb::verify_query(proof, &path_query, &platform_version.drive.grove_version)
                .map_err(|e| Error::GroveDB(Box::new(e)))?;

        let mut out: Vec<SumEntry> = Vec::with_capacity(elements.len());
        for (path, key, elem) in elements {
            if let Some(e) = elem {
                let sum = e.sum_value_or_default();
                let in_key = if has_in_on_prefix && path.len() > base_path_len {
                    Some(path[base_path_len].clone())
                } else {
                    None
                };
                // Distinct-sum proof emits one entry per verified
                // `KVSum` op in the proof — always `Some(_)`. SDK-
                // side synthesis can add `None` entries for missing-
                // from-proof keys if the caller's request named them
                // (only meaningful for In-grouped paths; range-
                // distinct doesn't enumerate keys in advance).
                out.push(SumEntry {
                    in_key,
                    key,
                    sum: Some(sum),
                });
            }
        }
        Ok((root_hash, out))
    }
}
