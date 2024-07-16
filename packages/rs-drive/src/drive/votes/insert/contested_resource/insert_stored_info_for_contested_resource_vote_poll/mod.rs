mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::fees::op::LowLevelDriveOperation;
use dpp::version::PlatformVersion;
use dpp::voting::vote_info_storage::contested_document_vote_poll_stored_info::ContestedDocumentVotePollStoredInfo;
use grovedb::TransactionArg;

impl Drive {
    /// Inserts a record of a finished vote poll that can later be queried
    pub fn insert_stored_info_for_contested_resource_vote_poll(
        &self,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        finalized_contested_document_vote_poll_stored_info: ContestedDocumentVotePollStoredInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .contested_resource_insert
            .insert_stored_info_for_contested_resource_vote_poll
        {
            0 => self.insert_stored_info_for_contested_resource_vote_poll_v0(
                vote_poll,
                finalized_contested_document_vote_poll_stored_info,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_stored_info_for_contested_resource_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Returns the operations of inserting a record of a finished vote poll that can later be queried
    pub fn insert_stored_info_for_contested_resource_vote_poll_operations(
        &self,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        finalized_contested_document_vote_poll_stored_info: ContestedDocumentVotePollStoredInfo,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .contested_resource_insert
            .insert_stored_info_for_contested_resource_vote_poll
        {
            0 => self.insert_stored_info_for_contested_resource_vote_poll_operations_v0(
                vote_poll,
                finalized_contested_document_vote_poll_stored_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "insert_stored_info_for_contested_resource_vote_poll_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
