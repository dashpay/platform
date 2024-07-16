use crate::drive::votes::paths::vote_contested_resource_end_date_queries_at_time_tree_path_vec;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteUpTreeApplyType;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::batch::KeyInfoPath;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any vote polls should be closed.
    pub(in crate::drive::votes) fn remove_contested_resource_vote_poll_end_date_query_operations_v0(
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
        // This is a GroveDB Tree (Not Sub Tree Merk representation)
        //                         End Date queries
        //              /                                  \
        //       15/08/2025 5PM                                   15/08/2025 6PM
        //          /              \                                    |
        //     VotePoll Info 1   VotePoll Info 2                 VotePoll Info 3

        let delete_apply_type = BatchDeleteUpTreeApplyType::StatefulBatchDelete {
            is_known_to_be_subtree_with_sum: Some((false, false)),
        };

        for (vote_poll, end_date, _) in vote_polls {
            let time_path =
                vote_contested_resource_end_date_queries_at_time_tree_path_vec(**end_date);

            let unique_id = vote_poll.unique_id()?;

            self.batch_delete_up_tree_while_empty(
                KeyInfoPath::from_known_owned_path(time_path),
                unique_id.as_bytes(),
                Some(2),
                delete_apply_type.clone(),
                transaction,
                &None,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        Ok(())
    }
}
