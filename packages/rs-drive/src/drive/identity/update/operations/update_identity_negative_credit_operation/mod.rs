mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;

impl Drive {
    /// We can set an identities negative credit balance
    pub fn update_identity_negative_credit_operation(
        &self,
        identity_id: [u8; 32],
        negative_credit: Credits,
        platform_version: &PlatformVersion,
    ) -> Result<LowLevelDriveOperation, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .update
            .update_identity_negative_credit_operation
        {
            0 => {
                Ok(self.update_identity_negative_credit_operation_v0(identity_id, negative_credit))
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "update_identity_revision".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
