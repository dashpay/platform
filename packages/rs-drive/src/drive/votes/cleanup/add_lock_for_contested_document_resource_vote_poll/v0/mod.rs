use crate::drive::votes::paths::VotePollPaths;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

impl Drive {
    /// We remove vote_choices for an identity when that identity is somehow disabled. Currently there is
    /// no way to "disable" identities except for masternodes being removed from the list
    pub(super) fn add_lock_for_contested_document_resource_vote_poll_v0(
        &self,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let path = vote_poll.contenders_path(platform_version)?;

        Ok(())
    }
}
