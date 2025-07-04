mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Token direct_purchase_price from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The ID of the token.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<Option<TokenStatus>, Error>` - The token direct_purchase_price if successful, or an error.
    pub fn fetch_token_direct_purchase_price(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenPricingSchedule>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .token_direct_purchase_price
        {
            0 => self.fetch_token_direct_purchase_price_v0(token_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_direct_purchase_price".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the Token direct_purchase_price with costs (if `apply = true`) and returns associated fee result.
    pub fn fetch_token_direct_purchase_price_with_costs(
        &self,
        token_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<TokenPricingSchedule>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_token_direct_purchase_price_operations(
            token_id,
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

        Ok((value, fees))
    }

    /// Creates the operations to get Token's direct_purchase_price from the backing store.
    /// If `apply` is false, the operations are stateless and only used for cost estimation.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The ID of the token.
    /// * `apply` - Whether to fetch actual stateful data (true) or just estimate costs (false).
    /// * `transaction` - The current transaction.
    /// * `drive_operations` - The drive operations vector to populate.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<Option<TokenStatus>, Error>` - The token info of the Identity if successful, or an error.
    pub fn fetch_token_direct_purchase_price_operations(
        &self,
        token_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenPricingSchedule>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .token_direct_purchase_price
        {
            0 => self.fetch_token_direct_purchase_price_operations_v0(
                token_id,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_direct_purchase_price_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
