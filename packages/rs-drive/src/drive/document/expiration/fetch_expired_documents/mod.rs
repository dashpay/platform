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

/// What [`Drive::fetch_expired_documents`] found: the expired documents in the order they
/// expired, and every expiry time it read the documents of.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExpiredDocuments {
    /// The expired documents, oldest first, then by id, at most the limit asked for
    pub documents: Vec<ExpiredDocument>,
    /// Every expiry time whose tree was read, oldest first, those holding no document
    /// included (their documents deleted some other way). The tree of each one found empty
    /// once `documents` are gone can be dropped.
    pub expiry_times: Vec<TimestampMillis>,
}

impl Drive {
    /// Reads the documents whose time to live has passed at `block_time_ms` from the documents
    /// expirations tree: oldest expiry time first, at most `limit` documents and at most `limit`
    /// expiry times.
    ///
    /// # Parameters
    /// - `block_time_ms`: documents expiring at or before this time have expired.
    /// - `limit`: the most documents, and the most expiry times, to read.
    /// - `transaction`: the transaction to read in.
    /// - `drive_operations`: receives the costs of the reads.
    /// - `platform_version`: selects the method version.
    ///
    /// # Returns
    /// The expired documents and the expiry times read.
    pub fn fetch_expired_documents(
        &self,
        block_time_ms: TimestampMillis,
        limit: u16,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<ExpiredDocuments, Error> {
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
