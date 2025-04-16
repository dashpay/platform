mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::tokens::token_pricing_schedule::TokenPricingSchedule;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches token direct purchase prices from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs whose direct purchase prices are to be fetched.
    /// * `transaction` - The current transaction context.
    /// * `platform_version` - The version of the platform to use for compatibility checks.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<[u8; 32], Option<TokenPricingSchedule>>, Error>` - A map of token IDs to their corresponding pricing schedules, or an error.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn fetch_tokens_direct_purchase_price(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<TokenPricingSchedule>>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .token_direct_purchase_prices
        {
            0 => {
                self.fetch_tokens_direct_purchase_price_v0(token_ids, transaction, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_tokens_direct_purchase_price".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
