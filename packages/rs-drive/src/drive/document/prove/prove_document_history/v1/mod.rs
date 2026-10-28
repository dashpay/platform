use crate::drive::Drive;
use crate::error::Error;
use crate::query::document_history_drive_query::DocumentHistoryDriveQuery;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves the history page and lifecycle in one GroveDB proof.
    pub(super) fn prove_document_history_v1(
        &self,
        query: &DocumentHistoryDriveQuery,
        document_type: DocumentTypeRef,
        transaction: TransactionArg,
        version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        self.fetch_document_history(query, document_type, transaction, version)?;
        self.grove_get_proved_path_query(
            &query.construct_path_query(version)?,
            transaction,
            &mut vec![],
            &version.drive,
        )
    }
}
