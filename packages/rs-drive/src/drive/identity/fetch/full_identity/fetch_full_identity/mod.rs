mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::epoch::Epoch;
use dpp::fee::fee_result::FeeResult;

use dpp::identity::Identity;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches an identity with all its information and
    /// the cost it took from storage.
    pub fn fetch_full_identity_with_costs(
        &self,
        identity_id: [u8; 32],
        epoch: &Epoch,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<Identity>, FeeResult), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .full_identity
            .fetch_full_identity
        {
            Some(0) => self.fetch_full_identity_with_costs_v0(
                identity_id,
                epoch,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_full_identity_with_costs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_full_identity_with_costs".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// Fetches an identity with all its information from storage.
    pub fn fetch_full_identity(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .full_identity
            .fetch_full_identity
        {
            Some(0) => self.fetch_full_identity_v0(identity_id, transaction, platform_version),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_full_identity".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_full_identity".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// Given an identity, fetches the identity with its flags from storage.
    pub fn fetch_full_identity_operations(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Identity>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .full_identity
            .fetch_full_identity
        {
            Some(0) => self.fetch_full_identity_operations_v0(
                identity_id,
                transaction,
                drive_operations,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_full_identity_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "fetch_full_identity_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
