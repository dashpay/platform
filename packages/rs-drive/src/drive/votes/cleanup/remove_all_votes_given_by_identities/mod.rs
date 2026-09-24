mod v0;
mod v1;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use dpp::dashcore::Network;
use dpp::prelude::BlockHeight;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// We remove votes for identities when those identities have been disabled. Currently there is
    /// no way to "disable" identities except for masternodes being removed from the list
    pub fn remove_all_votes_given_by_identities(
        &self,
        identity_ids_as_byte_arrays: Vec<Vec<u8>>,
        block_height: BlockHeight,
        network: Network,
        chain_id: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .cleanup
            .remove_all_votes_given_by_identities
        {
            0 => self.remove_all_votes_given_by_identities_v0(
                identity_ids_as_byte_arrays,
                block_height,
                network,
                chain_id,
                transaction,
                platform_version,
            ),
            1 => self.remove_all_votes_given_by_identities_v1(
                identity_ids_as_byte_arrays,
                block_height,
                network,
                chain_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "remove_all_votes_given_by_identities".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
