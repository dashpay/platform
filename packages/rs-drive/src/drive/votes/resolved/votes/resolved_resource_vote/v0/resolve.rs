use crate::drive::votes::resolved::vote_polls::resolve::VotePollResolver;
use crate::drive::votes::resolved::votes::resolved_resource_vote::v0::ResolvedResourceVoteV0;
use crate::drive::Drive;
use crate::error::Error;
use dpp::voting::votes::resource_vote::v0::ResourceVoteV0;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

pub(in crate::drive::votes::resolved::votes::resolved_resource_vote) trait ResourceVoteResolverV0 {
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
    ) -> Result<ResolvedResourceVoteV0, Error>;

    fn resolve_owned(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedResourceVoteV0, Error>;
}

impl ResourceVoteResolverV0 for ResourceVoteV0 {
    fn resolve(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedResourceVoteV0, Error> {
        let ResourceVoteV0 {
            vote_poll,
            resource_vote_choice,
        } = self;

        let resolved_vote_poll = vote_poll.resolve(drive, transaction, platform_version)?;

        Ok(ResolvedResourceVoteV0 {
            resolved_vote_poll,
            resource_vote_choice: *resource_vote_choice,
        })
    }

    fn resolve_owned(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedResourceVoteV0, Error> {
        let ResourceVoteV0 {
            vote_poll,
            resource_vote_choice,
        } = self;

        let resolved_vote_poll = vote_poll.resolve_owned(drive, transaction, platform_version)?;

        Ok(ResolvedResourceVoteV0 {
            resolved_vote_poll,
            resource_vote_choice,
        })
    }
}
