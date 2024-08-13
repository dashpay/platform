mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::block::epoch::Epoch;
use dpp::identity::IdentityPublicKey;
use std::collections::HashMap;

impl Drive {
    /// Refreshes the identity key reference
    pub fn refresh_identity_key_reference_operations(
        &self,
        identity_id: [u8; 32],
        key: &IdentityPublicKey,
        epoch: &Epoch,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .refresh_identity_key_reference_operations
        {
            0 => self.refresh_identity_key_reference_operations_v0(
                identity_id,
                key,
                epoch,
                estimated_costs_only_with_layer_info,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "refresh_identity_key_reference_v0".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
