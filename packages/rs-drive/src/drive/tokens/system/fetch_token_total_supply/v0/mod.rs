use crate::drive::balances::total_tokens_root_supply_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn fetch_token_total_supply_v0(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenAmount>, Error> {
        let mut drive_operations = vec![];

        self.fetch_token_total_supply_add_to_operations_v0(
            token_id,
            &mut None,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    pub(super) fn fetch_token_total_supply_with_cost_v0(
        &self,
        token_id: [u8; 32],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<TokenAmount>, FeeResult), Error> {
        let mut drive_operations = vec![];

        let token_amount = self.fetch_token_total_supply_add_to_operations_v0(
            token_id,
            &mut None,
            transaction,
            &mut drive_operations,
            platform_version,
        )?;
        let fees = Drive::calculate_fee(
            None,
            Some(drive_operations),
            &block_info.epoch,
            self.config.epochs_per_era,
            platform_version,
            None,
        )?;
        Ok((token_amount, fees))
    }

    pub(super) fn fetch_token_total_supply_add_to_operations_v0(
        &self,
        token_id: [u8; 32],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenAmount>, Error> {
        // If we only estimate, add estimation costs
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // Add your estimation logic similar to add_to_system_credits_operations_v0
            // For example:
            Self::add_estimation_costs_for_token_total_supply(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let direct_query_type = if estimated_costs_only_with_layer_info.is_none() {
            DirectQueryType::StatefulDirectQuery
        } else {
            DirectQueryType::StatelessDirectQuery {
                in_tree_type: TreeType::BigSumTree,
                query_target: QueryTargetValue(8),
            }
        };

        let path_holding_total_token_supply = total_tokens_root_supply_path();
        let total_token_supply_in_platform = self.grove_get_raw_value_u64_from_encoded_var_vec(
            (&path_holding_total_token_supply).into(),
            &token_id,
            direct_query_type,
            transaction,
            drive_operations,
            &platform_version.drive,
        )?;

        Ok(total_token_supply_in_platform)
    }
}
