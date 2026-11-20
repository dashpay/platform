use super::v1::{earliest_contender, EndedContest};
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::{DocumentTypeV0Getters, DocumentTypeV2Getters};
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::data_contract::schema::DataContractSchemaMethodsV0;
use dpp::document::{Document, DocumentV0Getters};
use dpp::platform_value::Value;
use dpp::prelude::TimestampMillis;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::FinalizedContender;
use dpp::voting::vote_info_storage::contested_document_vote_poll_winner_info::ContestedDocumentVotePollWinnerInfo;
use dpp::voting::vote_polls::VotePoll;
use drive::drive::document::paths::document_history_path;
use drive::drive::document::read_document_sum_contribution;
use drive::drive::votes::paths::VOTE_POLLS_LEFT_OPEN_END_DATE;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::TransactionArg;
use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use drive::util::grove_operations::DirectQueryType;
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
    /// - The winner's document cannot be written as a document created outside a contest is
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

    /// Whether the winner's document can be written, by what a document created outside a
    /// contest is held to. A contender's create was checked against the contest alone, and
    /// other documents were written since:
    ///
    /// - the type keeps its summed values within the limit registration holds a type with a
    ///   contested index to (`DocumentTypeV2::summed_property_past_contested_limit`), so
    ///   adding the winner's value keeps the type's sums in range. A type registered before
    ///   protocol version 14 may sum values of any size and sign, and then nothing read before
    ///   the write tells whether a sum stays in range. A schema the limit cannot be read from
    ///   counts as past it: that depends on the contract alone, never on a node's storage;
    /// - the document holds, for every property the type's sums read, a value Drive writes into
    ///   them: an integer that fits an `i64`, at the top level of the document
    ///   (`read_document_sum_contribution`). A contender's document is stored without the values
    ///   of the type's transient properties, and Drive reads a summed value by its property's
    ///   top-level name. From protocol version 15 registration refuses a type summing a
    ///   transient property, and from protocol version 14 a summed property named by a dotted
    ///   path; a type registered earlier may sum either;
    /// - no other document holds a value of one of its unique indexes. The document is
    ///   checked as stored: its owner is the identity it is awarded to;
    /// - no document of its type holds its id: neither a document in the type's primary
    ///   storage, read without decoding what it holds, nor the revisions a deleted document
    ///   retains. From protocol version 15 registration refuses a contested index on a type
    ///   that keeps history; a type registered earlier may have both.
    fn state_admits_awarded_document(
        &self,
        document: &Document,
        document_type: DocumentTypeRef,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let contract = vote_poll.contract.as_ref();
        if let DocumentTypeRef::V2(document_type) = document_type {
            if !matches!(
                document_type
                    .summed_property_past_contested_limit(contract.schema_defs(), platform_version),
                Ok(None)
            ) {
                return Ok(false);
            }
            let mut summed_properties = document_type.documents_summable().into_iter().chain(
                document_type
                    .indexes()
                    .values()
                    .filter_map(|index| index.summable.as_deref()),
            );
            if summed_properties
                .any(|property| read_document_sum_contribution(document, property).is_err())
            {
                return Ok(false);
            }
        }
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
        if !document_type.documents_keep_history() {
            return Ok(true);
        }
        // The revisions of a document are kept in a tree named by its id, the last segment
        // of its history path
        let mut history_trees_path = document_history_path(
            contract.id_ref().as_bytes(),
            document_type.name(),
            document.id().as_slice(),
        );
        history_trees_path.pop();
        Ok(!self.drive.grove_has_raw(
            history_trees_path.as_slice().into(),
            document.id().as_slice(),
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?)
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
