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
        let token_id = [99u8; 32];

        let supply = drive
            .fetch_token_total_supply_v0(token_id, None, platform_version)
            .expect("expected fetch to succeed");
        assert_eq!(supply, None);
    }

    #[test]
    fn should_return_zero_for_freshly_created_token() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [11u8; 32];
        let contract_id = Identifier::from([12u8; 32]);

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

        let supply = drive
            .fetch_token_total_supply_v0(token_id, None, platform_version)
            .expect("expected fetch to succeed");
        assert_eq!(supply, Some(0));
    }

    #[test]
    fn should_return_supply_after_additions() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [13u8; 32];
        let contract_id = Identifier::from([14u8; 32]);

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
            .add_to_token_total_supply(
                token_id,
                7_500,
                false,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to add supply");

        let supply = drive
            .fetch_token_total_supply_v0(token_id, None, platform_version)
            .expect("expected fetch to succeed");
        assert_eq!(supply, Some(7_500));
    }

    #[test]
    fn should_populate_estimated_costs_in_stateless_mode() {
        use grovedb::batch::KeyInfoPath;
        use grovedb::EstimatedLayerInformation;
        use std::collections::HashMap;

        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let token_id = [17u8; 32];

        // Stateless mode: we pass Some(HashMap::new()) so the estimation branch runs.
        let mut estimated_costs: Option<HashMap<KeyInfoPath, EstimatedLayerInformation>> =
            Some(HashMap::new());
        let mut drive_operations = vec![];

        let result = drive.fetch_token_total_supply_add_to_operations_v0(
            token_id,
            &mut estimated_costs,
            None,
            &mut drive_operations,
            platform_version,
        );

        // Even without a stored entry, stateless mode should not panic.
        // It either returns Ok(Some(0)) or Ok(None); importantly it populates estimation info.
        assert!(result.is_ok());
        let estimated_costs = estimated_costs.expect("estimation state must persist");
        assert!(
            !estimated_costs.is_empty(),
            "expected stateless path to populate estimation layer info"
        );
    }

    #[test]
    fn should_return_supply_with_cost() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [15u8; 32];
        let contract_id = Identifier::from([16u8; 32]);

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
            .add_to_token_total_supply(
                token_id,
                42,
                false,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to add supply");

        let (supply, fees) = drive
            .fetch_token_total_supply_with_cost_v0(token_id, &block_info, None, platform_version)
            .expect("expected fetch with cost to succeed");

        assert_eq!(supply, Some(42));
        // At minimum, fetching costs something (read ops or storage)
        assert!(
            fees.processing_fee > 0 || fees.storage_fee > 0,
            "expected non-zero fees for a fetch"
        );
    }
}
