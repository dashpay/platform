use super::v1::{earliest_contender, EndedContest};
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::FinalizedContender;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use drive::error::drive::DriveError;
use drive::error::Error as DriveCrateError;
use drive::grovedb::error::Error as GroveError;
use drive::grovedb::TransactionArg;
use grovedb_merk::error::Error as MerkError;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls as generation 1 does, except that a contest ends whatever
    /// its contenders hold: a contender whose document does not decode cannot win, and a
    /// winner whose document the state refuses leaves the contest with no winner (see
    /// [`Self::award_contest_winner_v2`]).
    #[inline(always)]
    pub(super) fn check_for_ended_vote_polls_v2(
        &self,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.end_due_vote_polls_v1(
            last_committed_platform_state,
            block_platform_state,
            block_info,
            Self::award_contest_winner_v2,
            transaction,
            platform_version,
        )
    }

    /// Picks the winner of an ended contest and awards it the document, as generation 1 does,
    /// with two differences that make the outcome depend on the contest alone:
    ///
    /// - A contender whose stored document does not decode with the contest's document type
    ///   cannot win. The winner is picked from the contenders that decode with the highest
    ///   tally among them, the earliest of them on a tie, and the lock tally is compared with
    ///   its tally. With none that decodes, the contest has no winner.
    /// - When another document already holds a value of one of the winner's document's unique
    ///   indexes, the contest has no winner and nothing is written. When the state refuses the
    ///   winner's document as the award writes it (see [`award_was_refused`]), what the award
    ///   wrote is undone and the contest has no winner.
    ///
    /// A contest with no winner is recorded and cleaned up like one with no contenders: its
    /// contenders' documents are removed and its prefunded balance goes to the epoch's
    /// processing fees.
    pub(super) fn award_contest_winner_v2(
        &self,
        block_info: &BlockInfo,
        contest: EndedContest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ContestedDocumentVotePollWinnerInfo, Error> {
        let EndedContest {
            sorted_contenders,
            locked_vote_tally,
            locking_allowed,
            document_type,
            vote_poll,
        } = contest;
        // The contenders come sorted by tally from the highest: decode them in that order and
        // stop at the first tally below that of a contender that decoded.
        let mut eligible_contenders: Vec<FinalizedContender> = vec![];
        for contender in sorted_contenders {
            if eligible_contenders
                .first()
                .is_some_and(|eligible| eligible.final_vote_tally > contender.final_vote_tally)
            {
                break;
            }
            if let Ok(contender) = FinalizedContender::try_from_contender_with_serialized_document(
                contender,
                document_type,
                platform_version,
            ) {
                eligible_contenders.push(contender);
            }
        }
        let Some(top_contender) = earliest_contender(eligible_contenders) else {
            return Ok(ContestedDocumentVotePollWinnerInfo::NoWinner);
        };
        // If the lock is tied with the top contender the top contender gets it
        if locking_allowed && locked_vote_tally > top_contender.final_vote_tally {
            return Ok(ContestedDocumentVotePollWinnerInfo::Locked);
        }
        let contender_id = top_contender.identity_id;
        // A value of one of the document's unique indexes already held by another document
        // refuses the award before it writes, by the check a document creation runs. The
        // document is checked as stored: its owner is the identity it is awarded to.
        if !self
            .drive
            .validate_moderated_document_uniqueness(
                vote_poll.contract.as_ref(),
                document_type,
                &top_contender.document,
                None,
                transaction,
                platform_version,
            )?
            .is_valid()
        {
            return Ok(ContestedDocumentVotePollWinnerInfo::NoWinner);
        }
        // The award writes in one or more batches: a refusal in a later one would keep what
        // the earlier ones wrote, so the award is undone to a savepoint. A savepoint left after
        // an award that went through is inert: the block's transaction is committed or dropped
        // with it. No contest ends in the genesis block, whose rounds rewind to the one
        // savepoint `init_chain` sets.
        if let Some(transaction) = transaction {
            transaction.set_savepoint();
        }
        match self.award_document_to_winner(
            block_info,
            top_contender,
            vote_poll,
            transaction,
            platform_version,
        ) {
            Ok(()) => Ok(ContestedDocumentVotePollWinnerInfo::WonByIdentity(
                contender_id,
            )),
            Err(error) if award_was_refused(&error) => {
                if let Some(transaction) = transaction {
                    if let Err(rollback_error) = transaction.rollback_to_savepoint() {
                        // Only the rollback's error is returned: log the award's error with it
                        tracing::error!(
                            ?error,
                            ?rollback_error,
                            "could not undo a contest award the state refused"
                        );
                        return Err(Error::Drive(DriveCrateError::GroveDB(Box::new(
                            GroveError::StorageError(rollback_error),
                        ))));
                    }
                }
                Ok(ContestedDocumentVotePollWinnerInfo::NoWinner)
            }
            Err(error) => Err(error),
        }
    }
}

/// Whether the state refused the document awarded to a contest's winner, for what the state
/// already holds or what the document adds to it: the document's id already taken, or a sum
/// tree aggregate the document would take past its integer's range. Merk raises the second as
/// its `Overflow` error when it computes the aggregates of the tree it commits; its other
/// `Overflow` errors come from cost estimation, which an applied write does not run. Each
/// follows from the state every node holds, so every node refuses the same award. Any other
/// error, a storage failure among them, is returned as before.
fn award_was_refused(error: &Error) -> bool {
    match error {
        Error::Drive(DriveCrateError::Drive(DriveError::CorruptedDocumentAlreadyExists(_))) => true,
        Error::Drive(DriveCrateError::GroveDB(error)) => {
            matches!(
                error.as_ref(),
                GroveError::MerkError(MerkError::Overflow(_))
            )
        }
        _ => false,
    }
}
