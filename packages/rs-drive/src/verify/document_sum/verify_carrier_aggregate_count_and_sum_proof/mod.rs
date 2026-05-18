mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// Verifies a combined PCPS carrier proof
    /// (`AggregateCountAndSumOnRange` on a carrier subquery) and
    /// returns `(root_hash, per_key_count_sums)` — one
    /// `(in_key, u64 count, i64 sum)` triple per resolved In branch.
    ///
    /// Combined-axis analog of
    /// [`Self::verify_carrier_aggregate_sum_proof`]. Requires the
    /// covering index to declare BOTH `rangeCountable: true` AND
    /// `rangeSummable: true` so the terminator's value tree is a
    /// `ProvableCountProvableSumTree`. Counterpart to the prover-side
    /// [`execute_carrier_aggregate_count_and_sum_with_proof`](DriveDocumentSumQuery::execute_carrier_aggregate_count_and_sum_with_proof).
    /// Calls `GroveDb::verify_aggregate_count_and_sum_query_per_key`
    /// (grovedb develop (PR #670 merged; head `e98bab5f` as of this PR)).
    #[allow(clippy::type_complexity)]
    pub fn verify_carrier_aggregate_count_and_sum_proof(
        &self,
        proof: &[u8],
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, u64, i64)>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_sum
            .verify_carrier_aggregate_count_and_sum_proof
        {
            0 => self.verify_carrier_aggregate_count_and_sum_proof_v0(
                proof,
                limit,
                left_to_right,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentSumQuery::verify_carrier_aggregate_count_and_sum_proof"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
