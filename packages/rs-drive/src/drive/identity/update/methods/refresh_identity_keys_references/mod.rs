mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;

use dpp::prelude::Revision;

use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};

use dpp::fee::default_costs::CachedEpochIndexFeeVersions;
use dpp::identity::IdentityPublicKey;
use platform_version::version::drive_versions::DriveVersion;
use std::collections::HashMap;

impl Drive {
    /// Updates the revision for a specific identity. This function is version controlled.
    pub fn refresh_identity_keys_references(
        &self,
        identity_id: [u8; 32],
        key: &IdentityPublicKey,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .refresh_identity_keys_references
        {
            0 => self.refresh_identity_keys_references_v0(
                identity_id,
                key,
                drive_operations,
                &platform_version.drive,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "refresh_identity_keys_references_v0".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
