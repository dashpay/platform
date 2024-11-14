use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use drive::drive::votes::resolved::vote_polls::ResolvedVotePollWithVotes;
use drive::grovedb::TransactionArg;
use std::collections::BTreeMap;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls
    pub(in crate::execution) fn clean_up_after_vote_polls_end(
        &self,
        vote_polls: &BTreeMap<TimestampMillis, Vec<ResolvedVotePollWithVotes>>,
        clean_up_testnet_corrupted_reference_issue: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .clean_up_after_vote_poll_end
        {
            0 => self.clean_up_after_vote_polls_end_v0(
                vote_polls,
                clean_up_testnet_corrupted_reference_issue,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "clean_up_after_vote_polls_end".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
