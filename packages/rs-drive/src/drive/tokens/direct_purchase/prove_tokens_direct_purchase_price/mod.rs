mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Proves direct purchase prices for one or more tokens.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs whose prices are to be proved.
    /// * `transaction` - The current transaction context.
    /// * `platform_version` - The version of the platform to use for compatibility checks.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<u8>, Error>` - A grovedb proof, or an error.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn prove_tokens_direct_purchase_price(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .prove
            .token_direct_purchase_prices
        {
            0 => {
                self.prove_tokens_direct_purchase_price_v0(token_ids, transaction, platform_version)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_tokens_direct_purchase_price".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
