mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_document_sum_query::DriveDocumentSumQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentSumQuery<'_> {
    /// Verifies a grovedb `AggregateSumOnRange` proof (grovedb PR
    /// #670) and returns `(root_hash, i64 sum)`. Counterpart to the
    /// prover-side
    /// [`Self::execute_aggregate_sum_with_proof`].
    /// Calls `GroveDb::verify_aggregate_sum_query`.
    ///
    /// Tree-type restriction: the chosen index must declare
    /// `rangeSummable: true` so the terminator's value tree is at
    /// least a `ProvableSumTree`; the grovedb merk gate rejects
    /// lighter sum-bearing variants on the aggregate primitive.
    /// Same path-query byte-equality contract as every other paired
    /// prover/verifier in this surface — both sides share
    /// [`Self::aggregate_sum_path_query`].
    pub fn verify_aggregate_sum_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, i64), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_sum
            .verify_aggregate_sum_proof
        {
            0 => self.verify_aggregate_sum_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentSumQuery::verify_aggregate_sum_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
