use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::document::Document;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

mod v0;

impl Drive {
    /// Fetch withdrawal documents by it's status ordered by updated_at ascending with limit 100
    pub fn fetch_up_to_100_oldest_withdrawal_documents_by_status(
        &self,
        status: u8,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .document
            .fetch_up_to_100_oldest_withdrawal_documents_by_status
        {
            0 => self.fetch_up_to_100_oldest_withdrawal_documents_by_status_v0(
                status,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_up_to_100_oldest_withdrawal_documents_by_status".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
