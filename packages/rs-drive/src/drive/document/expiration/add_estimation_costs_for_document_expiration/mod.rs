mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::prelude::TimestampMillis;
use dpp::version::drive_versions::DriveVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::EstimatedLayerInformation;
use std::collections::HashMap;

impl Drive {
    /// Adds the layers a write to the documents expirations tree goes through to a dry run's
    /// estimated layer information: the root, `Misc`, the expirations tree and the tree of the
    /// documents expiring at `expires_at_ms`.
    ///
    /// # Parameters
    /// - `expires_at_ms`: the time key of the entry written or removed.
    /// - `entry_value_size`: the size of the entry's value
    ///   (`DocumentExpirationEntry::serialized_size`).
    /// - `estimated_costs_only_with_layer_info`: the dry run's layer information.
    /// - `drive_version`: selects the method version.
    ///
    /// # Returns
    /// `Ok(())` once the layers are added.
    pub(crate) fn add_estimation_costs_for_document_expiration(
        expires_at_ms: TimestampMillis,
        entry_value_size: u32,
        estimated_costs_only_with_layer_info: &mut HashMap<KeyInfoPath, EstimatedLayerInformation>,
        drive_version: &DriveVersion,
    ) -> Result<(), Error> {
        match drive_version
            .methods
            .document
            .expiration
            .add_estimation_costs_for_document_expiration
        {
            0 => {
                Self::add_estimation_costs_for_document_expiration_v0(
                    expires_at_ms,
                    entry_value_size,
                    estimated_costs_only_with_layer_info,
                );
                Ok(())
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_estimation_costs_for_document_expiration".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
