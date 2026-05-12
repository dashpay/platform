mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::DriveDocumentCountQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentCountQuery<'_> {
    /// Verifies an `AggregateCountOnRange` proof and returns
    /// `(root_hash, count)`.
    ///
    /// Counterpart to the prover-side
    /// [`execute_aggregate_count_with_proof`](Self::execute_aggregate_count_with_proof):
    /// rebuilds the same `PathQuery` via
    /// [`aggregate_count_path_query`](Self::aggregate_count_path_query)
    /// and calls `GroveDb::verify_aggregate_count_query`. The
    /// caller is responsible for combining the returned `root_hash`
    /// with the surrounding tenderdash signature — see
    /// `rs-drive-proof-verifier`'s `verify_aggregate_count_proof`
    /// wrapper for the canonical composition.
    ///
    /// # Arguments
    /// * `proof` — raw grovedb proof bytes.
    /// * `platform_version` — selects the method version.
    pub fn verify_aggregate_count_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, u64), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_count
            .verify_aggregate_count_proof
        {
            0 => self.verify_aggregate_count_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentCountQuery::verify_aggregate_count_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
