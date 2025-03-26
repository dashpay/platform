use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::version::PlatformVersion;
use grovedb::{batch::KeyInfoPath, EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_mint_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        issuance_amount: u64,
        allow_first_mint: bool,
        allow_saturation: bool,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut drive_operations = vec![];

        self.token_mint_add_to_operations_v0(
            token_id,
            identity_id,
            issuance_amount,
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

        Ok(fees)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_mint_add_to_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        issuance_amount: u64,
        allow_first_mint: bool,
        allow_saturation: bool,
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.token_mint_operations_v0(
            token_id,
            identity_id,
            issuance_amount,
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
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_mint_operations_v0(
        &self,
        token_id: [u8; 32],
        identity_id: [u8; 32],
        issuance_amount: u64,
        allow_first_mint: bool,
        allow_saturation: bool,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = vec![];

        let (add_to_supply_operations, actual_issuance_amount) = self
            .add_to_token_total_supply_operations(
                token_id,
                issuance_amount,
                allow_first_mint,
                allow_saturation,
                estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?;

        // There is a chance that we can't add more to the supply because it would overflow, in that case we issue what can be issued if allow saturation is set to true

        // Update identity balance
        drive_operations.extend(self.add_to_identity_token_balance_operations(
            token_id,
            identity_id,
            actual_issuance_amount,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?);

        drive_operations.extend(add_to_supply_operations);

        Ok(drive_operations)
    }
}
