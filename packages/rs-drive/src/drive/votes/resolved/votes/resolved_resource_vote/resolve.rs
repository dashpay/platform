use crate::drive::votes::resolved::votes::resolved_resource_vote::v0::resolve::ResourceVoteResolverV0;
use crate::drive::votes::resolved::votes::resolved_resource_vote::ResolvedResourceVote;
use crate::drive::Drive;
use crate::error::Error;
use dpp::voting::votes::resource_vote::ResourceVote;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;

pub trait ResourceVoteResolver {
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
    ) -> Result<ResolvedResourceVote, Error>;

    fn resolve_owned(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedResourceVote, Error>;
}

impl ResourceVoteResolver for ResourceVote {
    fn resolve(
        &self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedResourceVote, Error> {
        match self {
            ResourceVote::V0(resource_vote) => Ok(ResolvedResourceVote::V0(
                resource_vote.resolve(drive, transaction, platform_version)?,
            )),
        }
    }

    fn resolve_owned(
        self,
        drive: &Drive,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ResolvedResourceVote, Error> {
        match self {
            ResourceVote::V0(resource_vote) => Ok(ResolvedResourceVote::V0(
                resource_vote.resolve_owned(drive, transaction, platform_version)?,
            )),
        }
    }
}
