use grovedb::TransactionArg;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::fee::calculate_fee;
use dpp::state_transition::fee::fee_result::FeeResult;
use dpp::version::drive_versions::DriveVersion;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fee::op::LowLevelDriveOperation;

impl Drive {
    /// Fetches the Identity's balance from the backing store
    /// If the balance is 0, then also provide debt
    pub(super) fn fetch_identity_balance_include_debt_v0(
        &self,
        identity_id: [u8; 32],
        transaction: TransactionArg,
        drive_version: &DriveVersion,
    ) -> Result<Option<SignedCredits>, Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        self.fetch_identity_balance_include_debt_operations_v0(
            identity_id,
            true,
            transaction,
            &mut drive_operations,
            drive_version,
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
        drive_version: &DriveVersion,
    ) -> Result<(Option<SignedCredits>, FeeResult), Error> {
        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];
        let value = self.fetch_identity_balance_include_debt_operations_v0(
            identity_id,
            apply,
            transaction,
            &mut drive_operations,
            drive_version,
        )?;
        let fees = calculate_fee(None, Some(drive_operations), &block_info.epoch)?;
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
        drive_version: &DriveVersion,
    ) -> Result<Option<SignedCredits>, Error> {
        Ok(self
            .fetch_identity_balance_operations(identity_id, apply, transaction, drive_operations, drive_version)?
            .map(|credits| {
                if credits > 0 {
                    Ok::<Option<SignedCredits>, Error>(Some(credits.to_signed()?))
                } else {
                    self.fetch_identity_negative_balance_operations(
                        identity_id,
                        apply,
                        transaction,
                        drive_operations,
                        drive_version,
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