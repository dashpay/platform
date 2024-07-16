use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::balances::credits::Creditable;
use dpp::block::block_info::BlockInfo;
use dpp::fee::fee_result::FeeResult;
use dpp::fee::SignedCredits;

use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Fetches the Identity's balance from the backing store
    /// If the balance is 0, then also provide debt
    pub(super) fn fetch_identity_balance_include_debt_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Option<SignedCredits>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_balance_include_debt_operations_v0(
            identity_id,
            true,
            transaction,
            &mut drive_operations,
            platform_version,
        )
    }

    /// Fetches the Identity's balance from the backing store
    /// If the balance is 0, then also provide debt
    /// Also returns the costs of the operation
    pub(super) fn fetch_identity_balance_include_debt_with_costs_v0(
        &self,
        identity_id: [u8; 32],
        block_info: &BlockInfo,
        apply: bool,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Option<SignedCredits>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_identity_balance_include_debt_operations_v0(
            identity_id,
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

    /// Fetches the Identity's balance from the backing store
    /// If the balance is 0, then also provide debt
    pub(super) fn fetch_identity_balance_include_debt_operations_v0(
        &self,
        identity_id: [u8; 32],
        apply: bool,
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<Option<SignedCredits>, Error> {
        Ok(self
            .fetch_identity_balance_operations(
                identity_id,
                apply,
                transaction,
                drive_operations,
                platform_version,
            )?
            .map(|credits| {
                if credits > 0 {
                    Ok::<Option<SignedCredits>, Error>(Some(credits.to_signed()?))
                } else {
                    self.fetch_identity_negative_balance_operations(
                        identity_id,
                        apply,
                        transaction,
                        drive_operations,
                        platform_version,
                    )
                    .map(|negative_credits| {
                        let negative_credits = negative_credits.ok_or(Error::Drive(
                            DriveError::CorruptedDriveState(
                                "Identity has balance but no negative credit holder".to_string(),
                            ),
                        ))?;
                        Ok(Some(-negative_credits.to_signed()?))
                    })?
                }
            })
            .transpose()?
            .flatten())
    }
}
