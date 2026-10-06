use crate::error::Error;
use crate::query::drive_document_count_query::counter_sum_as_document_count;
use crate::query::DriveDocumentCountQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentCountQuery<'_> {
    /// v0 of [`Self::verify_aggregate_count_proof`].
    ///
    /// Rebuilds the same `PathQuery` the prover used via
    /// [`Self::aggregate_count_path_query`] and feeds it through
    /// `GroveDb::verify_aggregate_count_query`. The merk-level
    /// `AggregateCountOnRange` primitive returns a single `u64`
    /// directly (capped only by the merk tree size, not `u16::MAX`).
    ///
    /// Prover/verifier byte-for-byte path query agreement is
    /// load-bearing: any drift in serialization of the path bytes,
    /// the range query item, or the limit field would break the
    /// merk-root recomputation. Both sides share
    /// [`Self::aggregate_count_path_query`] for that reason.
    #[inline(always)]
    pub(super) fn verify_aggregate_count_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, u64), Error> {
        // A `summableOffCountIndex` index's documents are its range sums
        // (`counter_sums_query`), proved by the sum surface. Edited in place in
        // this shipped generation: only meta-schema v3 (protocol version 14)
        // admits such an index, so every earlier version verifies as before.
        if let Some(sums) = self.counter_sums_query() {
            let (root_hash, sum) = sums.verify_aggregate_sum_proof(proof, platform_version)?;
            return Ok((root_hash, counter_sum_as_document_count(sum)));
        }
        let path_query = self.aggregate_count_path_query(platform_version)?;
        let (root_hash, count) = GroveDb::verify_aggregate_count_query(
            proof,
            &path_query,
            &platform_version.drive.grove_version,
        )
        .map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok((root_hash, count))
    }
}
