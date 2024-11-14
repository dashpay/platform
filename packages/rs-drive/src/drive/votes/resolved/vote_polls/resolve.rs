use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::resolve::ContestedDocumentResourceVotePollResolver;
use crate::drive::votes::resolved::vote_polls::ResolvedVotePoll;
use crate::drive::Drive;
use crate::error::Error;
use dpp::voting::vote_polls::VotePoll;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

/// The vote poll resolver
pub trait VotePollResolver {
    /// Resolves the contested document resource vote poll information.
    ///
    /// This method fetches the contract, document type name, index name, and index values
    /// required to process a contested document resource vote poll.
    ///
    /// # Parameters
    ///
    /// * `drive`: A reference to the `Drive` object used for database interactions.
    /// * `transaction`: The transaction argument used to ensure consistency during the resolve operation.
    /// * `platform_version`: The platform version to ensure compatibility.
    ///
    /// # Returns
    ///
    /// * `Ok(ContestedDocumentResourceVotePollWithContractInfo)` - The resolved information needed for the vote poll.
    /// * `Err(Error)` - An error if the resolution process fails.
    ///
    /// # Errors
    ///
    /// This method returns an `Error` variant if there is an issue resolving the contested document resource vote poll
    /// information. The specific error depends on the underlying problem encountered during resolution.
    fn resolve(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedVotePoll, Error>;

    /// Resolve owned
    fn resolve_owned(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedVotePoll, Error>;
}

impl VotePollResolver for VotePoll {
    fn resolve(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedVotePoll, Error> {
        match self {
            VotePoll::ContestedDocumentResourceVotePoll(contested_document_resource_vote_poll) => {
                Ok(
                    ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                        contested_document_resource_vote_poll.resolve(
                            drive,
                            transaction,
                            platform_version,
                        )?,
                    ),
                )
            }
        }
    }

    fn resolve_owned(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedVotePoll, Error> {
        match self {
            VotePoll::ContestedDocumentResourceVotePoll(contested_document_resource_vote_poll) => {
                Ok(
                    ResolvedVotePoll::ContestedDocumentResourceVotePollWithContractInfo(
                        contested_document_resource_vote_poll.resolve(
                            drive,
                            transaction,
                            platform_version,
                        )?,
                    ),
                )
            }
        }
    }
}
