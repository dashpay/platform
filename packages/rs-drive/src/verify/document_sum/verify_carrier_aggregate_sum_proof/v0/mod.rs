use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::{verify_absent_range_tree, RootHash};
use dpp::version::PlatformVersion;
use grovedb::GroveDb;

impl DriveDocumentSumQuery<'_> {
    /// v0 of [`Self::verify_carrier_aggregate_sum_proof`].
    ///
    /// Rebuilds the same `PathQuery` the prover used via
    /// [`Self::carrier_aggregate_sum_path_query`] and feeds it through
    /// [`grovedb::GroveDb::verify_aggregate_sum_query_per_key`]. The
    /// merk-level carrier composition emits one aggregate `i64` per
    /// resolved outer In key (each independently cryptographically
    /// committed via `node_hash_with_sum` — sum analog of count's
    /// `node_hash_with_count` from
    /// [grovedb PR #670](https://github.com/dashpay/grovedb/pull/670)).
    ///
    /// Prover/verifier byte-for-byte path query agreement is
    /// load-bearing: any drift in serialization of the In-key bytes,
    /// the subquery path, the range query item, or the limit field
    /// would break the merk-root recomputation. Both sides share
    /// [`Self::carrier_aggregate_sum_path_query`] for that reason.
    ///
    /// The `Vec<(Vec<u8>, i64)>` payload is the grovedb-native
    /// per-key carrier shape (one serialized In-key + its aggregate
    /// `i64`); naming it via a `type` alias would only rebrand the
    /// same nested tuple without making the call site clearer.
    #[inline(always)]
    #[allow(clippy::type_complexity)]
    pub(super) fn verify_carrier_aggregate_sum_proof_v0(
        &self,
        proof: &[u8],
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, i64)>), Error> {
        let path_query =
            self.carrier_aggregate_sum_path_query(limit, left_to_right, platform_version)?;
        // A carrier below an equality value no document holds has no branch
        // (`verify_absent_range_tree`). Edited in place in this shipped
        // generation: the prover is unchanged, and such a proof failed to
        // verify before.
        match GroveDb::verify_aggregate_sum_query_per_key(
            proof,
            &path_query,
            &platform_version.drive.grove_version,
        ) {
            Ok(verified) => Ok(verified),
            Err(error) => verify_absent_range_tree(proof, &path_query.path, platform_version)
                .map(|root_hash| (root_hash, Vec::new()))
                .ok_or_else(|| Error::GroveDB(Box::new(error))),
        }
    }
}
