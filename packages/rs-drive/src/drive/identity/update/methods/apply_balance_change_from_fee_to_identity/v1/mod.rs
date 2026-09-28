use crate::drive::identity::update::add_to_previous_balance_outcome::AddToPreviousBalanceOutcomeV0;
use crate::drive::identity::update::add_to_previous_balance_outcome::AddToPreviousBalanceOutcomeV0Methods;
use crate::drive::identity::update::apply_balance_change_outcome::ApplyBalanceChangeOutcome;
use crate::drive::identity::update::apply_balance_change_outcome::ApplyBalanceChangeOutcomeV0;
use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::identity::IdentityError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::fee::fee_result::{BalanceChange, BalanceChangeForIdentity, FeeResult};

use dpp::version::PlatformVersion;
use dpp::ProtocolError;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Balances are stored in the balance tree under the identity's id
    ///
    /// Generation 1 (protocol version 14) is generation 0, and the credits that repaid a debt,
    /// of the payer or of an identity a refund went to, reach the processing fee pool of the
    /// block's epoch in the same batch. They are not part of the fee paid: that is still what
    /// generation 0 reports.
    pub(super) fn apply_balance_change_from_fee_to_identity_v1(
        &self,
        balance_change: BalanceChangeForIdentity,
        block_info: &BlockInfo,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<ApplyBalanceChangeOutcome, Error> {
        let (mut batch_operations, actual_fee_paid) = self
            .apply_balance_change_from_fee_to_identity_operations_v1(
                balance_change,
                transaction,
                platform_version,
            )?;

        let repaid_debt = LowLevelDriveOperation::take_repaid_identity_debt(&mut batch_operations)?;
        // Nothing else in this batch writes a fee pool, so the pool credit shares it
        if repaid_debt > 0 {
            batch_operations.push(
                self.add_epoch_processing_credits_for_distribution_operation(
                    &block_info.epoch,
                    repaid_debt,
                    transaction,
                    platform_version,
                )?,
            );
        }

        let mut drive_operations: Vec<LowLevelDriveOperation> = vec![];

        self.apply_batch_low_level_drive_operations(
            None,
            transaction,
            batch_operations,
            &mut drive_operations,
            &platform_version.drive,
        )?;

        Ok(ApplyBalanceChangeOutcomeV0 { actual_fee_paid }.into())
    }

    /// Applies a balance change based on Fee Result
    /// If calculated balance is below 0 it will go to negative balance
    ///
    /// Balances are stored in the identity under key 0
    ///
    /// Generation 1 (protocol version 14) is generation 0, and the operations end with a
    /// [`LowLevelDriveOperation::RepaidIdentityDebt`] for every identity whose incoming credits
    /// repaid its debt: the payer, when its refunds exceed its fee, and each identity another
    /// refund goes to (`add_to_identity_balance_operations` 1). Whoever applies them owes those
    /// credits to the current epoch's processing fee pool.
    #[inline(always)]
    pub(super) fn apply_balance_change_from_fee_to_identity_operations_v1(
        &self,
        balance_change: BalanceChangeForIdentity,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(Vec<LowLevelDriveOperation>, FeeResult), Error> {
        let mut drive_operations = vec![];

        if matches!(balance_change.change(), BalanceChange::NoBalanceChange) {
            // The payer's own refund covers its fee exactly, so its balance stays as it is. The
            // fee result returned here still takes the refunds owed to other identities out of
            // the storage pools, so they are credited as in every other branch.
            self.add_other_refunds_to_identity_balances_operations_v1(
                &balance_change,
                &mut drive_operations,
                transaction,
                platform_version,
            )?;
            return Ok((drive_operations, balance_change.into_fee_result()));
        }

        // Update identity's balance according to calculated fees
        let previous_balance = self
            .fetch_identity_balance_operations(
                balance_change.identity_id.to_buffer(),
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?
            .ok_or(Error::Drive(DriveError::CorruptedCodeExecution(
                "there should always be a balance if apply is set to true",
            )))?;

        let add_to_previous_balance_outcome = match balance_change.change() {
            BalanceChange::AddToBalance(balance_to_add) => self.add_to_previous_balance(
                balance_change.identity_id.to_buffer(),
                previous_balance,
                *balance_to_add,
                true,
                transaction,
                &mut drive_operations,
                platform_version,
            )?,
            BalanceChange::RemoveFromBalance {
                required_removed_balance,
                desired_removed_balance,
            } => {
                if *desired_removed_balance > previous_balance {
                    // we do not have enough balance
                    // there is a part we absolutely need to pay for
                    if *required_removed_balance > previous_balance {
                        return Err(Error::Identity(IdentityError::IdentityInsufficientBalance(
                            format!(
                                "identity with balance {} does not have the required balance {}",
                                previous_balance, *required_removed_balance
                            ),
                        )));
                    }
                    AddToPreviousBalanceOutcomeV0 {
                        balance_modified: Some(0),
                        negative_credit_balance_modified: Some(
                            *desired_removed_balance - previous_balance,
                        ),
                        repaid_debt: 0,
                    }
                    .into()
                } else {
                    // we have enough balance
                    AddToPreviousBalanceOutcomeV0 {
                        balance_modified: Some(previous_balance - desired_removed_balance),
                        negative_credit_balance_modified: None,
                        repaid_debt: 0,
                    }
                    .into()
                }
            }
            BalanceChange::NoBalanceChange => {
                return Err(Error::Drive(DriveError::CorruptedCodeExecution(
                    "a balance change with no change returned above",
                )))
            }
        };

        if let Some(new_balance) = add_to_previous_balance_outcome.balance_modified() {
            drive_operations.push(self.update_identity_balance_operation_v0(
                balance_change.identity_id.to_buffer(),
                new_balance,
            )?);
        }

        if let Some(new_negative_balance) =
            add_to_previous_balance_outcome.negative_credit_balance_modified()
        {
            drive_operations.push(self.update_identity_negative_credit_operation_v0(
                balance_change.identity_id.to_buffer(),
                new_negative_balance,
            ));
        }

        let repaid_debt = add_to_previous_balance_outcome.repaid_debt();
        if repaid_debt > 0 {
            drive_operations.push(LowLevelDriveOperation::RepaidIdentityDebt(repaid_debt));
        }

        self.add_other_refunds_to_identity_balances_operations_v1(
            &balance_change,
            &mut drive_operations,
            transaction,
            platform_version,
        )?;

        Ok((
            drive_operations,
            balance_change
                .fee_result_outcome::<ConsensusError>(previous_balance)
                .map_err(|e| ProtocolError::ConsensusError(Box::new(e)))?,
        ))
    }

    /// Credits the storage refunds a fee result owes identities other than its payer. Each
    /// credit that repays a debt leaves a [`LowLevelDriveOperation::RepaidIdentityDebt`] among
    /// the operations.
    fn add_other_refunds_to_identity_balances_operations_v1(
        &self,
        balance_change: &BalanceChangeForIdentity,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        for (identity_id, credits) in balance_change.other_refunds() {
            let mut estimated_costs_only_with_layer_info =
                None::<HashMap<KeyInfoPath, EstimatedLayerInformation>>;

            drive_operations.extend(self.add_to_identity_balance_operations(
                identity_id.to_buffer(),
                credits,
                &mut estimated_costs_only_with_layer_info,
                transaction,
                platform_version,
            )?);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::drive::identity::update::apply_balance_change_outcome::ApplyBalanceChangeOutcomeV0Methods;
    use crate::drive::identity::update::methods::debt_test_helpers::{
        balance, debt_test_block_info as block_info, indebted_identity, processing_pool,
    };
    use crate::util::test_helpers::setup::setup_drive_with_initial_state_structure;
    use dpp::fee::epoch::GENESIS_EPOCH_INDEX;
    use dpp::fee::fee_result::refunds::{CreditsPerEpochByIdentifier, FeeRefunds};
    use dpp::fee::fee_result::{BalanceChange, FeeResult};
    use dpp::fee::Credits;
    use dpp::version::PlatformVersion;
    use nohash_hasher::IntMap;
    use std::collections::BTreeMap;

    fn refunds(entries: [([u8; 32], Credits); 2]) -> FeeRefunds {
        let refunds_per_epoch_by_identifier: CreditsPerEpochByIdentifier =
            BTreeMap::from_iter(entries.map(|(identity_id, credits)| {
                (
                    identity_id,
                    IntMap::from_iter([(GENESIS_EPOCH_INDEX, credits)]),
                )
            }));
        FeeRefunds(refunds_per_epoch_by_identifier)
    }

    #[test]
    fn should_credit_the_debts_that_the_payer_and_another_refund_repay_to_the_processing_fee_pool()
    {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let payer = indebted_identity(&drive, [1; 32], 100, platform_version);
        let other = indebted_identity(&drive, [2; 32], 5_000, platform_version);

        // The payer's refund exceeds its fee by 200_000, and 20_000 goes to another identity
        let fee_result = FeeResult {
            storage_fee: 30_000,
            processing_fee: 70_000,
            fee_refunds: refunds([(payer, 300_000), (other, 20_000)]),
            ..Default::default()
        };
        let balance_change = fee_result.clone().into_balance_change(payer.into());
        assert_eq!(
            balance_change.change(),
            &BalanceChange::AddToBalance(200_000)
        );

        let outcome = drive
            .apply_balance_change_from_fee_to_identity(
                balance_change,
                &block_info(),
                None,
                platform_version,
            )
            .expect("expected to apply the balance change");

        assert_eq!(balance(&drive, payer, platform_version), 199_900);
        assert_eq!(balance(&drive, other, platform_version), 15_000);
        assert_eq!(processing_pool(&drive, platform_version), 5_100);
        // The repaid debts are not part of the fee this transition paid
        assert_eq!(outcome.actual_fee_paid(), &fee_result);
    }

    #[test]
    fn should_credit_the_debt_another_refund_repays_when_the_payers_balance_does_not_change() {
        let platform_version = PlatformVersion::latest();
        let drive = setup_drive_with_initial_state_structure(None);
        let payer = indebted_identity(&drive, [1; 32], 100, platform_version);
        let other = indebted_identity(&drive, [2; 32], 5_000, platform_version);

        // The payer's refund equals its fee exactly
        let fee_result = FeeResult {
            storage_fee: 30_000,
            processing_fee: 70_000,
            fee_refunds: refunds([(payer, 100_000), (other, 20_000)]),
            ..Default::default()
        };
        let balance_change = fee_result.into_balance_change(payer.into());
        assert_eq!(balance_change.change(), &BalanceChange::NoBalanceChange);

        drive
            .apply_balance_change_from_fee_to_identity(
                balance_change,
                &block_info(),
                None,
                platform_version,
            )
            .expect("expected to apply the balance change");

        assert_eq!(balance(&drive, payer, platform_version), 0);
        assert_eq!(balance(&drive, other, platform_version), 15_000);
        assert_eq!(processing_pool(&drive, platform_version), 5_000);
    }

    #[test]
    fn should_leave_the_processing_fee_pool_alone_at_protocol_version_13() {
        let platform_version = PlatformVersion::get(13).expect("expected protocol version 13");
        let drive = setup_drive_with_initial_state_structure(Some(platform_version));
        let payer = indebted_identity(&drive, [1; 32], 100, platform_version);
        let other = indebted_identity(&drive, [2; 32], 5_000, platform_version);

        let fee_result = FeeResult {
            storage_fee: 30_000,
            processing_fee: 70_000,
            fee_refunds: refunds([(payer, 300_000), (other, 20_000)]),
            ..Default::default()
        };
        let balance_change = fee_result.into_balance_change(payer.into());

        drive
            .apply_balance_change_from_fee_to_identity(
                balance_change,
                &block_info(),
                None,
                platform_version,
            )
            .expect("expected to apply the balance change");

        // Generation 0 repays both debts and credits them nowhere
        assert_eq!(balance(&drive, payer, platform_version), 199_900);
        assert_eq!(balance(&drive, other, platform_version), 15_000);
        assert_eq!(processing_pool(&drive, platform_version), 0);
    }
}
