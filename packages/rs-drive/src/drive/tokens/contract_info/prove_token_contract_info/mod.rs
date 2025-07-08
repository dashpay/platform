mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Generates a proof for the token contract info from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_id` - A token ID whose contract info is to be proved.
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
    pub fn prove_token_contract_info(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .prove
            .token_contract_info
        {
            0 => self.prove_token_contract_info_v0(token_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_token_contract_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
