mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Gathers the operations removing a document's entry from the documents expirations
    /// tree, and the tree of its expiry time with it when that was its last entry: no tree of
    /// an expiry time is ever left empty, so every one the cleanup reads holds a document.
    /// Entries the rest of the batch removes or adds under the same time count.
    ///
    /// # Parameters
    /// - `document_id`: the document's id.
    /// - `expires_at_ms`: when the document expires, the key of the tree holding its entry.
    /// - `entry_value_size`: the size of the entry's value, for a dry run.
    /// - `estimated_costs_only_with_layer_info`: set in a dry run, which prices the removal
    ///   of the tree too.
    /// - `check_existing_operations`: the operations of the rest of the batch.
    /// - `transaction`: the transaction to read in.
    /// - `batch_operations`: receives the operations.
    /// - `platform_version`: selects the method version.
    ///
    /// # Returns
    /// `Ok(())` once the operations are queued.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn remove_document_expiration_operations(
        &self,
        document_id: [u8; 32],
        expires_at_ms: TimestampMillis,
        entry_value_size: u32,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        check_existing_operations: &Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .expiration
            .remove_document_expiration_operations
        {
            0 => self.remove_document_expiration_operations_v0(
                document_id,
                expires_at_ms,
                entry_value_size,
                estimated_costs_only_with_layer_info,
                check_existing_operations,
                transaction,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_document_expiration_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
