use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::document::Document;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

mod v0;

impl Drive {
    /// Fetch withdrawal documents by its status ordered by updated_at ascending
    pub fn fetch_oldest_withdrawal_documents_by_status(
        &self,
        status: u8,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<Document>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .document
            .fetch_oldest_withdrawal_documents_by_status
        {
            0 => self.fetch_oldest_withdrawal_documents_by_status_v0(
                status,
                limit,
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

    pub fn fetch_oldest_withdrawal_documents(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<u8, Vec<Document>>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .withdrawals
            .document
            .fetch_oldest_withdrawal_documents_by_status
        {
            0 => self.fetch_oldest_withdrawal_documents_v0(transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_oldest_withdrawal_documents".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
