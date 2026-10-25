use crate::drive::Drive;

use crate::error::Error;

use crate::drive::votes::paths::{
    vote_decisions_identity_votes_tree_path_vec, vote_decisions_poll_choice_votes_tree_path_vec,
    vote_decisions_tree_path, IDENTITY_VOTES_TREE_KEY,
};
use crate::drive::votes::storage_form::yes_no_vote_reference_storage_form::YesNoVoteReferenceStorageForm;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{BatchDeleteApplyType, DirectQueryType};
use dpp::dashcore::Network;
use dpp::prelude::{BlockHeight, Identifier};
use dpp::version::PlatformVersion;
use grovedb::batch::{QualifiedGroveDbOp, SubelementsDeletionBehavior};
use grovedb::{MaybeTree, TransactionArg, TreeType};

impl Drive {
    /// Version 1 (protocol version 15) removes the contested resource votes of the removed
    /// masternodes through version 0, unchanged, then their yes/no votes in a batch of its own:
    /// the vote under the poll's choice sum tree, and each removed masternode's tree in the
    /// decisions identity votes index, entries included. The two batches touch different
    /// subtrees, so applying them one after the other leaves the same state as one batch would.
    pub(super) fn remove_all_votes_given_by_identities_v1(
        &self,
        identity_ids_as_byte_arrays: Vec<Vec<u8>>,
        block_height: BlockHeight,
        network: Network,
        chain_id: &str,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.remove_all_votes_given_by_identities_v0(
            identity_ids_as_byte_arrays.clone(),
            block_height,
            network,
            chain_id,
            transaction,
            platform_version,
        )?;

        // Yes/no votes: the index item names the poll and the answer, so the vote sits under
        // the poll's tree at the answer's sum tree. A chain that activated protocol version 15
        // before the decisions trees existed has no identity votes index, and no yes/no votes.
        let has_decisions_index = self.grove_has_raw(
            (&vote_decisions_tree_path()).into(),
            &[IDENTITY_VOTES_TREE_KEY as u8],
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut vec![],
            &platform_version.drive,
        )?;
        if !has_decisions_index {
            return Ok(());
        }

        let mut deletion_batch = vec![];
        for (identifier_bytes, votes_to_remove) in self.identity_vote_entries_of_identities_v0(
            vote_decisions_identity_votes_tree_path_vec(),
            identity_ids_as_byte_arrays.clone(),
            transaction,
            platform_version,
        )? {
            for (vote_poll_id, vote_to_remove) in votes_to_remove {
                let storage_form =
                    YesNoVoteReferenceStorageForm::deserialize(&vote_to_remove.into_item_bytes()?)?;
                let vote_poll_id = Identifier::from_vec(vote_poll_id)?;
                let votes_path = vote_decisions_poll_choice_votes_tree_path_vec(
                    vote_poll_id.as_bytes(),
                    storage_form.vote_choice,
                );
                self.batch_delete(
                    votes_path.as_slice().into(),
                    identifier_bytes.as_slice(),
                    BatchDeleteApplyType::StatefulBatchDelete {
                        is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                    },
                    transaction,
                    &mut deletion_batch,
                    &platform_version.drive,
                )?;
            }
        }

        // Then each removed masternode's tree in the identity votes index, emptied or not: the
        // close of a poll removes the poll's entries but keeps the tree, which no one else
        // removes. It holds only items, so it is dropped flat with them (GroveDB's flat drop,
        // which the grove version of protocol version 15 accepts in a batch).
        let identity_votes_path = vote_decisions_identity_votes_tree_path_vec();
        for identifier_bytes in identity_ids_as_byte_arrays {
            let has_identity_votes_tree = self.grove_has_raw(
                identity_votes_path.as_slice().into(),
                identifier_bytes.as_slice(),
                DirectQueryType::StatefulDirectQuery,
                transaction,
                &mut vec![],
                &platform_version.drive,
            )?;
            if has_identity_votes_tree {
                deletion_batch.push(LowLevelDriveOperation::GroveOperation(
                    QualifiedGroveDbOp::delete_tree_op(
                        identity_votes_path.clone(),
                        identifier_bytes,
                        TreeType::NormalTree,
                        SubelementsDeletionBehavior::DropFlat,
                    )
                    .dont_check_for_backwards_references(),
                ));
            }
        }

        if !deletion_batch.is_empty() {
            self.apply_batch_low_level_drive_operations(
                None,
                transaction,
                deletion_batch,
                &mut vec![],
                &platform_version.drive,
            )
            .inspect_err(|err| tracing::error!(?err, "yes/no vote deletion batch failed"))?;
        }

        Ok(())
    }
}
