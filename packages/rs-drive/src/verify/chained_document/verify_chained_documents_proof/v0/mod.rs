use crate::error::proof::ProofError;
use crate::error::Error;
use crate::query::drive_chained_document_query::{
    ChainedDocumentsResult, DriveChainedDocumentQuery,
};
use crate::verify::RootHash;
use dpp::version::PlatformVersion;

impl DriveChainedDocumentQuery<'_> {
    /// v0 of the chained proof verification — see the versioned wrapper
    /// for the trust model.
    #[inline(always)]
    pub(super) fn verify_chained_documents_proof_v0(
        &self,
        inner_proof: &[u8],
        outer_proof: Option<&[u8]>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, ChainedDocumentsResult), Error> {
        self.validate(platform_version)?;

        // Inner half: the standard document proof of the query the
        // client built itself. For indexOnly types this dispatches to
        // the terminal/synthesis verifier, so the returned projections
        // carry the join property positionally.
        let (inner_root_hash, inner_documents) =
            self.inner.verify_proof(inner_proof, platform_version)?;

        let join_values = self.join_values(&inner_documents)?;

        if join_values.is_empty() {
            if outer_proof.is_some() {
                return Err(Error::Proof(ProofError::CorruptedProof(
                    "chained proof carries an outer proof although the proven inner page \
                     is empty: there is no derived query it could prove"
                        .to_string(),
                )));
            }
            return Ok((
                inner_root_hash,
                ChainedDocumentsResult {
                    inner_documents,
                    outer_documents: Vec::new(),
                },
            ));
        }

        let Some(outer_proof) = outer_proof else {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "chained proof is missing the outer proof although the proven inner page \
                 is non-empty"
                    .to_string(),
            )));
        };

        // Outer half: verified against the query DERIVED from the
        // proven join values — never against anything the server sent.
        let outer_query = self.derive_outer_query(&join_values);
        let (outer_root_hash, outer_documents) =
            outer_query.verify_proof(outer_proof, platform_version)?;

        // The composed statement is only sound at ONE state root. The
        // caller binds this shared root to the quorum-signed app hash.
        if outer_root_hash != inner_root_hash {
            return Err(Error::Proof(ProofError::CorruptedProof(
                "chained proof halves verify to different root hashes: the halves do not \
                 describe one state"
                    .to_string(),
            )));
        }

        // Exact set equality in both directions + first-appearance
        // ordering, shared with the server-side assembly.
        let outer_documents = self.assemble_outer_documents(&join_values, outer_documents)?;

        Ok((
            inner_root_hash,
            ChainedDocumentsResult {
                inner_documents,
                outer_documents,
            },
        ))
    }
}
