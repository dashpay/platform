mod v0;

use crate::drive::Drive;
use crate::error::{drive::DriveError, Error};

use dpp::version::PlatformVersion;

use grovedb::TransactionArg;

impl Drive {
    /// proves a prefunded specialized balance from the backing store, respecting drive versioning.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Specialized balance whose amount is to be proved.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Credits>, Error>` - The proof if successful, or an error.
    pub fn prove_prefunded_specialized_balance(
        &self,
        prefunded_specialized_balance_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<u8>, Error> {
        match platform_version
            .drive
            .methods
            .prefunded_specialized_balances
            .prove_single
        {
            0 => self.prove_prefunded_specialized_balance_v0(
                prefunded_specialized_balance_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "prove_prefunded_specialized_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
