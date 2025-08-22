use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::identifier::Identifier;
use dpp::version::PlatformVersion;
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_mint_many_v0(
        &self,
        token_id: Identifier,
        recipients: Vec<(Identifier, u64)>,
        issuance_amount: u64,
        allow_first_mint: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];

        self.token_mint_many_add_to_operations_v0(
            token_id,
            recipients,
            issuance_amount,
            allow_first_mint,
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

        Ok(fees)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_mint_many_add_to_operations_v0(
        &self,
        token_id: Identifier,
        recipients: Vec<(Identifier, u64)>,
        issuance_amount: u64,
        allow_first_mint: bool,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.token_mint_many_operations_v0(
            token_id,
            recipients,
            issuance_amount,
            allow_first_mint,
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
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_mint_many_operations_v0(
        &self,
        token_id: Identifier,
        mut recipients: Vec<(Identifier, u64)>,
        total_mint_amount: u64,
        allow_first_mint: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        let weight_sum = recipients
            .iter_mut()
            .map(|(_, weight)| {
                // We do this so we can't overflow
                if *weight > u32::MAX as u64 {
                    *weight = u32::MAX as u64
                }
                *weight
            })
            .sum::<u64>();
        let total_mint_amount_u128 = total_mint_amount as u128;

        let mut balance_left = total_mint_amount;

        for (i, (identity_id, weight)) in recipients.iter().enumerate() {
            let amount = if i == recipients.len() - 1 {
                balance_left
            } else {
                let amount = total_mint_amount_u128
                    .saturating_mul(*weight as u128)
                    .div_ceil(weight_sum as u128) as u64;
                balance_left -= amount;
                amount
            };

            drive_operations.extend(self.add_to_identity_token_balance_operations(
                token_id.to_buffer(),
                identity_id.to_buffer(),
                amount,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        // Update total supply

        drive_operations.extend(
            self.add_to_token_total_supply_operations(
                token_id.to_buffer(),
                total_mint_amount,
                allow_first_mint,
                false,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?
            .0,
        );

        Ok(drive_operations)
    }
}
