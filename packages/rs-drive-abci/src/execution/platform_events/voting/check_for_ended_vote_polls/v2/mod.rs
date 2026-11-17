use super::v1::{earliest_contender, EndedContest};
use crate::error::Error;
use crate::execution::types::state_transition_execution_context::StateTransitionExecutionContext;
use crate::execution::validation::state_transition::batch::validate_summed_value_within_type_total;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::Value;
use dpp::prelude::TimestampMillis;
use dpp::version::{DefaultForPlatformVersion, PlatformVersion};
use dpp::voting::contender_structs::FinalizedContender;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dpp::voting::vote_polls::VotePoll;
use drive::drive::votes::paths::VOTE_POLLS_LEFT_OPEN_END_DATE;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::TransactionArg;
use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use std::collections::BTreeMap;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks for ended vote polls as generation 1 does, except that a contest whose winner's
    /// document cannot be decoded or written is left open (see
    /// [`Self::award_contest_winner_v2`]): nothing of it is awarded, recorded or cleaned up,
    /// and it moves to the end date no block time reaches
    /// ([`VOTE_POLLS_LEFT_OPEN_END_DATE`]), where a later protocol version finds it.
    #[inline(always)]
    pub(super) fn check_for_ended_vote_polls_v2(
        &self,
        last_committed_platform_state: &PlatformState,
        block_platform_state: &PlatformState,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let contests_left_open = self.end_due_vote_polls_v1(
            last_committed_platform_state,
            block_platform_state,
            block_info,
            Self::award_contest_winner_v2,
            transaction,
            platform_version,
        )?;
        // After the clean-ups of the polls that ended, each in a batch of its own: its read of
        // an end date sees what the ones before it left there.
        for (end_date, vote_poll) in contests_left_open {
            self.move_contest_to_the_left_open_end_date(
                block_info,
                end_date,
                &vote_poll,
                transaction,
                platform_version,
            )?;
        }
        Ok(())
    }

    /// Picks the winner of an ended contest and awards it the document, as generation 1 does,
    /// except that nothing is awarded to anyone, and the contest is left as it is (`None`),
    /// when the winner cannot be awarded its document:
    ///
    /// - A contender with the highest tally holds a document that does not decode with the
    ///   contest's document type. Which of them is the earliest is then not known, and no other
    ///   contender is awarded in its place.
    /// - The state would refuse the winner's document as a document created outside a contest
    ///   (see [`Self::state_admits_awarded_document`]).
    ///
    /// The lock tally is compared first, as it needs no document: a contest the lock wins is
    /// locked whatever its contenders hold.
    pub(super) fn award_contest_winner_v2(
        &self,
        block_info: &BlockInfo,
        contest: EndedContest,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ContestedDocumentVotePollWinnerInfo>, Error> {
        let EndedContest {
            sorted_contenders,
            locked_vote_tally,
            locking_allowed,
            document_type,
            vote_poll,
        } = contest;
        // The contenders come sorted by tally from the highest
        let Some(highest_vote_tally) = sorted_contenders
            .first()
            .map(|max_voted_contender| max_voted_contender.final_vote_tally)
        else {
            return Ok(Some(ContestedDocumentVotePollWinnerInfo::NoWinner));
        };
        // If the lock is tied with the top contenders the earliest of them gets it
        if locking_allowed && locked_vote_tally > highest_vote_tally {
            return Ok(Some(ContestedDocumentVotePollWinnerInfo::Locked));
        }
        let mut top_contenders: Vec<FinalizedContender> = vec![];
        for contender in sorted_contenders
            .into_iter()
            .take_while(|contender| contender.final_vote_tally == highest_vote_tally)
        {
            let Ok(contender) = FinalizedContender::try_from_contender_with_serialized_document(
                contender,
                document_type,
                platform_version,
            ) else {
                return Ok(None);
            };
            top_contenders.push(contender);
        }
        let Some(top_contender) = earliest_contender(top_contenders) else {
            return Ok(Some(ContestedDocumentVotePollWinnerInfo::NoWinner));
        };
        if !self.state_admits_awarded_document(
            block_info,
            &top_contender.document,
            document_type,
            vote_poll,
            transaction,
            platform_version,
        )? {
            return Ok(None);
        }
        let contender_id = top_contender.identity_id;
        self.award_document_to_winner(
            block_info,
            top_contender,
            vote_poll,
            transaction,
            platform_version,
        )?;
        Ok(Some(ContestedDocumentVotePollWinnerInfo::WonByIdentity(
            contender_id,
        )))
    }

    /// Whether the state admits the document a contest awards, by what a document created
    /// outside a contest is held to in the state. A contender's create was checked against
    /// the contest alone, and other documents were written since:
    ///
    /// - no other document holds a value of one of its unique indexes. The document is
    ///   checked as stored: its owner is the identity it is awarded to;
    /// - no document of its type holds its id. A contested document type keeps no history, so
    ///   an id is taken only by a document in the type's primary storage, which is read
    ///   without decoding what it holds;
    /// - it does not take the total its type keeps of its summed property past what the
    ///   type's sums hold.
    fn state_admits_awarded_document(
        &self,
        block_info: &BlockInfo,
        document: &Document,
        document_type: DocumentTypeRef,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let contract = vote_poll.contract.as_ref();
        if !self
            .drive
            .validate_moderated_document_uniqueness(
                contract,
                document_type,
                document,
                None,
                transaction,
                platform_version,
            )?
            .is_valid()
        {
            return Ok(false);
        }
        let documents_holding_the_id = DriveDocumentQuery {
            internal_clauses: InternalClauses {
                primary_key_equal_clause: Some(WhereClause {
                    field: "$id".to_string(),
                    operator: WhereOperator::Equal,
                    value: Value::Identifier(document.id().to_buffer()),
                }),
                ..Default::default()
            },
            ..DriveDocumentQuery::all_items_query(contract, document_type, Some(1))
        };
        let (serialized_documents, ..) = documents_holding_the_id.execute_raw_results_no_proof(
            &self.drive,
            None,
            transaction,
            platform_version,
        )?;
        if !serialized_documents.is_empty() {
            return Ok(false);
        }
        // The check bills its read to a transition's execution context: ending a contest
        // bills no one, so the context is dropped.
        let mut execution_context =
            StateTransitionExecutionContext::default_for_platform_version(platform_version)?;
        Ok(validate_summed_value_within_type_total(
            contract,
            document_type,
            document.properties(),
            None,
            &self.drive,
            block_info,
            transaction,
            &mut execution_context,
            platform_version,
        )?
        .is_valid())
    }

    /// Takes a contest left open off the end date it was due at and lists it under
    /// [`VOTE_POLLS_LEFT_OPEN_END_DATE`], so that it is not ended again at every block and a
    /// later protocol version finds it. Its contenders, their documents, its votes, its record
    /// and its prefunded balance stay as they are.
    fn move_contest_to_the_left_open_end_date(
        &self,
        block_info: &BlockInfo,
        end_date: TimestampMillis,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut operations = vec![];
        self.drive
            .remove_contested_resource_vote_poll_end_date_query_operations(
                &[(vote_poll, &end_date, &BTreeMap::new())],
                &mut operations,
                transaction,
                platform_version,
            )?;
        // The entry is the protocol's: it names no identity as the owner of its storage
        self.drive.add_vote_poll_end_date_query_operations(
            None,
            VotePoll::ContestedDocumentResourceVotePoll(vote_poll.into()),
            VOTE_POLLS_LEFT_OPEN_END_DATE,
            block_info,
            &mut None,
            &mut None,
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
