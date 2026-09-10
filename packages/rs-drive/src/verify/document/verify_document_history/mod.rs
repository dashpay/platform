use crate::drive::document::history::{
    invalid, DocumentHistoryProof, DocumentHistoryQuery, DocumentHistoryResult,
};
mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::verify::RootHash;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::Document;
use dpp::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// Verifies history using the selected protocol layout and matching proof shape.
    pub fn verify_document_history(
        proof: &DocumentHistoryProof,
        query: &DocumentHistoryQuery,
        document_type: DocumentTypeRef,
        platform_version: &PlatformVersion,
    ) -> Result<(RootHash, Option<DocumentHistoryResult>), Error> {
        match (
            platform_version
                .drive
                .methods
                .verify
                .document
                .verify_document_history,
            query,
            proof,
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
                DocumentHistoryProof::V0(proof),
            ) => Self::verify_document_history_v0(
                proof,
                *contract_id,
                document_type_name,
                document_type,
                *document_id,
                *start_at_ms,
                *limit,
                *offset,
                platform_version,
            )
            .map(|(root, history)| (root, history.map(DocumentHistoryResult::V0))),
            (1, DocumentHistoryQuery::V1(query), DocumentHistoryProof::V1(proof)) => {
                Self::verify_document_history_v1_impl(query, proof, document_type, platform_version)
                    .map(|(root, history)| (root, Some(DocumentHistoryResult::V1(history))))
            }
            (0 | 1, _, _) => Err(invalid(
                "document history request or proof shape is unsupported at this protocol version",
            )),
            (version, _, _) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "verify_document_history".to_owned(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }

    /// Verifies that the document's history is included in the proof.
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
        let (root, history) = Self::verify_document_history(
            &DocumentHistoryProof::V0(proof.to_vec()),
            &DocumentHistoryQuery::V0 {
                contract_id,
                document_type_name: document_type_name.to_owned(),
                document_id,
                start_at_ms,
                limit,
                offset,
            },
            document_type,
            platform_version,
        )?;
        match history {
            None => Ok((root, None)),
            Some(DocumentHistoryResult::V0(history)) => Ok((root, Some(history))),
            _ => Err(invalid("legacy history requires the timestamp-only layout")),
        }
    }
}
