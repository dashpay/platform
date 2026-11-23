use crate::drive::Drive;
use crate::error::Error;
use crate::query::document_history_drive_query::{
    invalid, DocumentHistoryDriveQuery, DocumentHistoryDriveQueryExecutionResult,
    SerializedDocumentHistoryDriveQueryExecutionResult,
};
use crate::query::refuse_a_private_document_type;
use crate::verify::grovedb_proof_envelope::{
    require_supported_grovedb_proof_envelope, MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION,
};
use crate::verify::{with_proved_root_on_budget_exceeded, RootHash};
use dpp::data_contract::document_type::accessors::DocumentTypeV0Getters;
use dpp::data_contract::document_type::{
    DecodedValueBudget, DocumentTypeRef, DEFAULT_CLIENT_PAGE_DECODED_VALUE_BUDGET,
};
use dpp::version::PlatformVersion;

// Count-tree offset pagination requires GroveDB proof envelopes from version 1.
const _: () = assert!(MINIMUM_GROVEDB_PROOF_ENVELOPE_VERSION >= 1);

impl Drive {
    /// Verifies the history page proof, lifecycle, and revision headers without
    /// materializing document properties, so callers can authenticate the root
    /// signature before decoding revision bodies.
    pub(super) fn verify_document_history_serialized_v1(
        query: &DocumentHistoryDriveQuery,
        proof: &[u8],
        document_type: DocumentTypeRef,
        version: &PlatformVersion,
    ) -> Result<(RootHash, SerializedDocumentHistoryDriveQueryExecutionResult), Error> {
        refuse_a_private_document_type(document_type).map_err(Error::Query)?;
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
        let serialized = query.decode_result_serialized(rows, document_type, version)?;
        Ok((root, serialized))
    }

    /// Verifies the history page, lifecycle, and revision positions.
    pub(super) fn verify_document_history_v1(
        query: &DocumentHistoryDriveQuery,
        proof: &[u8],
        document_type: DocumentTypeRef,
        version: &PlatformVersion,
    ) -> Result<(RootHash, DocumentHistoryDriveQueryExecutionResult), Error> {
        let (root, serialized) =
            Self::verify_document_history_serialized_v1(query, proof, document_type, version)?;
        let mut budget = DecodedValueBudget::new(DEFAULT_CLIENT_PAGE_DECODED_VALUE_BUDGET);
        let result = serialized
            .into_documents(
                &query.document_id,
                document_type,
                Some(&mut budget),
                version,
            )
            .map_err(|error| with_proved_root_on_budget_exceeded(root, error))?;
        Ok((root, result))
    }
}
