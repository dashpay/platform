mod v0;

use crate::drive::Drive;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Adding a lock to a vote poll disables voting for the vote poll
    pub fn add_lock_for_contested_document_resource_vote_poll(
        &self,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive
            .methods
            .vote
            .cleanup
            .add_lock_for_contested_document_resource_vote_poll
        {
            0 => self.add_lock_for_contested_document_resource_vote_poll_v0(
                vote_poll,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "add_lock_for_contested_document_resource_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
