mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::DriveDocumentCountQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentCountQuery<'_> {
    /// Verifies a **carrier** `AggregateCountOnRange` proof and
    /// returns `(root_hash, per_key_counts)` — one `(in_key, u64)`
    /// pair per resolved In branch. Order depends on
    /// `left_to_right`: `true` returns serialized lex-ascending,
    /// `false` returns serialized lex-descending.
    ///
    /// Counterpart to the prover-side
    /// [`execute_carrier_aggregate_count_with_proof`](Self::execute_carrier_aggregate_count_with_proof):
    /// rebuilds the same `PathQuery` via
    /// [`carrier_aggregate_count_path_query`](Self::carrier_aggregate_count_path_query)
    /// and calls
    /// [`grovedb::GroveDb::verify_aggregate_count_query_per_key`].
    /// The caller is responsible for combining the returned
    /// `root_hash` with the surrounding tenderdash signature — see
    /// `rs-drive-proof-verifier`'s wrapper for the canonical
    /// composition.
    ///
    /// # Arguments
    /// * `proof` — raw grovedb proof bytes.
    /// * `limit` — per-branch carrier walk cap; must match the
    ///   prover's `SizedQuery::limit`.
    /// * `left_to_right` — proof-shaping bit. Must match the value
    ///   the prover passed to
    ///   [`Self::execute_carrier_aggregate_count_with_proof`]
    ///   (typically derived from the request's first
    ///   `order_by_clauses` entry's `ascending`). The verifier
    ///   constructs the outer `Query` via
    ///   `Query::new_with_direction(left_to_right)`; a mismatch
    ///   produces different `PathQuery` bytes and the tenderdash
    ///   root check fails.
    /// * `platform_version` — selects the method version.
    ///
    /// The `Vec<(Vec<u8>, u64)>` payload mirrors grovedb's per-key
    /// carrier shape — see the v0 inner method for the rationale.
    #[allow(clippy::type_complexity)]
    pub fn verify_carrier_aggregate_count_proof(
        &self,
        proof: &[u8],
        limit: Option<u16>,
        left_to_right: bool,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, u64)>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_count
            .verify_carrier_aggregate_count_proof
        {
            0 => self.verify_carrier_aggregate_count_proof_v0(
                proof,
                limit,
                left_to_right,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentCountQuery::verify_carrier_aggregate_count_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
