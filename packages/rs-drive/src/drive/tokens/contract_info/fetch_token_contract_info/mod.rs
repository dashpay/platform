mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::tokens::contract_info::TokenContractInfo;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Token's contract info from the backing store.
    /// Passing `apply = false` will return estimated costs (0 or Some(0) in place of actual values).
    ///
    /// # Arguments
    ///
    /// * `token_id` - The ID of the token.
    /// * `apply` - Whether to actually fetch from state (true) or estimate costs (false).
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<Option<TokenContractInfo>, Error>` - The token contract info if successful, or an error.
    pub fn fetch_token_contract_info(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenContractInfo>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .token_contract_info
        {
            0 => self.fetch_token_contract_info_v0(token_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_contract_info".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the Token's contract info with costs (if `apply = true`) and returns associated fee result.
    pub fn fetch_token_contract_info_with_costs(
        &self,
        token_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<TokenContractInfo>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_token_contract_info_operations(
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

    /// Creates the operations to get the token's contract info from the backing store.
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
    /// * `Result<Option<TokenContractInfo>, Error>` - The token info of the Identity if successful, or an error.
    pub fn fetch_token_contract_info_operations(
        &self,
        token_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenContractInfo>, Error> {
        match platform_version
            .drive
            .methods
            .token
            .fetch
            .token_contract_info
        {
            0 => self.fetch_token_contract_info_operations_v0(
                token_id,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_contract_info_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
