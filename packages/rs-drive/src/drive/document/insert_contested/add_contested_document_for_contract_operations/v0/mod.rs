use crate::drive::object_size_info::DocumentAndContractInfo;
use crate::drive::Drive;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::version::PlatformVersion;
use dpp::voting::vote_polls::contested_document_resource_vote_poll::ContestedDocumentResourceVotePoll;
use dpp::voting::vote_polls::VotePoll;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Gathers the operations to add a contested document to a contract.
    #[inline(always)]
    pub(super) fn add_contested_document_for_contract_operations_v0(
        &self,
        document_and_contract_info: DocumentAndContractInfo,
        contested_document_resource_vote_poll: ContestedDocumentResourceVotePoll,
        insert_without_check: bool,
        block_info: &BlockInfo,
        previous_batch_operations: &mut Option<&mut Vec<LowLevelDriveOperation>>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut batch_operations: Vec<LowLevelDriveOperation> = vec![];

        // if we are trying to get estimated costs we need to add the upper levels
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            Self::add_estimation_costs_for_contested_document_tree_levels_up_to_contract(
                document_and_contract_info.contract,
                Some(document_and_contract_info.document_type),
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        // if we have override_document set that means we already checked if it exists
        self.add_contested_document_to_primary_storage(
            &document_and_contract_info,
            insert_without_check,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;

        let end_date = block_info.time_ms.saturating_add(
            platform_version
                .dpp
                .voting_versions
                .default_vote_poll_time_duration_ms,
        );

        let contest_already_existed = self.add_contested_indices_for_contract_operations(
            &document_and_contract_info,
            previous_batch_operations,
            estimated_costs_only_with_layer_info,
            transaction,
            &mut batch_operations,
            platform_version,
        )?;

        if !contest_already_existed {
            self.add_vote_poll_end_date_query_operations(
                document_and_contract_info.owned_document_info.owner_id,
                VotePoll::ContestedDocumentResourceVotePoll(contested_document_resource_vote_poll),
                end_date,
                block_info,
                estimated_costs_only_with_layer_info,
                previous_batch_operations,
                &mut batch_operations,
                transaction,
                platform_version,
            )?;
        }

        Ok(batch_operations)
    }
}
