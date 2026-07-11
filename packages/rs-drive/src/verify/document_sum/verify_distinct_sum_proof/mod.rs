mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_sum_query::{DriveDocumentSumQuery, SumEntry};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// Verifies a regular grovedb range proof against a
    /// `rangeSummable: true` index's terminator `SumTree`s and
    /// returns the per-`(in_key, terminator_key)` sums. Sum analog
    /// of count's `verify_distinct_count_proof`.
    ///
    /// Used by the prove path's
    /// [`DocumentSumMode::RangeDistinctProof`] (GroupByRange /
    /// GroupByCompound + range + prove). Rebuilds the same
    /// `PathQuery` the prover used via
    /// [`Self::distinct_sum_path_query`] (including `limit` and
    /// `left_to_right` — both are encoded into the path query
    /// bytes) and walks the verified
    /// `(path, key, Option<Element>)` triples to extract
    /// `sum_value_or_default()` from each terminator SumTree.
    ///
    /// Cross-fork aggregation is intentionally NOT done here —
    /// callers reduce by `key` client-side if they want a flat
    /// histogram. See [`SumEntry`]'s sibling
    /// [`crate::query::SplitCountEntry`] for the no-merge
    /// rationale (identical contract on the sum side).
    pub fn verify_distinct_sum_proof(
        &self,
        proof: &[u8],
        limit: u16,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<SumEntry>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_sum
            .verify_distinct_sum_proof
        {
            0 => self.verify_distinct_sum_proof_v0(proof, limit, left_to_right, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentSumQuery::verify_distinct_sum_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
