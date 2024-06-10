use crate::drive::grove_operations::{BatchDeleteApplyType, BatchInsertTreeApplyType};
use crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::drive::object_size_info::PathKeyInfo;
use crate::drive::votes::paths::{
    vote_contested_resource_identity_votes_tree_path_for_identity_vec,
    vote_contested_resource_identity_votes_tree_path_vec, VotePollPaths,
};
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use dpp::{bincode, ProtocolError};
use grovedb::batch::KeyInfoPath;
use grovedb::reference_path::ReferencePathType;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    pub(super) fn register_contested_resource_identity_vote_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
        vote_choice: ResourceVoteChoice,
        previous_resource_vote_choice_to_remove: Option<ResourceVoteChoice>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info = if apply {
            None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>
        } else {
            Some(HashMap::new())
        };

        let batch_operations = self.register_contested_resource_identity_vote_operations_v0(
            voter_pro_tx_hash,
            strength,
            vote_poll,
            vote_choice,
            previous_resource_vote_choice_to_remove,
            block_info,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
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
        )?;
        Ok(fees)
    }

    pub(super) fn register_contested_resource_identity_vote_operations_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        strength: u8,
        vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
        vote_choice: ResourceVoteChoice,
        previous_resource_vote_choice_to_remove: Option<ResourceVoteChoice>,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        //todo estimated costs
        // let's start by creating a batch of operations
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        // The vote at this point will have been verified as valid by rs-drive-abci

        // We start by inserting the main vote as a value of 1 or 4 depending on the strength

        let mut voting_path = vote_poll.contender_voting_path(&vote_choice, platform_version)?;

        self.batch_insert::<0>(
            PathKeyElement((
                voting_path.clone(),
                voter_pro_tx_hash.to_vec(),
                Element::new_sum_item(strength as i64),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        if let Some(previous_resource_vote_choice_to_remove) =
            previous_resource_vote_choice_to_remove
        {
            let previous_voting_path = vote_poll.contender_voting_path(
                &previous_resource_vote_choice_to_remove,
                platform_version,
            )?;

            self.batch_delete(
                previous_voting_path.as_slice().into(),
                voter_pro_tx_hash.as_slice(),
                BatchDeleteApplyType::StatefulBatchDelete {
                    is_known_to_be_subtree_with_sum: Some((false, true)),
                },
                transaction,
                &mut drive_operations,
                &platform_version.drive,
            )?;
        }

        let votes_identities_path = vote_contested_resource_identity_votes_tree_path_vec();

        self.batch_insert_empty_tree_if_not_exists(
            PathKeyInfo::PathKey::<0>((votes_identities_path, voter_pro_tx_hash.to_vec())),
            false,
            None,
            BatchInsertTreeApplyType::StatefulBatchInsertTree, //todo this shouldn't always be stateful
            transaction,
            &mut None, //we shouldn't have more than one document here
            &mut drive_operations,
            &platform_version.drive,
        )?;

        // Now we create the vote reference

        let path =
            vote_contested_resource_identity_votes_tree_path_for_identity_vec(&voter_pro_tx_hash);

        voting_path.remove(0); // we remove the top (root tree vote key)
        voting_path.remove(0); // contested resource

        let reference =
            ReferencePathType::UpstreamRootHeightWithParentPathAdditionReference(2, voting_path);
        let config = bincode::config::standard()
            .with_big_endian()
            .with_no_limit();
        let encoded_reference = bincode::encode_to_vec(reference, config).map_err(|e| {
            Error::Protocol(ProtocolError::CorruptedSerialization(format!(
                "can not encode reference: {}",
                e
            )))
        })?;

        self.batch_insert::<0>(
            PathKeyElement((
                path,
                vote_poll.unique_id()?.to_vec(),
                // We store the encoded reference as an item on purpose as we want the advantages of a resolvable
                // reference, but at the same time, we don't want the proof to have the value of the followed
                // reference, because here there is no point, it being 1 or 4.
                Element::new_item(encoded_reference),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok(drive_operations)
    }
}
