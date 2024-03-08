mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::fee::fee_result::FeeResult;

use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// We remove votes for an identity when that identity is somehow disabled. Currently there is
    /// no way to "disable" identities except for masternodes being removed from the list
    pub fn remove_votes_for_identity(
        &self,
        identity_id: Identifier,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .cleanup
            .remove_votes_for_identity
        {
            0 => self.remove_votes_for_identity_v0(identity_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_votes_for_identity".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
