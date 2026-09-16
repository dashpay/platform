//! Verifying a proved history page (server and verify builds).

use super::{corrupt, invalid, DocumentHistoryProofV1, DocumentHistoryQueryV1, DocumentHistoryV1};
use crate::drive::Drive;
use crate::error::Error;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;

#[cfg(any(feature = "server", feature = "verify"))]
impl Drive {
    /// Verifies both proofs, their common root, and the revision positions.
    pub(crate) fn verify_document_history_v1_impl(
        query: &DocumentHistoryQueryV1,
        proof: &DocumentHistoryProofV1,
        document_type: DocumentTypeRef,
        version: &PlatformVersion,
    ) -> Result<([u8; 32], DocumentHistoryV1), Error> {
        proof.validate_envelopes()?;
        if !document_type.documents_keep_history() {
            return Err(invalid("document type does not keep history"));
        }
        let entries_query = query.entries_query(version)?;
        let (root, metadata) = grovedb::GroveDb::verify_query_with_options(
            &proof.metadata_proof,
            &query.metadata_query(version)?,
            grovedb::VerifyOptions {
                // The lifecycle is derived from which metadata keys are missing,
                // so every queried key must come back as a proven element or a
                // proven absence rather than being silently left out.
                absence_proofs_for_non_existing_searched_keys: true,
                verify_proof_succinctness: true,
                include_empty_trees_in_result: true,
            },
            &version.drive.grove_version,
        )?;
        let (lifecycle, present) = query.lifecycle(metadata, document_type, version)?;
        let entries = match (&proof.entries_proof, present) {
            (None, false) => vec![],
            (Some(bytes), true) => {
                let (entries_root, entries) = grovedb::GroveDb::verify_query(
                    bytes,
                    &entries_query,
                    &version.drive.grove_version,
                )?;
                if root != entries_root {
                    return Err(corrupt("history proofs have different roots"));
                }
                let entries = entries
                    .into_iter()
                    .map(|(path, key, element)| {
                        if path != entries_query.path {
                            return Err(corrupt("history proof returned a different path"));
                        }
                        Ok((
                            key,
                            element.ok_or_else(|| {
                                corrupt("history entry proof contains an absent element")
                            })?,
                        ))
                    })
                    .collect::<Result<Vec<_>, Error>>()?;
                query.decode_entries(entries, document_type, version)?
            }
            _ => {
                return Err(corrupt(
                    "entries proof presence contradicts history metadata",
                ))
            }
        };
        Ok((root, DocumentHistoryV1 { entries, lifecycle }))
    }
}

#[cfg(any(feature = "server", feature = "verify"))]
impl Drive {
    /// Verifies a proved page of a historical document's history and lifecycle.
    pub fn verify_document_history_v1(
        query: &DocumentHistoryQueryV1,
        proof: &DocumentHistoryProofV1,
        document_type: DocumentTypeRef,
        version: &PlatformVersion,
    ) -> Result<([u8; 32], DocumentHistoryV1), Error> {
        Self::verify_document_history(proof, query, document_type, version)
    }
}
