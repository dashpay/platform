use crate::drive::Drive;
use crate::error::Error;

use crate::drive::votes::paths::vote_contested_resource_identity_votes_tree_path_for_identity;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::BatchDeleteApplyType;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{MaybeTree, TransactionArg};

impl Drive {
    /// We remove votes for an identity when that identity is somehow disabled. Currently there is
    /// no way to "disable" identities except for masternodes being removed from the list
    pub(super) fn remove_specific_vote_references_given_by_identity_v0(
        &self,
        identity_id: &Identifier,
        votes: &[&Identifier],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // Then we take each votes and go looking for it (to remove it)

        let vote_path_ref =
            vote_contested_resource_identity_votes_tree_path_for_identity(identity_id.as_bytes());

        self.remove_identity_vote_references_operations_v0(
            &vote_path_ref,
            votes,
            batch_operations,
            transaction,
            platform_version,
        )
    }

    /// The loop of `remove_specific_vote_references_given_by_identity_v0`, moved here unchanged
    /// (in place: the same deletes in the same order at every protocol version that selects
    /// it) so the yes/no identity votes index, which differs only in its path, removes its
    /// entries the same way.
    pub(in crate::drive::votes::cleanup) fn remove_identity_vote_references_operations_v0(
        &self,
        vote_path_ref: &[&[u8]; 4],
        votes: &[&Identifier],
        batch_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for vote_identifier_to_remove in votes {
            self.batch_delete(
                vote_path_ref.as_slice().into(),
                vote_identifier_to_remove.as_slice(),
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
