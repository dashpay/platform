mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::SignedCredits;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Identity's balance from the backing store.
    /// If the balance is 0, then also provide debt, respecting drive versioning.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose balance is to be fetched.
    /// * `transaction` - The current transaction.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<Option<SignedCredits>, Error>` - The balance of the Identity if successful, or an error.
    pub fn fetch_identity_balance_include_debt(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<SignedCredits>, Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .attributes
            .balance
        {
            0 => self.fetch_identity_balance_include_debt_v0(
                identity_id,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_balance_include_debt".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the Identity's balance from the backing store, including the estimated cost.
    /// If the balance is 0, then also provide debt, respecting drive versioning.
    ///
    /// # Arguments
    ///
    /// * `identity_id` - The ID of the Identity whose balance is to be fetched.
    /// * `block_info` - The information about the current block.
    /// * `apply` - Whether to get the estimated cost or the actual balance.
    /// * `transaction` - The current transaction.
    /// * `drive_version` - The drive version.
    ///
    /// # Returns
    ///
    /// * `Result<(Option<SignedCredits>, FeeResult), Error>` - The balance of the Identity and the fee if successful, or an error.
    pub fn fetch_identity_balance_include_debt_with_costs(
        &self,
        identity_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<SignedCredits>, FeeResult), Error> {
        match platform_version
            .drive
            .methods
            .identity
            .fetch
            .attributes
            .balance
        {
            0 => self.fetch_identity_balance_include_debt_with_costs_v0(
                identity_id,
                block_info,
                apply,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_balance_include_debt_with_costs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
