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

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_return_none_for_non_existent_token() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let token_id = [21u8; 32];

        let balances = drive
            .fetch_token_total_aggregated_identity_balances_v0(token_id, None, platform_version)
            .expect("expected fetch to succeed");

        assert_eq!(balances, None);
    }

    #[test]
    fn should_return_zero_for_freshly_created_token_with_no_holders() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [22u8; 32];
        let contract_id = Identifier::from([23u8; 32]);

        drive
            .create_token_trees(
                contract_id,
                0,
                token_id,
                false,
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");

        let balances = drive
            .fetch_token_total_aggregated_identity_balances_v0(token_id, None, platform_version)
            .expect("expected fetch to succeed");

        // The balances tree exists but is empty — sum is 0
        assert_eq!(balances, Some(0));
    }

    #[test]
    fn should_aggregate_single_holder_balance() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [24u8; 32];
        let contract_id = Identifier::from([25u8; 32]);
        let identity_id = [26u8; 32];

        drive
            .create_token_trees(
                contract_id,
                0,
                token_id,
                false,
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");

        drive
            .add_to_identity_token_balance(
                token_id,
                identity_id,
                1_234,
                &block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to add token balance");

        let balances = drive
            .fetch_token_total_aggregated_identity_balances_v0(token_id, None, platform_version)
            .expect("expected fetch to succeed");

        assert_eq!(balances, Some(1_234));
    }

    #[test]
    fn should_populate_estimated_costs_in_stateless_mode() {
        use grovedb::batch::KeyInfoPath;
        use grovedb::EstimatedLayerInformation;
        use std::collections::HashMap;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let token_id = [28u8; 32];

        let mut estimated_costs: Option<HashMap<KeyInfoPath, EstimatedLayerInformation>> =
            Some(HashMap::new());
        let mut drive_operations = vec![];

        // Stateless estimation path
        let result = drive.fetch_token_total_aggregated_identity_balances_add_to_operations_v0(
            token_id,
            &mut estimated_costs,
            None,
            &mut drive_operations,
            platform_version,
        );

        assert!(result.is_ok());
        let estimated_costs = estimated_costs.expect("estimation state must persist");
        assert!(
            !estimated_costs.is_empty(),
            "expected stateless path to populate estimation layer info"
        );
    }

    #[test]
    fn should_reflect_balance_after_remove() {
        // Covers the aggregated value decreasing after remove_from_identity_token_balance
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [35u8; 32];
        let contract_id = Identifier::from([36u8; 32]);
        let identity_id = [37u8; 32];

        drive
            .create_token_trees(
                contract_id,
                0,
                token_id,
                false,
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");

        drive
            .add_to_identity_token_balance(
                token_id,
                identity_id,
                2_000,
                &block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to add balance");

        drive
            .remove_from_identity_token_balance(
                token_id,
                identity_id,
                750,
                &block_info,
                true,
                None,
                platform_version,
                None,
            )
            .expect("expected to remove balance");

        let aggregated = drive
            .fetch_token_total_aggregated_identity_balances_v0(token_id, None, platform_version)
            .expect("expected fetch to succeed");

        assert_eq!(aggregated, Some(1_250));
    }

    #[test]
    fn should_aggregate_multiple_holder_balances() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [30u8; 32];
        let contract_id = Identifier::from([31u8; 32]);

        drive
            .create_token_trees(
                contract_id,
                0,
                token_id,
                false,
                false,
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to create token trees");

        for (identity_id, amount) in [
            ([32u8; 32], 100u64),
            ([33u8; 32], 250u64),
            ([34u8; 32], 650u64),
        ] {
            drive
                .add_to_identity_token_balance(
                    token_id,
                    identity_id,
                    amount,
                    &block_info,
                    true,
                    None,
                    platform_version,
                    None,
                )
                .expect("expected to add token balance");
        }

        let balances = drive
            .fetch_token_total_aggregated_identity_balances_v0(token_id, None, platform_version)
            .expect("expected fetch to succeed");

        assert_eq!(balances, Some(1_000));
    }
}
