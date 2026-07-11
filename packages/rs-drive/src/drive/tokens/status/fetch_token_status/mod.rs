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

impl Drive {
    /// Fetches the Token status from the backing store.
    ///
    /// # Arguments
    ///
    /// * `token_id` - The ID of the token.
    /// * `transaction` - The current transaction.
    /// * `platform_version` - The platform version to use.
    ///
    /// # Returns
    ///
    /// * `Result<Option<TokenStatus>, Error>` - The token status if successful, or an error.
    pub fn fetch_token_status(
        &self,
        token_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenStatus>, Error> {
        match platform_version.drive.methods.token.fetch.token_status {
            0 => self.fetch_token_status_v0(token_id, transaction, platform_version),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_status".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }

    /// Fetches the Token status with costs (if `apply = true`) and returns associated fee result.
    pub fn fetch_token_status_with_costs(
        &self,
        token_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<TokenStatus>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_token_status_operations(
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

    /// Creates the operations to get Token's status from the backing store.
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
    pub fn fetch_token_status_operations(
        &self,
        token_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<TokenStatus>, Error> {
        match platform_version.drive.methods.token.fetch.token_status {
            0 => self.fetch_token_status_operations_v0(
                token_id,
                apply,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_token_status_operations".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::tokens::status::v0::TokenStatusV0;

    /// fetch_token_status_with_costs returns (value, FeeResult) and the FeeResult
    /// must be non-trivial when a real stateful fetch runs against existing data.
    #[test]
    fn with_costs_returns_fee_result_for_existing_token() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let token_id = [0x33u8; 32];
        let paused = TokenStatus::V0(TokenStatusV0 { paused: true });
        drive
            .token_apply_status(
                token_id,
                paused.clone(),
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("apply status");

        let (value, fees) = drive
            .fetch_token_status_with_costs(
                token_id,
                &BlockInfo::default(),
                true,
                None,
                platform_version,
            )
            .expect("fetch with costs");

        assert_eq!(value, Some(paused));
        // A real stateful fetch should have accumulated storage or processing fees.
        assert!(fees.processing_fee > 0 || fees.storage_fee > 0);
    }

    /// apply=false (stateless) returns a FeeResult with only estimated costs
    /// (no real IO), and the value is None when nothing exists.
    #[test]
    fn with_costs_stateless_missing_token_returns_none() {
        let drive = setup_drive_with_initial_state_structure(None);
        let platform_version = PlatformVersion::latest();

        let (value, _fees) = drive
            .fetch_token_status_with_costs(
                [0xFFu8; 32],
                &BlockInfo::default(),
                false, // stateless → stateless direct query + no IO
                None,
                platform_version,
            )
            .expect("stateless fetch");
        assert_eq!(value, None);
    }
}
