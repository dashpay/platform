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
    /// Version 0: debit the identity, then append the notes and credit the pool balance.
    /// The token's total supply is unchanged: the pool balance is a conservation-side term.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_shield_operations_v0(
        &self,
        token_id: [u8; 32],
        from_identity_id: [u8; 32],
        amount: TokenAmount,
        notes: &[ShieldedActionNote],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let mut drive_operations = self.remove_from_identity_token_balance_operations(
            token_id,
            from_identity_id,
            amount,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        // An outputs-only bundle spends nothing: its dummy nullifiers are never recorded.
        drive_operations.extend(self.token_shielded_pool_update_operations(
            token_id,
            TokenPoolBalanceChange::Add(amount),
            &[],
            notes,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?);

        Ok(drive_operations)
    }
}
