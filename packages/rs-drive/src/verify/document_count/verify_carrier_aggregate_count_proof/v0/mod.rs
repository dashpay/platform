use crate::error::Error;
use crate::query::DriveDocumentCountQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentCountQuery<'_> {
    /// v0 of [`Self::verify_carrier_aggregate_count_proof`].
    ///
    /// Rebuilds the same `PathQuery` the prover used via
    /// [`Self::carrier_aggregate_count_path_query`] and feeds it
    /// through
    /// [`grovedb::GroveDb::verify_aggregate_count_query_per_key`].
    /// The merk-level carrier composition emits one aggregate
    /// `u64` per outer In key (each independently cryptographically
    /// committed via `node_hash_with_count` — see
    /// [grovedb PR #663](https://github.com/dashpay/grovedb/pull/663)).
    ///
    /// Prover/verifier byte-for-byte path query agreement is
    /// load-bearing: any drift in serialization of the In-key
    /// bytes, the subquery path, the range query item, or the
    /// limit field would break the merk-root recomputation. Both
    /// sides share [`Self::carrier_aggregate_count_path_query`]
    /// for that reason.
    ///
    /// The `Vec<(Vec<u8>, u64)>` payload is the grovedb-native
    /// per-key carrier shape (one serialized In-key + its
    /// aggregate `u64`); naming it via a `type` alias would only
    /// rebrand the same nested tuple without making the call site
    /// clearer.
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    pub(super) fn verify_carrier_aggregate_count_proof_v0(
        &self,
        proof: &[u8],
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, u64)>), Error> {
        let path_query =
            self.carrier_aggregate_count_path_query(limit, left_to_right, platform_version)?;
        let (root_hash, entries) = GroveDb::verify_aggregate_count_query_per_key(
            proof,
            &path_query,
            &platform_version.drive.grove_version,
        )
        .map_err(|e| Error::GroveDB(Box::new(e)))?;
        Ok((root_hash, entries))
    }
}
