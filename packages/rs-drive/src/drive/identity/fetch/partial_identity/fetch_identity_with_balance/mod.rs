mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::fee::fee_result::FeeResult;

use dpp::identity::PartialIdentity;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Identity's balance as `PartialIdentityInfo` from the backing store.
    ///
    /// This method selects the appropriate version of the function to call based on the
    /// provided platform version.
    ///
    /// # Parameters
    ///
    /// - `identity_id`: A 32-byte array representing the ID of the identity.
    /// - `transaction`: A transaction argument for the database.
    /// - `platform_version`: The platform version being used.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing an `Option<PartialIdentity>`.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform version is not recognized or if there's a failure
    /// during the operation.
    pub fn fetch_identity_with_balance(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<PartialIdentity>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .partial_identity
            .fetch_identity_with_balance
        {
            0 => self.fetch_identity_with_balance_v0(identity_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_with_balance".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the Identity's balance as `PartialIdentityInfo` from the backing store,
    /// and also calculates the cost of the operation.
    ///
    /// This method selects the appropriate version of the function to call based on the
    /// provided platform version.
    ///
    /// # Parameters
    ///
    /// - `identity_id`: A 32-byte array representing the ID of the identity.
    /// - `apply`: A boolean to determine if the balance should be applied or just estimated.
    /// - `transaction`: A transaction argument for the database.
    /// - `platform_version`: The platform version being used.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a tuple of `Option<PartialIdentity>` and `FeeResult`.
    ///
    /// # Errors
    ///
    /// Returns an error if the platform version is not recognized or if there's a failure
    /// during the operation.
    pub fn fetch_identity_with_balance_with_cost(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<PartialIdentity>, FeeResult), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .partial_identity
            .fetch_identity_with_balance
        {
            0 => self.fetch_identity_with_balance_with_cost_v0(
                identity_id,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_with_balance_with_cost".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
