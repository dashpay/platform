mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStoredInfo;
use dpp::voting::vote_polls::yes_no_vote_poll::YesNoVotePoll;
use grovedb::TransactionArg;

impl Drive {
    /// Writes the stored info of a yes/no vote poll, replacing what was there.
    pub fn insert_stored_info_for_yes_no_vote_poll(
        &self,
        vote_poll: &YesNoVotePoll,
        stored_info: YesNoVotePollStoredInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .insert_stored_info_for_yes_no_vote_poll
        {
            0 => self.insert_stored_info_for_yes_no_vote_poll_v0(
                vote_poll,
                stored_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_stored_info_for_yes_no_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// The operations of [`Self::insert_stored_info_for_yes_no_vote_poll`].
    pub fn insert_stored_info_for_yes_no_vote_poll_operations(
        &self,
        vote_poll: &YesNoVotePoll,
        stored_info: YesNoVotePollStoredInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .yes_no
            .insert_stored_info_for_yes_no_vote_poll
        {
            0 => self.insert_stored_info_for_yes_no_vote_poll_operations_v0(
                vote_poll,
                stored_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_stored_info_for_yes_no_vote_poll_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
