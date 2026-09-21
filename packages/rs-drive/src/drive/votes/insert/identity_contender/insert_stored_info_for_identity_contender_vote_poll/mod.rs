mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::identity_contender_vote_poll_stored_info::IdentityContenderVotePollStoredInfo;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use grovedb::TransactionArg;

impl Drive {
    /// Replaces the stored info of an identity contender vote poll and applies the operation.
    pub fn insert_stored_info_for_identity_contender_vote_poll(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        stored_info: IdentityContenderVotePollStoredInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .insert_stored_info_for_identity_contender_vote_poll
        {
            0 => self.insert_stored_info_for_identity_contender_vote_poll_v0(
                vote_poll,
                stored_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_stored_info_for_identity_contender_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operation replacing the stored info of an identity contender vote poll.
    pub fn insert_stored_info_for_identity_contender_vote_poll_operations(
        &self,
        vote_poll: &IdentityContenderVotePoll,
        stored_info: IdentityContenderVotePollStoredInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .identity_contender
            .insert_stored_info_for_identity_contender_vote_poll
        {
            0 => self.insert_stored_info_for_identity_contender_vote_poll_operations_v0(
                vote_poll,
                stored_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_stored_info_for_identity_contender_vote_poll_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
