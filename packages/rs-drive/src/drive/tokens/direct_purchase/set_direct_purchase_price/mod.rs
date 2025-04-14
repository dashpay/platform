use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use platform_version::version::PlatformVersion;
use std::collections::HashMap;
mod v0;

impl Drive {
    /// Mints (issues) new tokens by increasing the total supply and adding them to an identity's balance.
    pub fn token_set_direct_purchase_price(
        &self,
        token_id: [u8; 32],
        price: Option<TokenPricingSchedule>,
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<FeeResult, Error> {
        let mut estimated_costs_only_with_layer_info =
            if apply { None } else { Some(HashMap::new()) };

        let batch_operations = self.token_set_direct_purchase_price_operations(
            token_id,
            price,
            &mut estimated_costs_only_with_layer_info,
            platform_version,
        )?;

        let mut drive_operations = vec![];

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
            None,
        )?;

        Ok(fees)
    }

    /// Gathers the operations needed to mint tokens.
    pub fn token_set_direct_purchase_price_operations(
        &self,
        token_id: [u8; 32],
        price: Option<TokenPricingSchedule>,
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        match platform_version.drive.methods.token.update.mint {
            0 => self.token_set_direct_purchase_price_operations_v0(
                token_id,
                price,
                estimated_costs_only_with_layer_info,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "token_set_direct_purchase_price_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
