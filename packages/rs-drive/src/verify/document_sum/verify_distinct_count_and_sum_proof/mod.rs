mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_average_query::AverageEntry;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// Verifies a per-distinct-key range-AVG proof against an index
    /// whose terminator value trees are count-sum-bearing variants
    /// (the chosen index opts into BOTH `rangeCountable: true` AND
    /// `rangeSummable: true`, i.e. a `rangeAverageable: true`
    /// index). Average analog of
    /// [`Self::verify_distinct_sum_proof`] / count's
    /// `verify_distinct_count_proof`.
    ///
    /// Rebuilds the same `PathQuery` the prover used via
    /// [`Self::distinct_sum_path_query`] (the count + sum sides
    /// share the same path-query shape — the difference is at
    /// proof-emission time which merk ops are emitted) and walks
    /// the verified `(path, key, Option<Element>)` triples to
    /// extract `count_sum_value_or_default()` from each present
    /// terminator element.
    pub fn verify_distinct_count_and_sum_proof(
        &self,
        proof: &[u8],
        limit: u16,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<AverageEntry>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_sum
            .verify_distinct_count_and_sum_proof
        {
            0 => self.verify_distinct_count_and_sum_proof_v0(
                proof,
                limit,
                left_to_right,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentSumQuery::verify_distinct_count_and_sum_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
