use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::RootHash;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentSumQuery<'_> {
    /// v0 of [`Self::verify_carrier_aggregate_count_and_sum_proof`].
    ///
    /// Rebuilds the same PCPS-leaf `PathQuery` the prover used via
    /// [`Self::carrier_aggregate_count_and_sum_path_query`] and feeds
    /// it through
    /// [`grovedb::GroveDb::verify_aggregate_count_and_sum_query_per_key`].
    /// Returns one `(in_key, u64, i128)` triple per resolved outer In
    /// branch — the same shape the leaf entry point
    /// `verify_aggregate_count_and_sum_query` returns per key, just
    /// fanned out across the carrier's outer dimension.
    ///
    /// Tree-type restriction: the terminator MUST be a
    /// `ProvableCountProvableSumTree` (PCPS). Lighter sum-bearing
    /// variants (`ProvableSumTree`, `ProvableCountSumTree`) are
    /// rejected at the grovedb-side classification gate. The drive
    /// path-query builder already enforces this via the picker's
    /// `range_countable && range_summable` requirement; this v0 just
    /// surfaces whatever the merk verifier returns.
    ///
    /// As with the sum-only carrier, prover/verifier path-query
    /// byte-equality is load-bearing — both sides share
    /// [`Self::carrier_aggregate_count_and_sum_path_query`].
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    pub(super) fn verify_carrier_aggregate_count_and_sum_proof_v0(
        &self,
        proof: &[u8],
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, u64, i128)>), Error> {
        let path_query = self.carrier_aggregate_count_and_sum_path_query(
            limit,
            left_to_right,
            platform_version,
        )?;
        // A type that keeps its sums in 128 bits is verified through the verifier that
        // returns one: the 64-bit one refuses its trees. No type keeps them so before
        // protocol version 15 (`DocumentTypeV2Getters::uses_big_sums`), so earlier
        // versions verify as they did. Both branches return the sum at 128 bits. Edited in
        // place in this shipped generation: it returned the sum at 64 bits and refused a
        // wider one; a verifier runs on the client and decides nothing in a block, and every
        // sum it verified before is returned as the same number.
        if self.document_type.uses_big_sums() {
            let (root_hash, entries) = GroveDb::verify_aggregate_count_and_big_sum_query_per_key(
                proof,
                &path_query,
                &platform_version.drive.grove_version,
            )
            .map_err(|e| Error::GroveDB(Box::new(e)))?;
            return Ok((root_hash, entries));
        }
        let (root_hash, entries) = GroveDb::verify_aggregate_count_and_sum_query_per_key(
            proof,
            &path_query,
            &platform_version.drive.grove_version,
        )
        .map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok((
            root_hash,
            entries
                .into_iter()
                .map(|(key, count, sum)| (key, count, i128::from(sum)))
                .collect(),
        ))
    }
}
