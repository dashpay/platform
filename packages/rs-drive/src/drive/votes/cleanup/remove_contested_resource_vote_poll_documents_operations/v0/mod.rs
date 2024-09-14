use crate::drive::votes::paths::VotePollPaths;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::query::QueryItem;
use crate::util::grove_operations::BatchDeleteApplyType;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::query_result_type::QueryResultType;
use grovedb::{PathQuery, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;
use std::ops::RangeFull;

impl Drive {
    /// We add documents poll references by end date in order to be able to check on every new block if
    /// any vote polls should be closed.
    /// !!!!! THIS VERSION CONTAINED A SERIOUS ISSUE !!!!!
    /// However, it should never have made it to mainnet.
    pub(in crate::drive::votes) fn remove_contested_resource_vote_poll_documents_operations_v0(
        &self,
        vote_polls: &[(
            &ContestedDocumentResourceVotePollWithContractInfo,
            &TimestampMillis,
            &BTreeMap<ResourceVoteChoice, Vec<Identifier>>,
        )],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for (vote_poll, _, vote_choices) in vote_polls {
            let documents_storage_path = vote_poll.documents_storage_path_vec();

            let path_query = PathQuery::new_single_query_item(
                documents_storage_path.clone(),
                QueryItem::RangeFull(RangeFull),
            );

            let document_keys = self
                .grove_get_raw_path_query(
                    &path_query,
                    transaction,
                    QueryResultType::QueryKeyElementPairResultType,
                    &mut vec![],
                    &platform_version.drive,
                )?
                .0
                .to_keys();

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
