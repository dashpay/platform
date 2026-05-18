mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// Verifies a leaf-PCPS `AggregateCountAndSumOnRange` proof
    /// and returns `(root_hash, count, sum)` — the verified
    /// `(count, sum)` pair extracted from one merk traversal of the
    /// PCPS terminator. Counterpart to the prover-side
    /// [`execute_aggregate_count_and_sum_with_proof`](DriveDocumentSumQuery::execute_aggregate_count_and_sum_with_proof).
    /// Calls `GroveDb::verify_aggregate_count_and_sum_query` (grovedb
    /// PR #670 head `e98bab5f`).
    ///
    /// The client computes `avg = sum as f64 / count as f64` (or
    /// preferred precision) to recover the verified average — the
    /// proof commits both metrics from the **same** in-range set, so
    /// there's no way for the server to splice a count from one set
    /// with a sum from another. This is the load-bearing primitive
    /// for the [average-index-examples chapter]
    /// (../../../../book/src/drive/average-index-examples.md)'s
    /// Query 5.
    ///
    /// Tree-type restriction: the chosen index must declare BOTH
    /// `rangeCountable: true` AND `rangeSummable: true` so the
    /// terminator's value tree is a
    /// `ProvableCountProvableSumTree` (PCPS). Lighter sum-bearing or
    /// count-bearing variants reject the combined primitive at the
    /// merk gate.
    pub fn verify_aggregate_count_and_sum_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, u64, i64), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_sum
            .verify_aggregate_count_and_sum_proof
        {
            0 => self.verify_aggregate_count_and_sum_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentSumQuery::verify_aggregate_count_and_sum_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
