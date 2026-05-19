use crate::drive::balances::{total_tokens_root_supply_path, total_tokens_root_supply_path_vec};
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::fees::op::LowLevelDriveOperation::GroveOperation;
use crate::util::grove_operations::DirectQueryType;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::batch::{KeyInfoPath, QualifiedGroveDbOp};
use grovedb::Element::SumItem;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_to_token_total_supply_v0(
        &self,
        token_id: [u8; 32],
        amount: TokenAmount,
        allow_first_mint: bool,
        allow_saturation: bool,
        apply: bool,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(FeeResult, TokenAmount), Error> {
        let mut drive_operations = vec![];

        let token_amount = self.add_to_token_total_supply_add_to_operations_v0(
            token_id,
            amount,
            allow_first_mint,
            allow_saturation,
            apply,
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

        Ok((fees, token_amount))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_to_token_total_supply_add_to_operations_v0(
        &self,
        token_id: [u8; 32],
        amount: TokenAmount,
        allow_first_mint: bool,
        allow_saturation: bool,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<TokenAmount, Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let (batch_operations, token_amount) = self.add_to_token_total_supply_operations_v0(
            token_id,
            amount,
            allow_first_mint,
            allow_saturation,
            &mut estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        self.apply_batch_low_level_drive_operations(
            estimated_costs_only_with_layer_info,
            transaction,
            batch_operations,
            drive_operations,
            &platform_version.drive,
        )?;
        Ok(token_amount)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_to_token_total_supply_operations_v0(
        &self,
        token_id: [u8; 32],
        amount: u64,
        allow_first_mint: bool,
        allow_saturation: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<LowLevelDriveOperation>, TokenAmount), Error> {
        let mut drive_operations = vec![];

        // If we only estimate, add estimation costs
        if let Some(estimated_costs_only_with_layer_info) = estimated_costs_only_with_layer_info {
            // Add your estimation logic similar to add_to_system_credits_operations_v0
            // For example:
            Self::add_estimation_costs_for_token_total_supply(
                estimated_costs_only_with_layer_info,
                &platform_version.drive,
            )?;
        }

        let path_holding_total_token_supply = total_tokens_root_supply_path();
        let path_holding_total_token_supply_vec = total_tokens_root_supply_path_vec();
        let total_token_supply_in_platform = self.grove_get_raw_value_u64_from_encoded_var_vec(
            (&path_holding_total_token_supply).into(),
            &token_id,
            DirectQueryType::StatefulDirectQuery,
            transaction,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        let added_amount =
            if let Some(total_token_supply_in_platform) = total_token_supply_in_platform {
                let new_total = if allow_saturation {
                    (total_token_supply_in_platform as i64).saturating_add(amount as i64)
                } else {
                    (total_token_supply_in_platform as i64)
                        .checked_add(amount as i64)
                        .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                            "trying to add an amount that would overflow total supply",
                        )))?
                };
                let replace_op = QualifiedGroveDbOp::replace_op(
                    path_holding_total_token_supply_vec,
                    token_id.to_vec(),
                    SumItem(new_total, None),
                );
                drive_operations.push(GroveOperation(replace_op));
                new_total as u64 - total_token_supply_in_platform
            } else if allow_first_mint {
                if amount > i64::MAX as u64 {
                    return Err(Error::Drive(DriveError::CriticalCorruptedState(
                        "amount is over max allowed in Sum Item (i64::Max)",
                    )));
                }
                let insert_op = QualifiedGroveDbOp::insert_only_known_to_not_already_exist_op(
                    path_holding_total_token_supply_vec,
                    token_id.to_vec(),
                    SumItem(amount as i64, None),
                );
                drive_operations.push(GroveOperation(insert_op));
                amount
            } else {
                return Err(Error::Drive(DriveError::CriticalCorruptedState(
                    "Total supply for token not found in Platform",
                )));
            };

        Ok((drive_operations, added_amount))
    }
}

#[cfg(test)]
mod tests {
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_add_to_existing_token_total_supply() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [1u8; 32];
        let contract_id = Identifier::from([3u8; 32]);

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

        // create_token_trees initializes supply to 0 — a subsequent call requires
        // an existing supply entry so the replace_op path is exercised.
        let (_fees, added) = drive
            .add_to_token_total_supply_v0(
                token_id,
                500,
                false,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to add to total supply");
        assert_eq!(added, 500);

        let supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("expected to fetch supply");
        assert_eq!(supply, Some(500));

        // Add more to exercise the replace path against a non-zero prior value
        let (_fees, added2) = drive
            .add_to_token_total_supply_v0(
                token_id,
                250,
                false,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to add to total supply again");
        assert_eq!(added2, 250);

        let supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("expected to fetch supply");
        assert_eq!(supply, Some(750));
    }

    #[test]
    fn should_error_when_adding_to_non_existent_token_without_allow_first_mint() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [7u8; 32];

        // No token tree created — supply does not exist, allow_first_mint=false -> error
        let result = drive.add_to_token_total_supply_v0(
            token_id,
            100,
            false, // allow_first_mint
            false,
            true,
            &block_info,
            None,
            platform_version,
        );

        assert!(
            result.is_err(),
            "expected CriticalCorruptedState error when adding to non-existent supply"
        );
    }

    #[test]
    fn should_error_on_overflow_when_allow_saturation_is_false() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [2u8; 32];
        let contract_id = Identifier::from([4u8; 32]);

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

        // Seed with a large but valid value
        drive
            .add_to_token_total_supply_v0(
                token_id,
                (i64::MAX as u64) - 10,
                false,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to add a large seed supply");

        // Now try to add enough to overflow i64 — without saturation this must error
        let result = drive.add_to_token_total_supply_v0(
            token_id,
            100,
            false,
            false, // allow_saturation
            true,
            &block_info,
            None,
            platform_version,
        );

        assert!(
            result.is_err(),
            "expected overflow error when allow_saturation is false"
        );
    }

    #[test]
    fn should_saturate_on_overflow_when_allow_saturation_is_true() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [3u8; 32];
        let contract_id = Identifier::from([5u8; 32]);

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

        // Seed near i64::MAX
        let seed = (i64::MAX as u64) - 10;
        drive
            .add_to_token_total_supply_v0(
                token_id,
                seed,
                false,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to add a large seed supply");

        // Add more than headroom — saturation path must clamp to i64::MAX
        let (_fees, added) = drive
            .add_to_token_total_supply_v0(
                token_id,
                100,
                false,
                true, // allow_saturation
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected saturation to succeed");

        // Only the headroom (10) should have been added
        assert_eq!(added, 10);

        let supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("expected to fetch supply");
        assert_eq!(supply, Some(i64::MAX as u64));
    }

    #[test]
    fn should_estimate_costs_without_mutating_state_when_apply_false() {
        // apply=false triggers the estimated_costs_only_with_layer_info branch.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [70u8; 32];
        let contract_id = Identifier::from([71u8; 32]);

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

        let app_hash_before = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        let (fees, _added) = drive
            .add_to_token_total_supply_v0(
                token_id,
                500,
                false,
                false,
                false, // apply=false -> estimation path
                &block_info,
                None,
                platform_version,
            )
            .expect("expected estimation to succeed");

        let app_hash_after = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        assert_eq!(app_hash_before, app_hash_after);
        assert!(fees.processing_fee > 0);

        // Supply unchanged (still 0)
        let supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("expected to fetch supply");
        assert_eq!(supply, Some(0));
    }

    #[test]
    fn should_report_full_added_amount_on_fresh_first_mint() {
        // First mint branch returns `amount` as added; covers the `allow_first_mint` insert path.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [80u8; 32];

        // Do NOT call create_token_trees so the supply entry is truly absent.
        let (_fees, added) = drive
            .add_to_token_total_supply_v0(
                token_id,
                1_234_567,
                true, // allow_first_mint
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected first-mint insert to succeed");

        assert_eq!(added, 1_234_567);

        let supply = drive
            .fetch_token_total_supply(token_id, None, platform_version)
            .expect("expected to fetch supply");
        assert_eq!(supply, Some(1_234_567));
    }

    #[test]
    fn should_error_when_first_mint_amount_exceeds_i64_max() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let token_id = [9u8; 32];

        // allow_first_mint=true but amount > i64::MAX -> CriticalCorruptedState
        let result = drive.add_to_token_total_supply_v0(
            token_id,
            (i64::MAX as u64) + 1,
            true, // allow_first_mint
            false,
            true,
            &block_info,
            None,
            platform_version,
        );

        assert!(
            result.is_err(),
            "expected error for first-mint amount over i64::MAX"
        );
    }
}
