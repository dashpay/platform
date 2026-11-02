mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the existence or absence of the specified document's history. A private
    /// document type keeps none: it is refused.
    #[allow(clippy::too_many_arguments)]
    pub fn prove_document_history(
        &self,
        contract_id: [u8; 32],
        document_type: DocumentTypeRef,
        document_id: [u8; 32],
        transaction: TransactionArg,
        start_at_ms: u64,
        limit: Option<u16>,
        offset: Option<u16>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .prove_document_history
        {
            0 => self.prove_document_history_v0(
                contract_id,
                document_type,
                document_id,
                transaction,
                start_at_ms,
                limit,
                offset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_document_history".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
