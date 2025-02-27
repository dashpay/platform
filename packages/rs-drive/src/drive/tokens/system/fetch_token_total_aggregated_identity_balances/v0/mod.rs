use crate::drive::tokens::paths::token_balances_root_path;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::util::grove_operations::DirectQueryType;
use crate::util::grove_operations::QueryTarget::QueryTargetValue;
use dpp::balances::credits::TokenAmount;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg, TreeType};
use std::collections::HashMap;

impl Drive {
    pub(super) fn fetch_token_total_aggregated_identity_balances_v0(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenAmount>, Error> {
        let mut drive_operations = vec![];

        self.fetch_token_total_aggregated_identity_balances_add_to_operations_v0(
            token_id,
            &mut None,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    pub(super) fn fetch_token_total_aggregated_identity_balances_add_to_operations_v0(
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
            Self::add_estimation_costs_for_token_balances(
                token_id,
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

        let tokens_root_path = token_balances_root_path();

        let total_token_aggregated_identity_balances_in_platform = self
            .grove_get_raw_value_u64_from_encoded_var_vec(
                (&tokens_root_path).into(),
                &token_id,
                direct_query_type,
                transaction,
                drive_operations,
                &platform_version.drive,
            )?;

        Ok(total_token_aggregated_identity_balances_in_platform)
    }
}
