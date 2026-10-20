use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use drive::grovedb::TransactionArg;
use drive::query::vote_poll_vote_state_query::FinalizedContestedDocumentVotePollDriveQueryExecutionResult;

mod v0;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Tally the votes for a contested resource vote poll
    pub(in crate::execution) fn tally_votes_for_contested_document_resource_vote_poll(
        &self,
        contested_document_resource_vote_poll: ContestedDocumentResourceVotePoll,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FinalizedContestedDocumentVotePollDriveQueryExecutionResult, Error> {
        match platform_version
            .drive_abci
            .methods
            .voting
            .tally_votes_for_contested_document_resource_vote_poll
        {
            0 => self.tally_votes_for_contested_document_resource_vote_poll_v0(
                contested_document_resource_vote_poll,
                transaction,
                platform_version,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "tally_votes_for_contested_resource_vote_poll".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use dpp::version::PLATFORM_VERSIONS;

    /// Wherever a join is checked against `max_contenders_per_contest` (document create state
    /// validation 2 on), the tally reaches that many contenders, so the cleanup built from it
    /// leaves none behind, and its query, two results a contender plus three, fits the u16
    /// query limit without saturating
    #[test]
    fn should_tally_every_contender_a_contest_accepts() {
        for platform_version in PLATFORM_VERSIONS {
            if platform_version
                .drive_abci
                .validation_and_processing
                .state_transitions
                .batch_state_transition
                .document_create_transition_state_validation
                < 2
            {
                continue;
            }
            let tallied = platform_version
                .drive_abci
                .validation_and_processing
                .event_constants
                .maximum_contenders_to_consider;
            assert!(
                tallied >= platform_version.system_limits.max_contenders_per_contest,
                "protocol version {}",
                platform_version.protocol_version
            );
            assert!(
                tallied as u32 * 2 + 3 <= u16::MAX as u32,
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }
}
