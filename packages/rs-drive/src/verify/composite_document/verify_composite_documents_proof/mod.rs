mod v0;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::drive_composite_document_query::{
    CompositeDocumentsResult, DriveCompositeDocumentQuery,
};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveCompositeDocumentQuery<'_> {
    /// Verifies a composite query's single merged proof and returns
    /// `(root_hash, result)`.
    ///
    /// The verifier trusts nothing about the derivation, and needs
    /// nothing beyond the proof itself: a BOOTSTRAP subset pass runs the
    /// page query (and every sub-query that feeds a later binding) alone
    /// against the merged proof to extract candidate values; every
    /// sub-query is derived from those exactly as the prover derived it
    /// from its materialization, the merged query is rebuilt, and the
    /// AUTHORITATIVE full pass verifies the whole composition — grovedb
    /// enforces every component's lifted per-instance limit and range
    /// completeness. The proven results are then routed back to their
    /// components: an entry no derivation asked for is an invalid proof,
    /// so is a by-id join missing a referenced document (a
    /// `permanentDocument` reference cannot dangle), and so is any
    /// divergence between the values the proven page derives and the
    /// candidates the query was built from. A proof covering only the
    /// page (an old node serving the plain query) fails the full pass
    /// whenever a sub-query derived anything.
    ///
    /// One proof means one root by construction; the caller combines the
    /// returned root hash with the surrounding tenderdash signature — see
    /// `rs-drive-proof-verifier` for the canonical composition.
    pub fn verify_composite_documents_proof(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, CompositeDocumentsResult), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .composite_document
            .verify_composite_documents_proof
        {
            0 => self.verify_composite_documents_proof_v0(proof, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "DriveCompositeDocumentQuery::verify_composite_documents_proof".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
