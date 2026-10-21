use super::DocumentHistoryProof;
use crate::drive::Drive;
use crate::error::Error;
use crate::query::document_history_drive_query::{
    corrupt, invalid, DocumentHistoryDriveQuery, DocumentHistoryDriveQueryExecutionResult,
};
use crate::verify::grovedb_proof_envelope::require_supported_grovedb_proof_envelope;
use crate::verify::RootHash;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies both proofs of the per-type history tree, their common root,
    /// and the revision positions.
    pub(super) fn verify_document_history_v1(
        query: &DocumentHistoryDriveQuery,
        proof: &[u8],
        document_type: DocumentTypeRef,
        version: &PlatformVersion,
    ) -> Result<(RootHash, DocumentHistoryDriveQueryExecutionResult), Error> {
        let proof = DocumentHistoryProof::from_bytes(proof)?;
        require_supported_grovedb_proof_envelope(&proof.metadata_proof, "history metadata proof")?;
        if let Some(entries_proof) = &proof.entries_proof {
            require_supported_grovedb_proof_envelope(entries_proof, "history entries proof")?;
        }
        if !document_type.documents_keep_history() {
            return Err(invalid("document type does not keep history"));
        }
        let entries_query = query.construct_path_query(version)?;
        let (root, metadata) = grovedb::GroveDb::verify_query_with_options(
            &proof.metadata_proof,
            &query.metadata_path_query(version)?,
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
        Ok((
            root,
            DocumentHistoryDriveQueryExecutionResult {
                entries,
                lifecycle: Some(lifecycle),
            },
        ))
    }
}
