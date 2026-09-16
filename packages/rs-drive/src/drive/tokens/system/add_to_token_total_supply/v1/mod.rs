use crate::drive::balances::{total_tokens_root_supply_path, total_tokens_root_supply_path_vec};
use crate::drive::tokens::lifecycle::add_to_contract_issued_supply::IssuedSupplyChange;
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
    /// Generation 1: the same supply write as v0, plus the issuer's lifecycle rollup moved by
    /// the amount actually added, in the same batch.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_to_token_total_supply_v1(
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

        let token_amount = self.add_to_token_total_supply_add_to_operations_v1(
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
    pub(super) fn add_to_token_total_supply_add_to_operations_v1(
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

        let (batch_operations, token_amount) = self.add_to_token_total_supply_operations_v1(
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
    pub(super) fn add_to_token_total_supply_operations_v1(
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
            // Add your estimation logic similar to add_to_system_credits_operations_v1
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

        // The issuer's rollup moves by the amount actually added to the supply leaf, which
        // is the saturated amount when saturation was allowed. In estimation mode the amount
        // is priced as any write of the record; a destroyed issuer is refused inside.
        drive_operations.extend(self.add_to_contract_issued_supply_operations(
            token_id,
            IssuedSupplyChange::Increase(added_amount),
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?);

        Ok((drive_operations, added_amount))
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::tokens::lifecycle::add_to_contract_issued_supply::IssuedSupplyChange;
    use crate::drive::Drive;
    use crate::error::drive::DriveError;
    use crate::error::Error;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::block::block_info::BlockInfo;
    use dpp::prelude::Identifier;
    use dpp::tokens::contract_lifecycle::v0::ContractTokenLifecycleV0Accessors;
    use dpp::version::PlatformVersion;

    fn drive_with_token() -> (Drive, Identifier, [u8; 32]) {
        let drive = setup_drive_with_initial_state_structure(None);
        let contract_id = Identifier::from([3u8; 32]);
        let token_id = [1u8; 32];
        drive
            .create_token_trees(
                contract_id,
                0,
                token_id,
                false,
                false,
                &BlockInfo::default(),
                true,
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to create token trees");
        (drive, contract_id, token_id)
    }

    fn rollup(drive: &Drive, contract_id: Identifier) -> u128 {
        drive
            .fetch_contract_token_lifecycle(
                contract_id.to_buffer(),
                None,
                PlatformVersion::latest(),
            )
            .expect("expected to read")
            .expect("expected a record")
            .issued_supply()
    }

    #[test]
    fn should_move_the_supply_and_the_rollup_together() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();

        let (_, added) = drive
            .add_to_token_total_supply(
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
        let (_, added) = drive
            .add_to_token_total_supply(
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
        assert_eq!(added, 250);

        assert_eq!(
            drive
                .fetch_token_total_supply(token_id, None, platform_version)
                .expect("expected to fetch supply"),
            Some(750)
        );
        assert_eq!(rollup(&drive, contract_id), 750);
    }

    #[test]
    fn should_move_the_rollup_by_the_saturated_amount() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();
        let seed = (i64::MAX as u64) - 10;

        drive
            .add_to_token_total_supply(
                token_id,
                seed,
                false,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to seed supply");
        let (_, added) = drive
            .add_to_token_total_supply(
                token_id,
                100,
                false,
                true,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected saturation to succeed");

        assert_eq!(added, 10);
        assert_eq!(rollup(&drive, contract_id), i64::MAX as u128);
    }

    #[test]
    fn should_error_on_overflow_when_allow_saturation_is_false() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();

        drive
            .add_to_token_total_supply(
                token_id,
                (i64::MAX as u64) - 10,
                false,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to seed supply");
        let result = drive.add_to_token_total_supply(
            token_id,
            100,
            false,
            false,
            true,
            &block_info,
            None,
            platform_version,
        );

        assert!(result.is_err());
        assert_eq!(rollup(&drive, contract_id), (i64::MAX as u128) - 10);
    }

    #[test]
    fn should_refuse_a_first_mint_of_a_token_without_an_issuer() {
        // v0 allowed the supply leaf of an unknown token to appear on first mint; with the
        // ledger a token must resolve to its issuer, so a token that was never created is
        // corrupted state.
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let result = drive.add_to_token_total_supply(
            [80u8; 32],
            1_234_567,
            true,
            false,
            true,
            &BlockInfo::default(),
            None,
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
    }

    #[test]
    fn should_refuse_a_destroyed_issuer() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();

        drive
            .destroy_token_issuer(
                contract_id.to_buffer(),
                &block_info,
                true,
                None,
                platform_version,
            )
            .expect("expected to destroy the issuer");

        let result = drive.add_to_token_total_supply(
            token_id,
            1,
            false,
            false,
            true,
            &block_info,
            None,
            platform_version,
        );

        assert!(matches!(
            result,
            Err(Error::Drive(DriveError::CorruptedDriveState(_)))
        ));
        assert_eq!(
            drive
                .fetch_token_total_supply(token_id, None, platform_version)
                .expect("expected to fetch supply"),
            Some(0)
        );
    }

    #[test]
    fn should_estimate_costs_without_mutating_state_when_apply_false() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();
        let root_hash_before = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");

        let (fees, _) = drive
            .add_to_token_total_supply(
                token_id,
                500,
                false,
                false,
                false,
                &BlockInfo::default(),
                None,
                platform_version,
            )
            .expect("expected estimation to succeed");

        assert!(fees.processing_fee > 0);
        let root_hash_after = drive
            .grove
            .root_hash(None, &platform_version.drive.grove_version)
            .unwrap()
            .expect("expected root hash");
        assert_eq!(root_hash_before, root_hash_after);
        assert_eq!(rollup(&drive, contract_id), 0);
    }

    #[test]
    fn should_price_at_least_the_applied_cost() {
        // Fees are charged from the estimate and the applied cost must never exceed it, so
        // the estimated write of the record has to cover the real one.
        let (drive, _, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();
        let block_info = BlockInfo::default();

        let (estimated, _) = drive
            .add_to_token_total_supply(
                token_id,
                500,
                false,
                false,
                false,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected an estimate");
        let (applied, _) = drive
            .add_to_token_total_supply(
                token_id,
                500,
                false,
                false,
                true,
                &block_info,
                None,
                platform_version,
            )
            .expect("expected to apply");

        assert!(
            estimated.processing_fee >= applied.processing_fee,
            "estimated {} is below applied {}",
            estimated.processing_fee,
            applied.processing_fee
        );
        assert!(estimated.storage_fee >= applied.storage_fee);
    }

    #[test]
    fn should_keep_the_rollup_change_type_in_sync_with_the_supply_delta() {
        let (drive, contract_id, token_id) = drive_with_token();
        let platform_version = PlatformVersion::latest();
        let operations = drive
            .add_to_contract_issued_supply_operations(
                token_id,
                IssuedSupplyChange::Increase(7),
                &mut None,
                None,
                platform_version,
            )
            .expect("expected operations");
        drive
            .apply_batch_low_level_drive_operations(
                None,
                None,
                operations,
                &mut vec![],
                &platform_version.drive,
            )
            .expect("expected to apply");
        assert_eq!(rollup(&drive, contract_id), 7);
    }
}
