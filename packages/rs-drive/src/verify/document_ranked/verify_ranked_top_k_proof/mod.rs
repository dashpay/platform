mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::DriveDocumentRankedQuery;
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentRankedQuery<'_> {
    /// Verifies a grovedb indexed-axis paginated top-k proof and returns
    /// `(root_hash, page)`.
    ///
    /// Counterpart to the prover-side
    /// [`execute_top_k_with_proof`](Self::execute_top_k_with_proof).
    /// Both sides derive the proved subtree from
    /// [`indexed_property_name_tree_path`](Self::indexed_property_name_tree_path),
    /// so the verifier cannot drift from the prover on *which* ranking
    /// it is checking.
    ///
    /// The returned entries are in ranking order, exactly as the
    /// unproven [`execute_top_k_no_proof`](Self::execute_top_k_no_proof)
    /// would return them, and
    /// [`RankedPage::skipped`](crate::query::RankedPage::skipped)
    /// carries the attested starting rank so entry `i` is the group at
    /// rank `skipped + i`. The caller combines `root_hash` with the
    /// surrounding tenderdash signature — see `rs-drive-proof-verifier`
    /// for the canonical composition.
    ///
    /// # Arguments
    /// * `proof` — raw grovedb proof bytes.
    /// * `platform_version` — selects the method version.
    pub fn verify_ranked_top_k_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, crate::query::RankedPage), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document_ranked
            .verify_ranked_top_k_proof
        {
            0 => self.verify_ranked_top_k_proof_v0(proof),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveDocumentRankedQuery::verify_ranked_top_k_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
