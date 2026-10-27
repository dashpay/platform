use crate::drive::Drive;
use crate::error::Error;
use crate::query::document_history_drive_query::{
    invalid, DocumentHistoryDriveQuery, DocumentHistoryDriveQueryExecutionResult,
};
use crate::verify::grovedb_proof_envelope::{
    require_supported_grovedb_proof_envelope, MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION,
};
use crate::verify::RootHash;
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;

// Count-tree offset pagination requires GroveDB proof envelopes from version 1.
const _: () = assert!(MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION >= 1);

impl Drive {
    /// Verifies the history page, lifecycle, and revision positions.
    pub(super) fn verify_document_history_v1(
        query: &DocumentHistoryDriveQuery,
        proof: &[u8],
        document_type: DocumentTypeRef,
        version: &PlatformVersion,
    ) -> Result<(RootHash, DocumentHistoryDriveQueryExecutionResult), Error> {
        require_supported_grovedb_proof_envelope(proof, "document history proof")?;
        if !document_type.documents_keep_history() {
            return Err(invalid("document type does not keep history"));
        }
        let (root, rows) = grovedb::GroveDb::verify_query_with_options(
            proof,
            &query.construct_path_query(version)?,
            grovedb::VerifyOptions {
                // All exact selections and their ancestors are unlimited, so
                // omitted siblings prove absence without explicit absence rows.
                absence_proofs_for_non_existing_searched_keys: false,
                verify_proof_succinctness: true,
                include_empty_trees_in_result: true,
            },
            &version.drive.grove_version,
        )?;
        Ok((root, query.decode_result(rows, document_type, version)?))
    }
}
