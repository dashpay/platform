mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// A document whose time to live has passed, as its expirations tree entry records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpiredDocument {
    /// When the document expired
    pub expires_at_ms: TimestampMillis,
    /// The document's id
    pub document_id: Identifier,
    /// The document's contract
    pub contract_id: Identifier,
    /// The name of the document's type
    pub document_type_name: String,
}

impl Drive {
    /// Reads the documents whose time to live has passed at `block_time_ms` from the documents
    /// expirations tree: oldest expiry time first, then by id, at most `limit` of them. Every
    /// tree of an expiry time holds at least one entry (the last entry removed takes its tree
    /// with it), so the read visits at most `limit` trees.
    ///
    /// # Parameters
    /// - `block_time_ms`: documents expiring at or before this time have expired.
    /// - `limit`: the most documents to read.
    /// - `transaction`: the transaction to read in.
    /// - `drive_operations`: receives the costs of the read.
    /// - `platform_version`: selects the method version.
    ///
    /// # Returns
    /// The expired documents, oldest first.
    pub fn fetch_expired_documents(
        &self,
        block_time_ms: TimestampMillis,
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<ExpiredDocument>, Error> {
        match platform_version
            .drive
            .methods
            .document
            .expiration
            .fetch_expired_documents
        {
            0 => self.fetch_expired_documents_v0(
                block_time_ms,
                limit,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_expired_documents".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
