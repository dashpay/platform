use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    #[inline(always)]
    pub(super) fn clean_up_after_contested_resources_vote_polls_end_v0(
        &self,
        vote_polls: Vec<(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut operations = vec![];
        self.drive
            .remove_contested_resource_vote_poll_end_date_query_operations(
                vote_polls.as_slice(),
                &mut operations,
                transaction,
                platform_version,
            )?;

        self.drive
            .remove_contested_resource_vote_poll_votes_operations(
                vote_polls.as_slice(),
                &mut operations,
                transaction,
                platform_version,
            )?;

        self.drive.apply_batch_low_level_drive_operations(
            None,
            transaction,
            operations,
            &mut vec![],
            &platform_version.drive,
        )?;

        Ok(())
    }
}
