mod v0;
mod v1;

#[cfg(all(test, feature = "server", feature = "verify"))]
mod tests;

use crate::drive::Drive;
use dpp::document::Document;
use std::collections::BTreeMap;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::document_history_drive_query::{
    DocumentHistoryDriveQuery, DocumentHistoryDriveQueryExecutionResult,
};
use crate::verify::grovedb_proof_envelope::require_supported_grovedb_proof_envelope;
use crate::verify::RootHash;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;

impl Drive {
    /// Verifies timestamp-keyed history with an exclusive time bound and an optional offset.
    ///
    /// # Parameters
    /// `start_at_ms` excludes matching timestamps; `offset` skips entries after
    /// that bound. The platform version must select the timestamp-keyed reader.
    ///
    /// # Returns
    /// The history result, or an error for an unsupported reader generation.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_document_history_legacy(
        proof: &[u8],
        contract_id: [u8; 32],
        document_type_name: &str,
        document_type: DocumentTypeRef,
        document_id: [u8; 32],
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<BTreeMap<u64, Document>>), Error> {
        require_supported_grovedb_proof_envelope(proof, "document history proof")?;
        match platform_version
            .drive
            .methods
            .verify
            .document
            .verify_document_history
        {
            0 => Self::verify_document_history_v0(
                proof,
                contract_id,
                document_type_name,
                document_type,
                document_id,
                start_at_ms,
                limit,
                offset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_document_history_legacy".to_owned(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Verifies a proved page of a historical document's history in the
    /// layout the protocol version stores.
    pub fn verify_document_history(
        query: &DocumentHistoryDriveQuery,
        proof: &[u8],
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, DocumentHistoryDriveQueryExecutionResult), Error> {
        match platform_version
            .drive
            .methods
            .verify
            .document
            .verify_document_history
        {
            0 => {
                require_supported_grovedb_proof_envelope(proof, "document history proof")?;
                let (start_at_ms, limit) = query.legacy_read()?;
                let (root, revisions) = Drive::verify_document_history_v0(
                    proof,
                    query.contract_id,
                    &query.document_type_name,
                    document_type,
                    query.document_id,
                    start_at_ms,
                    limit,
                    None,
                    platform_version,
                )?;
                let history = DocumentHistoryDriveQueryExecutionResult::from_legacy(
                    revisions.unwrap_or_default(),
                )?;
                Ok((root, history))
            }
            1 => Drive::verify_document_history_v1(query, proof, document_type, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_document_history".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
