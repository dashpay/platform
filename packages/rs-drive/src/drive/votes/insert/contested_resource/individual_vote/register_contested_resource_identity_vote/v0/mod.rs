use crate::drive::object_size_info::PathKeyElementInfo::PathKeyElement;
use crate::drive::votes::paths::VotePollPaths;
use crate::drive::votes::resolved::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePollWithContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::voting::vote_choices::resource_vote_choice::ResourceVoteChoice;
use grovedb::batch::KeyInfoPath;
use grovedb::{Element, EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;

impl Drive {
    pub(super) fn register_contested_resource_identity_vote_v0(
        &self,
        voter_pro_tx_hash: [u8; 32],
        vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
        vote_choice: ResourceVoteChoice,
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
            vote_poll,
            vote_choice,
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
        vote_poll: ContestedDocumentResourceVotePollWithContractInfo,
        vote_choice: ResourceVoteChoice,
        block_info: &BlockInfo,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        // let's start by creating a batch of operations
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        // The vote at this point will have been verified as valid by rs-drive-abci

        let voting_path = vote_poll.contender_voting_path(vote_choice, platform_version)?;

        self.batch_insert::<0>(
            PathKeyElement((
                voting_path,
                voter_pro_tx_hash.to_vec(),
                Element::new_sum_item(1),
            )),
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok(drive_operations)
    }
}
