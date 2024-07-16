mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// We remove votes for identities when those identities have been disabled. Currently there is
    /// no way to "disable" identities except for masternodes being removed from the list
    pub fn remove_all_votes_given_by_identities(
        &self,
        identity_ids_as_byte_arrays: Vec<Vec<u8>>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .cleanup
            .remove_specific_vote_references_given_by_identity
        {
            0 => self.remove_all_votes_given_by_identities_v0(
                identity_ids_as_byte_arrays,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_all_votes_given_by_identities".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
