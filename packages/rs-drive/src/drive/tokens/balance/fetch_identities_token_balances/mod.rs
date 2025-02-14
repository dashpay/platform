mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::TokenAmount;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::prelude::Identifier;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches the token balances of an identity from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs whose balances are to be fetched.
    /// * `identity_id` - The ID of the identity whose token balances are being queried.
    /// * `transaction` - The current transaction context.
    /// * `platform_version` - The version of the platform to use for compatibility checks.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<[u8; 32], Option<TokenAmount>>, Error>` - A map of token IDs to their corresponding balances, or an error.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn fetch_identities_token_balances(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, Option<TokenAmount>>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .identities_token_balances
        {
            0 => self.fetch_identities_token_balances_v0(
                token_id,
                identity_ids,
                transaction,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identity_token_balances".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the identity's token balances with associated costs.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs to fetch the balances for.
    /// * `identity_id` - The identity's ID whose balances are being queried.
    /// * `block_info` - Information about the current block for fee calculation.
    /// * `transaction` - The current transaction context.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<((BTreeMap<[u8; 32], Option<TokenAmount>>), FeeResult), Error>` - A tuple containing a map of token balances and the associated fee result.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn fetch_identities_token_balances_with_costs(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(BTreeMap<Identifier, Option<TokenAmount>>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_identities_token_balances_operations(
            token_id,
            identity_ids,
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

    /// Creates the low-level operations needed to fetch the identity's token balances from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs to query the balances for.
    /// * `identity_id` - The ID of the identity whose token balances are being queried.
    /// * `transaction` - The current transaction context.
    /// * `drive_operations` - A vector to store the created low-level drive operations.
    /// * `platform_version` - The platform version to use for compatibility checks.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<[u8; 32], Option<TokenAmount>>, Error>` - A map of token IDs to their corresponding balances, or an error.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn fetch_identities_token_balances_operations(
        &self,
        token_id: [u8; 32],
        identity_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<Identifier, Option<TokenAmount>>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .identities_token_balances
        {
            0 => self.fetch_identities_token_balances_operations_v0(
                token_id,
                identity_ids,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_identities_token_balances_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
