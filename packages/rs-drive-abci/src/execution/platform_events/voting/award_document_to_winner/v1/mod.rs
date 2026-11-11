use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::rpc::core::CoreRPCLike;
use dpp::block::block_info::BlockInfo;
use dpp::document::DocumentV0Getters;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use dpp::voting::contender_structs::FinalizedContender;
use drive::drive::document::query::QueryDocumentsOutcomeV0Methods;
use drive::drive::document::shared_unique_entry::unique_index_entry_is_shared;
use drive::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use drive::grovedb::TransactionArg;
use drive::query::{DriveDocumentQuery, InternalClauses, WhereClause, WhereOperator};
use drive::util::object_size_info::DocumentInfo;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Awards a document to the winner of a contest, unless the state has no room for it, and
    /// returns whether it did.
    ///
    /// Version 0 inserts the document as it is. A contender's document waits in the contest's
    /// area, where the checks a document create runs do not see it, so nothing but those
    /// checks, run on other creates while the contest is live, keeps its place free. When they
    /// let a document take it, the insert of version 0 has no room for the winner's document
    /// and returns an error. The known case is a contest its index values do not decide, which
    /// registration refuses from protocol version 15 but a contract registered earlier keeps:
    /// a document outside the contest holds the contest's index values from before the
    /// create's state validation 3 guarded them.
    ///
    /// This version runs the two checks of a document create on the winner's document first:
    /// no document holds its id, and no document holds the values of its unique index. When
    /// either fails nothing is inserted and `false` is returned, and the contest ends without
    /// a winner. Otherwise version 0 inserts it.
    #[inline(always)]
    pub(super) fn award_document_to_winner_v1(
        &self,
        block_info: &BlockInfo,
        contender: FinalizedContender,
        vote_poll: &ContestedDocumentResourceVotePollWithContractInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<bool, Error> {
        let contract = vote_poll.contract.as_ref();
        let document_type = vote_poll.document_type()?;

        let documents_with_the_id = self.drive.query_documents(
            DriveDocumentQuery {
                contract,
                document_type,
                internal_clauses: InternalClauses {
                    primary_key_in_clause: None,
                    primary_key_equal_clause: Some(WhereClause {
                        field: "$id".to_string(),
                        operator: WhereOperator::Equal,
                        value: Value::Identifier(contender.document.id().to_buffer()),
                    }),
                    in_clauses: Vec::new(),
                    range_clause: None,
                    equal_clauses: Default::default(),
                },
                offset: None,
                limit: Some(1),
                order_by: Default::default(),
                start_at: None,
                start_at_included: false,
                block_time_ms: None,
                resolved_time_ranges: vec![],
                sub_queries: vec![],
            },
            None,
            false,
            transaction,
            Some(platform_version.protocol_version),
        )?;
        if !documents_with_the_id.documents().is_empty() {
            return Ok(false);
        }

        // A type with a contested index has no other unique index
        // (`ContestedUniqueIndexWithUniqueIndexError`), so the contested one is the only entry
        // the insert can find taken. An entry Drive keeps under the document's id, because one
        // of its keys is empty, meets no other document's, whatever values that one holds.
        if unique_index_entry_is_shared(
            &DocumentInfo::DocumentRefInfo((&contender.document, None)),
            vote_poll.index()?,
            document_type,
            Some(contender.identity_id.to_buffer()),
            platform_version,
        )? && !self
            .drive
            .validate_moderated_document_uniqueness(
                contract,
                document_type,
                &contender.document,
                None,
                transaction,
                platform_version,
            )?
            .is_valid()
        {
            return Ok(false);
        }

        self.award_document_to_winner_v0(
            block_info,
            contender,
            vote_poll,
            transaction,
            platform_version,
        )?;
        Ok(true)
    }
}
