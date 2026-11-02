mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::yes_no_vote_poll_stored_info::YesNoVotePollStoredInfo;
use grovedb::TransactionArg;

impl Drive {
    /// Writes the stored info of a yes/no vote poll, replacing what was there, under the poll
    /// it holds.
    ///
    /// # Parameters
    /// - `stored_info`: the poll's stored info.
    /// - `transaction`: the transaction to write in.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// `Ok(())` once the stored info is written.
    pub fn insert_stored_info_for_yes_no_vote_poll(
        &self,
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
            Some(0) => self.insert_stored_info_for_yes_no_vote_poll_v0(
                stored_info,
                transaction,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_stored_info_for_yes_no_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "insert_stored_info_for_yes_no_vote_poll".to_string(),
                known_versions: vec![0],
            })),
        }
    }

    /// The operations of [`Self::insert_stored_info_for_yes_no_vote_poll`].
    ///
    /// # Parameters
    /// - `stored_info`: the poll's stored info.
    /// - `platform_version`: the platform version.
    ///
    /// # Returns
    /// The operations that write it.
    pub fn insert_stored_info_for_yes_no_vote_poll_operations(
        &self,
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
            Some(0) => self.insert_stored_info_for_yes_no_vote_poll_operations_v0(
                stored_info,
                platform_version,
            ),
            Some(version) => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_stored_info_for_yes_no_vote_poll_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
            None => Err(Error::Drive(DriveError::VersionNotActive {
                method: "insert_stored_info_for_yes_no_vote_poll_operations".to_string(),
                known_versions: vec![0],
            })),
        }
    }
}
