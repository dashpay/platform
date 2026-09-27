mod v0;

use crate::drive::document::expiration::DocumentExpirationEntry;
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
    /// Gathers the operations writing a document's entry in the documents expirations tree:
    /// the tree of the documents expiring at `expires_at_ms` if it does not exist yet, and
    /// the entry under it keyed by the document's id. Neither carries storage flags.
    ///
    /// # Parameters
    /// - `document_id`: the document's id; `None` in a worst-case estimate that has no
    ///   document, where the key is estimated at its size.
    /// - `entry`: where the document is.
    /// - `expires_at_ms`: when the document expires.
    /// - `estimated_costs_only_with_layer_info`: set in a dry run.
    /// - `previous_batch_operations`: operations queued earlier in the batch, so the tree of
    ///   one expiry time is queued once.
    /// - `transaction`: the transaction to read in.
    /// - `batch_operations`: receives the operations.
    /// - `platform_version`: selects the method version.
    ///
    /// # Returns
    /// `Ok(())` once the operations are queued.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_document_expiration_operations(
        &self,
        document_id: Option<[u8; 32]>,
        entry: &DocumentExpirationEntry,
        expires_at_ms: TimestampMillis,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        transaction: TransactionArg,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .document
            .expiration
            .add_document_expiration_operations
        {
            0 => self.add_document_expiration_operations_v0(
                document_id,
                entry,
                expires_at_ms,
                estimated_costs_only_with_layer_info,
                previous_batch_operations,
                transaction,
                batch_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_document_expiration_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
