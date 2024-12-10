use crate::drive::votes::paths::VotePollPaths;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::vote_poll_vote_state_query::{
    ContestedDocumentVotePollDriveQueryResultType, ResolvedContestedDocumentVotePollDriveQuery,
};
use crate::util::grove_operations::BatchDeleteApplyType;
use dpp::document::DocumentV0Getters;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// We add documents poll references by end date in order to be able to check on every new block if
    /// any vote polls should be closed.
    pub(in crate::drive::votes) fn remove_contested_resource_vote_poll_documents_operations_v1(
        &self,
        vote_polls: &[(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )],
        clean_up_testnet_corrupted_reference_issue: bool,
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for (vote_poll, _, vote_choices) in vote_polls {
            if !clean_up_testnet_corrupted_reference_issue {
                // if we are in clean up we keep all documents
                let query = ResolvedContestedDocumentVotePollDriveQuery {
                    vote_poll: (*vote_poll).into(),
                    result_type: ContestedDocumentVotePollDriveQueryResultType::Documents,
                    offset: None,
                    limit: None,
                    start_at: None,
                    allow_include_locked_and_abstaining_vote_tally: false,
                };

                let contested_document_vote_poll_drive_query_execution_result =
                    query.execute(self, transaction, &mut vec![], platform_version)?;

                let document_type = vote_poll.document_type()?;
                let document_keys = contested_document_vote_poll_drive_query_execution_result
                    .contenders
                    .into_iter()
                    .filter_map(|contender| {
                        let maybe_document_result =
                            match contender.try_into_contender(document_type, platform_version) {
                                Ok(mut contender) => contender.take_document(),
                                Err(e) => return Some(Err(e.into())),
                            };

                        match maybe_document_result {
                            Some(document) => Some(Ok(document.id().to_vec())), // Assuming document.id holds the document key
                            None => None, // Handle the case where no document is found
                        }
                    })
                    .collect::<Result<Vec<Vec<u8>>, Error>>()?;

                let documents_storage_path = vote_poll.documents_storage_path_vec();

                for document_key in document_keys {
                    self.batch_delete(
                        documents_storage_path.as_slice().into(),
                        document_key.as_slice(),
                        BatchDeleteApplyType::StatefulBatchDelete {
                            is_known_to_be_subtree_with_sum: Some((false, false)),
                        },
                        transaction,
                        batch_operations,
                        &platform_version.drive,
                    )?;
                }
            }

            // We also need to delete all the references

            for resource_vote_choice in vote_choices.keys() {
                if matches!(resource_vote_choice, ResourceVoteChoice::TowardsIdentity(_)) {
                    let contender_path =
                        vote_poll.contender_path(resource_vote_choice, platform_version)?;
                    self.batch_delete(
                        contender_path.as_slice().into(),
                        vec![0].as_slice(),
                        BatchDeleteApplyType::StatefulBatchDelete {
                            is_known_to_be_subtree_with_sum: Some((false, false)),
                        },
                        transaction,
                        batch_operations,
                        &platform_version.drive,
                    )?;
                }
            }
        }

        Ok(())
    }
}
