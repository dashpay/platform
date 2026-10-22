mod v0;
mod v1;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;
use crate::query::document_history_drive_query::DocumentHistoryDriveQuery;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves timestamp-keyed history with an exclusive time bound and an optional offset.
    ///
    /// # Parameters
    /// `start_at_ms` excludes matching timestamps; `offset` skips entries after
    /// that bound. The platform version must select the timestamp-keyed reader.
    ///
    /// # Returns
    /// The history result, or an error for an unsupported reader generation.
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
        match platform_version
            .drive
            .methods
            .document
            .query
            .prove_document_history
        {
            0 => self.prove_document_history_v0(
                contract_id,
                document_type_name,
                document_id,
                transaction,
                start_at_ms,
                limit,
                offset,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_document_history_legacy".to_owned(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Proves a page of a historical document's history in the layout the
    /// protocol version stores.
    pub fn prove_document_history(
        &self,
        query: &DocumentHistoryDriveQuery,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .query
            .prove_document_history
        {
            0 => {
                let (start_at_ms, limit) = query.legacy_read()?;
                self.prove_document_history_v0(
                    query.contract_id,
                    &query.document_type_name,
                    query.document_id,
                    transaction,
                    start_at_ms,
                    limit,
                    None,
                    platform_version,
                )
            }
            1 => {
                self.prove_document_history_v1(query, document_type, transaction, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_document_history".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
