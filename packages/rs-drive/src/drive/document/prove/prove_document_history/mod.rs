use crate::drive::document::history::{
    invalid, DocumentHistoryProof, DocumentHistoryQuery, DocumentHistoryV1,
};
mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Dispatches historical document queries using the selected protocol layout.
    pub fn prove_document_history(
        &self,
        query: &DocumentHistoryQuery,
        document_type: Option<dpp::data_contract::document_type::DocumentTypeRef>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<DocumentHistoryV1>, DocumentHistoryProof), Error> {
        match (
            platform_version
                .drive
                .methods
                .document
                .query
                .prove_document_history,
            query,
        ) {
            (
                0,
                DocumentHistoryQuery::V0 {
                    contract_id,
                    document_type_name,
                    document_id,
                    start_at_ms,
                    limit,
                    offset,
                },
            ) => self
                .prove_document_history_v0(
                    *contract_id,
                    document_type_name,
                    *document_id,
                    transaction,
                    *start_at_ms,
                    *limit,
                    *offset,
                    platform_version,
                )
                .map(|proof| (None, DocumentHistoryProof::V0(proof))),
            (1, DocumentHistoryQuery::V1(query)) => self
                .prove_document_history_v1_impl(
                    query,
                    document_type
                        .ok_or_else(|| invalid("history v1 requires its document type"))?,
                    transaction,
                    platform_version,
                )
                .map(|(history, proof)| (Some(history), DocumentHistoryProof::V1(proof))),
            (0 | 1, _) => Err(invalid(
                "document history request shape is unsupported at this protocol version",
            )),
            (version, _) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_document_history".to_owned(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Proves the existence or absence of the specified document's history.
    #[allow(clippy::too_many_arguments)]
    pub fn prove_document_history_legacy(
        &self,
        contract_id: [u8; 32],
        document_type_name: &str,
        document_id: [u8; 32],
        transaction: TransactionArg,
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match self
            .prove_document_history(
                &DocumentHistoryQuery::V0 {
                    contract_id,
                    document_type_name: document_type_name.to_owned(),
                    document_id,
                    start_at_ms,
                    limit,
                    offset,
                },
                None,
                transaction,
                platform_version,
            )?
            .1
        {
            DocumentHistoryProof::V0(proof) => Ok(proof),
            _ => Err(invalid("legacy history requires the timestamp-only layout")),
        }
    }
}
