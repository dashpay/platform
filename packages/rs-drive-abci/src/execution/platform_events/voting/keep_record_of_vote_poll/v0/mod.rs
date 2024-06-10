use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::FinalizedResourceVoteChoicesWithVoterInfo;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::error::drive::DriveError;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Keeps a record of the vote poll after it has finished
    #[inline(always)]
    pub(super) fn keep_record_of_finished_contested_resource_vote_poll_v0(
        &self,
        block_info: &BlockInfo,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        contender_votes: &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        winner_info: ContestedDocumentVotePollWinnerInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let finalized_resource_vote_choices_with_voter_infos = contender_votes
            .iter()
            .map(
                |(resource_vote_choice, voters)| FinalizedResourceVoteChoicesWithVoterInfo {
                    resource_vote_choice: resource_vote_choice.clone(),
                    voters: voters.clone(),
                },
            )
            .collect();
        let stored_info_from_disk = self
            .drive
            .fetch_contested_document_vote_poll_stored_info(
                vote_poll,
                None,
                transaction,
                platform_version,
            )?
            .1
            .ok_or(Error::Drive(drive::error::Error::Drive(
                DriveError::CorruptedDriveState(
                    "there must be a record of the vote poll in the state".to_string(),
                ),
            )))?;

        // We perform an upgrade of the stored version just in case, most of the time this does nothing
        let mut stored_info = stored_info_from_disk.update_to_latest_version(platform_version)?;

        // We need to construct the finalized contested document vote poll stored info
        stored_info.finalize_vote_poll(
            finalized_resource_vote_choices_with_voter_infos,
            *block_info,
            winner_info,
        )?;

        // We reinsert the info
        self.drive
            .insert_stored_info_for_contested_resource_vote_poll(
                vote_poll,
                stored_info,
                transaction,
                platform_version,
            )?;

        Ok(())
    }
}
