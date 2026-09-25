mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

/// What one run of [`Drive::remove_expired_documents`] did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RemovedExpiredDocuments {
    /// Expired documents deleted
    pub deleted_documents: u16,
    /// Entries removed without a document to delete: the document was gone or no longer
    /// matched its entry. Never expected; each is logged.
    pub orphaned_entries: u16,
    /// Trees of an expiry time dropped because nothing was left under them
    pub dropped_expiry_times: u16,
}

impl Drive {
    /// Deletes up to `limit` documents whose time to live has passed at the block's time,
    /// oldest first, then drops the trees of the expiry times it emptied. Nobody pays: the
    /// documents prepaid their deletion when they were created, and the fee results are
    /// discarded. Each deletion is its own batch in `transaction`, so every index tree the
    /// next one reads is in its final state; the entries of the deleted documents go with
    /// them.
    ///
    /// # Parameters
    /// - `block_info`: the block the cleanup runs at the end of.
    /// - `limit`: the most documents to delete (`max_document_expirations_per_block`).
    /// - `transaction`: the block's transaction.
    /// - `platform_version`: selects the method version.
    ///
    /// # Returns
    /// What was deleted and dropped.
    pub fn remove_expired_documents(
        &self,
        block_info: &BlockInfo,
        limit: u16,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<RemovedExpiredDocuments, Error> {
        match platform_version
            .drive
            .methods
            .document
            .expiration
            .remove_expired_documents
        {
            0 => self.remove_expired_documents_v0(block_info, limit, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_expired_documents".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
