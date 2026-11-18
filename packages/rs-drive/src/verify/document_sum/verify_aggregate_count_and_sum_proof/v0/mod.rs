use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::query::sum_width::sum_in_64_bits;
use crate::verify::RootHash;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentSumQuery<'_> {
    /// v0 of [`Self::verify_aggregate_count_and_sum_proof`].
    ///
    /// Rebuilds the same PCPS-leaf `PathQuery` the prover used via
    /// [`Self::aggregate_count_and_sum_path_query`] and feeds it
    /// through
    /// [`grovedb::GroveDb::verify_aggregate_count_and_sum_query`].
    /// Returns `(root_hash, u64 count, i64 sum)` — the single
    /// `(count, sum)` pair the verifier extracts from one
    /// `AggregateCountAndSumOnRange` traversal of the PCPS
    /// terminator. The client divides `sum / count` to get the
    /// verified average.
    ///
    /// Tree-type restriction: the terminator MUST be a
    /// `ProvableCountProvableSumTree` (PCPS). Lighter sum-bearing
    /// variants (`SumTree`, `ProvableSumTree`, `CountSumTree`,
    /// `ProvableCountSumTree`) are rejected at the grovedb-side
    /// classification gate. The drive path-query builder already
    /// enforces this via the picker's
    /// `range_countable && range_summable` requirement; this v0
    /// just surfaces whatever the merk verifier returns.
    ///
    /// As with every other paired prover/verifier in this surface,
    /// path-query byte-equality is load-bearing — both sides share
    /// [`Self::aggregate_count_and_sum_path_query`].
    #[inline(always)]
    pub(super) fn verify_aggregate_count_and_sum_proof_v0(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, u64, i64), Error> {
        let path_query = self.aggregate_count_and_sum_path_query(platform_version)?;
        // A type that keeps its sums in 128 bits is verified through the verifier that
        // returns one: the 64-bit one refuses its trees. No type keeps them so before
        // protocol version 15 (`DocumentTypeV2Getters::uses_big_sums`), so earlier
        // versions verify as they did.
        if self.document_type.uses_big_sums() {
            let (root_hash, count, sum) = GroveDb::verify_aggregate_count_and_big_sum_query(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )
            .map_err(|e| Error::GroveDB(Box::new(e)))?;
            return Ok((root_hash, count, sum_in_64_bits(sum)?));
        }
        let (root_hash, count, sum) = GroveDb::verify_aggregate_count_and_sum_query(
            proof,
            &path_query,
            &platform_version.drive.grove_version,
        )
        .map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok((root_hash, count, sum))
    }
}
