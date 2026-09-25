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
    /// Gathers the operation removing a document's entry from the documents expirations tree.
    /// The tree of its expiry time stays even when this empties it: the cleanup after a block's
    /// state transitions drops the empty trees it reaches.
    ///
    /// # Parameters
    /// - `document_id`: the document's id.
    /// - `expires_at_ms`: when the document expires, the key of the tree holding its entry.
    /// - `entry_value_size`: the size of the entry's value, for a dry run.
    /// - `estimated_costs_only_with_layer_info`: set in a dry run.
    /// - `transaction`: the transaction to read in.
    /// - `batch_operations`: receives the operation.
    /// - `platform_version`: selects the method version.
    ///
    /// # Returns
    /// `Ok(())` once the operation is queued.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn remove_document_expiration_operations(
        &self,
        document_id: [u8; 32],
        expires_at_ms: TimestampMillis,
        entry_value_size: u32,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
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
