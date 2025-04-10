mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::tokens::status::TokenStatus;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;
use std::collections::BTreeMap;

impl Drive {
    /// Fetches token statuses from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs whose infos are to be fetched.
    /// * `transaction` - The current transaction context.
    /// * `platform_version` - The version of the platform to use for compatibility checks.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<[u8; 32], Option<IdentityTokenInfo>>, Error>` - A map of token IDs to their corresponding infos, or an error.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn fetch_token_statuses(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<TokenStatus>>, Error> {
        match platform_version.drive.methods.token.fetch.token_statuses {
            0 => self.fetch_token_statuses_v0(token_ids, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_statuses".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the identity's token infos with associated costs.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs to fetch the infos for.
    /// * `block_info` - Information about the current block for fee calculation.
    /// * `transaction` - The current transaction context.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<((BTreeMap<[u8; 32], Option<TokenAmount>>), FeeResult), Error>` - A tuple containing a map of token infos and the associated fee result.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    // TODO: Use type alias or struct
    #[allow(clippy::type_complexity)]
    pub fn fetch_token_statuses_with_costs(
        &self,
        token_ids: &[[u8; 32]],
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(BTreeMap<[u8; 32], Option<TokenStatus>>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_token_statuses_operations(
            token_ids,
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

    /// Creates the low-level operations needed to fetch the identity's token infos from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_ids` - A list of token IDs to query the infos for.
    /// * `transaction` - The current transaction context.
    /// * `drive_operations` - A vector to store the created low-level drive operations.
    /// * `platform_version` - The platform version to use for compatibility checks.
    ///
    /// # Returns
    ///
    /// * `Result<BTreeMap<[u8; 32], Option<TokenStatus>>, Error>` - A map of token IDs to their corresponding statuses.
    ///
    /// # Errors
    ///
    /// * `DriveError::UnknownVersionMismatch` - If the platform version does not support the requested operation.
    pub fn fetch_token_statuses_operations(
        &self,
        token_ids: &[[u8; 32]],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<BTreeMap<[u8; 32], Option<TokenStatus>>, Error> {
        match platform_version.drive.methods.token.fetch.token_statuses {
            0 => self.fetch_token_statuses_operations_v0(
                token_ids,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_statuses_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
