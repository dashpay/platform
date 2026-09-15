use crate::drive::tokens::shielded_pool::TokenPoolBalanceChange;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::balances::credits::TokenAmount;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Version 0: record the spent nullifiers, append the change notes, debit the pool balance,
    /// then credit the recipient. The token's total supply is unchanged.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_unshield_operations_v0(
        &self,
        token_id: [u8; 32],
        recipient_id: [u8; 32],
        amount: TokenAmount,
        nullifiers: &[[u8; 32]],
        notes: &[ShieldedActionNote],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = self.token_shielded_pool_update_operations(
            token_id,
            TokenPoolBalanceChange::Remove(amount),
            nullifiers,
            notes,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        drive_operations.extend(self.add_to_identity_token_balance_operations(
            token_id,
            recipient_id,
            amount,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?);

        Ok(drive_operations)
    }
}
