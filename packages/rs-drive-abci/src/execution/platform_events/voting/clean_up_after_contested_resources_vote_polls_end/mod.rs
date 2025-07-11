use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::identifier::Identifier;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

mod v0;
mod v1;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    // TODO: Use type or struct
    #[allow(clippy::type_complexity)]
    pub(in crate::execution) fn clean_up_after_contested_resources_vote_polls_end(
        &self,
        block_info: &BlockInfo,
        vote_polls: Vec<(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )>,
        clean_up_testnet_corrupted_reference_issue: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .clean_up_after_contested_resources_vote_poll_end
        {
            0 => self.clean_up_after_contested_resources_vote_polls_end_v0(
                vote_polls,
                clean_up_testnet_corrupted_reference_issue,
                transaction,
                platform_version,
            ),
            1 => self.clean_up_after_contested_resources_vote_polls_end_v1(
                block_info,
                vote_polls,
                clean_up_testnet_corrupted_reference_issue,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "clean_up_after_contested_resources_vote_polls_end".to_string(),
                known_versions: vec![0, 1],
                received: version,
            })),
        }
    }
}
