use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::{verify_absent_range_tree, RootHash};
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentSumQuery<'_> {
    /// v0 of [`Self::verify_aggregate_sum_proof`].
    ///
    /// Rebuilds the same `PathQuery` the prover used via
    /// [`Self::aggregate_sum_path_query`] and feeds it through
    /// [`grovedb::GroveDb::verify_aggregate_sum_query`]. Returns
    /// `(root_hash, i64 sum)` — the verified aggregated sum from
    /// one `AggregateSumOnRange` merk traversal.
    ///
    /// Prover/verifier byte-for-byte path query agreement is
    /// load-bearing: both sides share
    /// [`Self::aggregate_sum_path_query`] for that reason.
    #[inline(always)]
    pub(super) fn verify_aggregate_sum_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, i64), Error> {
        let path_query = self.aggregate_sum_path_query(platform_version)?;
        // A range whose tree does not exist (an equality value no document
        // holds) sums to zero (`verify_absent_range_tree`). Edited in place in
        // this shipped generation: the prover is unchanged, and such a proof
        // failed to verify before.
        match GroveDb::verify_aggregate_sum_query(
            proof,
            &path_query,
            &platform_version.drive.grove_version,
        ) {
            Ok(verified) => Ok(verified),
            Err(error) => verify_absent_range_tree(proof, &path_query.path, platform_version)
                .map(|root_hash| (root_hash, 0))
                .ok_or_else(|| Error::GroveDB(Box::new(error))),
        }
    }
}
