mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::DriveDocumentCountQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentCountQuery<'_> {
    /// Verifies a **carrier** `AggregateCountOnRange` proof and
    /// returns `(root_hash, per_key_counts)` — one `(in_key, u64)`
    /// pair per resolved In branch in serialized lex-asc order.
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
    /// * `platform_version` — selects the method version.
    pub fn verify_carrier_aggregate_count_proof(
        &self,
        proof: &[u8],
        limit: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<(Vec<u8>, u64)>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_count
            .verify_carrier_aggregate_count_proof
        {
            0 => self.verify_carrier_aggregate_count_proof_v0(proof, limit, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentCountQuery::verify_carrier_aggregate_count_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
