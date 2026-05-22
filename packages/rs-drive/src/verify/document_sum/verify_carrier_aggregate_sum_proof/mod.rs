mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// Verifies a **carrier** `AggregateSumOnRange` proof and returns
    /// `(root_hash, per_key_sums)` — one `(in_key, i64)` pair per
    /// resolved In branch. Order depends on `left_to_right`:
    /// `true` returns serialized lex-ascending, `false` returns
    /// serialized lex-descending.
    ///
    /// Sum-side analog of count's
    /// [`crate::query::DriveDocumentCountQuery::verify_carrier_aggregate_count_proof`].
    /// Counterpart to the prover-side
    /// [`execute_carrier_aggregate_sum_with_proof`](DriveDocumentSumQuery::execute_carrier_aggregate_sum_with_proof):
    /// rebuilds the same `PathQuery` via
    /// [`carrier_aggregate_sum_path_query`](DriveDocumentSumQuery::carrier_aggregate_sum_path_query)
    /// and calls
    /// [`grovedb::GroveDb::verify_aggregate_sum_query_per_key`] (once
    /// the grovedb sister PR exposes it). The caller is responsible
    /// for combining the returned `root_hash` with the surrounding
    /// tenderdash signature — see `rs-drive-proof-verifier`'s wrapper
    /// for the canonical composition.
    ///
    /// # Arguments
    /// * `proof` — raw grovedb proof bytes.
    /// * `limit` — per-branch carrier walk cap; must match the
    ///   prover's `SizedQuery::limit`.
    /// * `left_to_right` — proof-shaping bit. Must match the value
    ///   the prover passed to
    ///   [`Self::execute_carrier_aggregate_sum_with_proof`]. Mismatch
    ///   produces different `PathQuery` bytes and the tenderdash
    ///   root check fails.
    /// * `platform_version` — selects the method version.
    #[allow(clippy::type_complexity)]
    pub fn verify_carrier_aggregate_sum_proof(
        &self,
        proof: &[u8],
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, i64)>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_sum
            .verify_carrier_aggregate_sum_proof
        {
            0 => self.verify_carrier_aggregate_sum_proof_v0(
                proof,
                limit,
                left_to_right,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentSumQuery::verify_carrier_aggregate_sum_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
