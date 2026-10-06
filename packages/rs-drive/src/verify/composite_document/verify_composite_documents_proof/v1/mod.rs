use crate::error::Error;
use crate::query::{CompositeDocumentsResult, DriveDocumentQuery};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveDocumentQuery<'_> {
    /// v1 of the composite proof verification, selected from protocol
    /// version 14: v0, reading every item variant as a document, so a
    /// composition over the sum-bearing items of a `documentsSummable` type or
    /// a `summable` indexOnly index verifies, where v0 reads them as counts no
    /// component selected and refuses the proof.
    #[inline(always)]
    pub(super) fn verify_composite_documents_proof_v1(
        &self,
        proof: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, CompositeDocumentsResult), Error> {
        self.verify_composite_documents_proof_classifying(proof, true, platform_version)
    }
}
