use crate::drive::votes::paths::{
    vote_decisions_identity_votes_tree_path_for_identity_vec,
    vote_decisions_identity_votes_tree_path_vec, YesNoVotePollPaths,
};
use crate::drive::votes::resolved::votes::resolved_yes_no_vote::ResolvedYesNoVote;
use crate::drive::votes::storage_form::yes_no_vote_reference_storage_form::YesNoVoteReferenceStorageForm;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::{BatchDeleteApplyType, BatchInsertTreeApplyType};
use crate::util::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::util::object_size_info::PathKeyInfo;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::{Element, MaybeTree, TransactionArg, TreeType};

impl Drive {
    pub(super) fn register_yes_no_identity_vote_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote: ResolvedYesNoVote,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let batch_operations = self.register_yes_no_identity_vote_operations_v0(
            voter_pro_tx_hash,
            strength,
            vote,
            transaction,
            platform_version,
        )?;
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok(fees)
    }

    pub(super) fn register_yes_no_identity_vote_operations_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote: ResolvedYesNoVote,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        // Voting has a fixed cost, so there are no estimated costs to keep.
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let ResolvedYesNoVote {
            vote_poll,
            vote_choice,
            previous_vote_choice_to_remove,
        } = vote;

        // The vote itself: the masternode's strength under its answer's sum tree.
        let voting_path = vote_poll.choice_votes_path_vec(vote_choice)?;
        self.batch_insert::<0>(
            PathKeyElement((
                voting_path,
                voter_pro_tx_hash.to_vec(),
                Element::new_sum_item(strength as i64),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // A changed vote leaves its previous answer.
        let mut identity_vote_times = 1;
        if let Some((previous_vote_choice, previous_vote_count)) = previous_vote_choice_to_remove {
            let previous_voting_path = vote_poll.choice_votes_path_vec(previous_vote_choice)?;
            self.batch_delete(
                previous_voting_path.as_slice().into(),
                voter_pro_tx_hash.as_slice(),
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some(MaybeTree::NotTree),
                },
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
            identity_vote_times += previous_vote_count;
        }

        // The identity votes index: the masternode's tree, then its answer to this poll.
        self.batch_insert_empty_tree_if_not_exists(
            PathKeyInfo::PathKey::<0>((
                vote_decisions_identity_votes_tree_path_vec(),
                voter_pro_tx_hash.to_vec(),
            )),
            TreeType::NormalTree,
            None,
            BatchInsertTreeApplyType::StatefulBatchInsertTree,
            transaction,
            &mut None,
            &mut drive_operations,
            &platform_version.drive,
        )?;
        let storage_form = YesNoVoteReferenceStorageForm {
            vote_choice,
            identity_vote_times,
        };
        let encoded = storage_form.serialize().map_err(|e| {
            Error::Drive(DriveError::CorruptedSerialization(format!(
                "can not encode yes/no vote reference: {}",
                e
            )))
        })?;
        self.batch_insert::<0>(
            PathKeyElement((
                vote_decisions_identity_votes_tree_path_for_identity_vec(&voter_pro_tx_hash),
                vote_poll.unique_id()?.to_vec(),
                Element::new_item(encoded),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Ok(drive_operations)
    }
}
