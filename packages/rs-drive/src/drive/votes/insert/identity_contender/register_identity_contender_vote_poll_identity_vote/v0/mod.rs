use crate::drive::votes::paths::{
    vote_identity_contender_identity_votes_tree_path_for_identity_vec,
    vote_identity_contender_identity_votes_tree_path_vec, ACTIVE_POLLS_TREE_KEY,
    VOTING_STORAGE_TREE_KEY,
};
use crate::drive::votes::storage_form::contested_document_resource_reference_storage_form::ContestedDocumentResourceVoteReferenceStorageForm;
use crate::drive::votes::ResourceVoteChoiceToKeyTrait;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::identity::masternode_vote::v0::PreviousVoteCount;
use crate::util::grove_operations::{BatchDeleteApplyType, BatchInsertTreeApplyType};
use crate::util::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::util::object_size_info::PathKeyInfo;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::voting::vote_polls::identity_contender_vote_poll::IdentityContenderVotePoll;
use dpp::{bincode, ProtocolError};
use grovedb::element::reference_path::ReferencePathType;
use grovedb::{Element, MaybeTree, TransactionArg, TreeType};
use platform_version::version::PlatformVersion;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn register_identity_contender_vote_poll_identity_vote_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote_poll: IdentityContenderVotePoll,
        vote_choice: ResourceVoteChoice,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let batch_operations = self
            .register_identity_contender_vote_poll_identity_vote_operations_v0(
                voter_pro_tx_hash,
                strength,
                vote_poll,
                vote_choice,
                previous_resource_vote_choice_to_remove,
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
        Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )
    }

    /// The vote is a sum item of the masternode's strength under the choice, and the masternode's
    /// identity votes tree keeps a reference to it, laid out exactly like a vote on a contested
    /// resource so the same reference decoding serves both.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn register_identity_contender_vote_poll_identity_vote_operations_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote_poll: IdentityContenderVotePoll,
        vote_choice: ResourceVoteChoice,
        previous_resource_vote_choice_to_remove: Option<(ResourceVoteChoice, PreviousVoteCount)>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        // Voting is a fixed cost, so no estimated costs are ever needed here.
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let vote_poll_id = vote_poll.unique_id()?;

        // The vote itself
        let vote_path_under_branch = vec![
            vec![ACTIVE_POLLS_TREE_KEY as u8],
            vote_poll_id.to_vec(),
            vote_choice.to_key(),
            vec![VOTING_STORAGE_TREE_KEY],
        ];
        let mut voting_path = vote_identity_contender_identity_votes_tree_path_vec();
        voting_path.pop(); // the branch: [votes, 'n']
        voting_path.extend(vote_path_under_branch.iter().cloned());
        self.batch_insert::<0>(
            PathKeyElement((
                voting_path,
                voter_pro_tx_hash.to_vec(),
                Element::new_sum_item(strength as i64),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        let mut identity_vote_times = 1;
        if let Some((previous_resource_vote_choice_to_remove, previous_vote_count)) =
            previous_resource_vote_choice_to_remove
        {
            let mut previous_voting_path = vote_identity_contender_identity_votes_tree_path_vec();
            previous_voting_path.pop();
            previous_voting_path.extend([
                vec![ACTIVE_POLLS_TREE_KEY as u8],
                vote_poll_id.to_vec(),
                previous_resource_vote_choice_to_remove.to_key(),
                vec![VOTING_STORAGE_TREE_KEY],
            ]);
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

        // The masternode's own list of votes, with a reference to the vote
        self.batch_insert_empty_tree_if_not_exists(
            PathKeyInfo::PathKey::<0>((
                vote_identity_contender_identity_votes_tree_path_vec(),
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
        let reference_path_type =
            ReferencePathType::UpstreamRootHeightWithParentPathAdditionReference(
                2,
                vote_path_under_branch,
            );
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let storage_form = ContestedDocumentResourceVoteReferenceStorageForm {
            reference_path_type,
            identity_vote_times,
        };
        let encoded_reference = bincode::encode_to_vec(storage_form, config).map_err(|e| {
            Error::Protocol(Box::new(ProtocolError::CorruptedSerialization(format!(
                "can not encode reference: {}",
                e
            ))))
        })?;
        self.batch_insert::<0>(
            PathKeyElement((
                vote_identity_contender_identity_votes_tree_path_for_identity_vec(
                    &voter_pro_tx_hash,
                ),
                vote_poll_id.to_vec(),
                // An item holding the reference path rather than a reference, as for contested
                // resources: a proof then carries the path and not the vote it points at.
                Element::new_item(encoded_reference),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;
        Ok(drive_operations)
    }
}
