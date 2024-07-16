use crate::drive::votes::paths::VotePollPaths;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::votes::ResourceVoteChoiceToKeyTrait;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;
use dpp::identifier::Identifier;
use dpp::identity::TimestampMillis;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::TransactionArg;
use platform_version::version::PlatformVersion;
use std::collections::BTreeMap;

impl Drive {
    /// We add votes poll references by end date in order to be able to check on every new block if
    /// any vote polls should be closed.
    pub(in crate::drive::votes) fn remove_contested_resource_vote_poll_contenders_operations_v0(
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
        for (vote_poll, _, votes) in vote_polls {
            let path = vote_poll.contenders_path(platform_version)?;
            for (resource_vote_choice, _) in *votes {
                self.batch_delete(
                    path.as_slice().into(),
                    resource_vote_choice.to_key().as_slice(),
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some((false, false)),
                    },
                    transaction,
                    batch_operations,
                    &platform_version.drive,
                )?;
            }
        }

        Ok(())
    }
}
