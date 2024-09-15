use crate::drive::votes::paths::{VotePollPaths, RESOURCE_STORED_INFO_KEY_U8_32};
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
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
    pub(in crate::drive::votes) fn remove_contested_resource_info_operations_v0(
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
        for (vote_poll, _, _) in vote_polls {
            let path = vote_poll.contenders_path(platform_version)?;
            self.batch_delete(
                path.as_slice().into(),
                &RESOURCE_STORED_INFO_KEY_U8_32,
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some((false, false)),
                },
                transaction,
                batch_operations,
                &platform_version.drive,
            )?;
        }

        Ok(())
    }
}
