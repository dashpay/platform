mod v0;

use crate::drive::Drive;
use std::collections::BTreeMap;

use crate::error::drive::DriveError;
use crate::error::Error;

use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use dpp::platform_value::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the identities voting for contenders.
    pub fn fetch_identities_voting_for_contenders(
        &self,
        contested_document_resource_vote_poll_with_contract_info: &ContestedDocumentResourceVotePollWithContractInfo,
        fetch_contenders: Vec<Identifier>,
        also_fetch_abstaining_and_locked_votes: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<ResourceVoteChoice, Vec<Identifier>>, Error> {
        match platform_version
            .drive
            .methods
            .vote
            .fetch
            .fetch_identities_voting_for_contenders
        {
            0 => self.fetch_identities_voting_for_contenders_v0(
                contested_document_resource_vote_poll_with_contract_info,
                fetch_contenders,
                also_fetch_abstaining_and_locked_votes,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identities_voting_for_contenders".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
