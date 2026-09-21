use crate::drive::votes::paths::vote_identity_contender_identity_votes_tree_path_for_identity;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{MaybeTree, TransactionArg};

impl Drive {
    pub(super) fn remove_identity_contender_vote_references_given_by_identity_operations_v0(
        &self,
        identity_id: &Identifier,
        vote_poll_ids: &[&Identifier],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let path =
            vote_identity_contender_identity_votes_tree_path_for_identity(identity_id.as_bytes());
        for vote_poll_id in vote_poll_ids {
            self.batch_delete(
                path.as_slice().into(),
                vote_poll_id.as_slice(),
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                },
                transaction,
                batch_operations,
                &platform_version.drive,
            )?;
        }
        Ok(())
    }
}
