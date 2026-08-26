mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::DriveDocumentHavingQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentHavingQuery<'_> {
    /// Verifies a grovedb indexed-axis range proof and returns
    /// `(root_hash, entries)`.
    ///
    /// Counterpart to the prover-side
    /// [`execute_range_with_proof`](Self::execute_range_with_proof).
    /// Both sides derive the proved subtree from
    /// [`indexed_property_name_tree_path`](Self::indexed_property_name_tree_path)
    /// and the same bounded axis `PathQuery` from
    /// [`AxisRangeBounds::i128_bounds`](crate::query::drive_document_having_query::AxisRangeBounds::i128_bounds),
    /// so the verifier cannot drift from the prover on *which* bound over
    /// *which* tree it is checking.
    ///
    /// The returned entries are in axis order in the walk direction,
    /// exactly as the unproven
    /// [`execute_range_no_proof`](Self::execute_range_no_proof) would
    /// return them. The caller combines `root_hash` with the surrounding
    /// tenderdash signature — see `rs-drive-proof-verifier` for the
    /// canonical composition.
    ///
    /// # Arguments
    /// * `proof` — raw grovedb proof bytes.
    /// * `platform_version` — selects the method version.
    pub fn verify_having_range_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Vec<crate::query::RankedEntry>), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_ranked
            .verify_having_range_proof
        {
            0 => self.verify_having_range_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentHavingQuery::verify_having_range_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
