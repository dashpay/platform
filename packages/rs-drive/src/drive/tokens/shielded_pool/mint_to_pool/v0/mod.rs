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
    /// Version 0: raise the total supply, then record the dummy nullifiers, append the notes
    /// and credit the pool balance.
    /// Conservation holds because the supply and the pool grow by the same amount.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn token_mint_to_pool_operations_v0(
        &self,
        token_id: [u8; 32],
        amount: TokenAmount,
        allow_first_mint: bool,
        notes: &[ShieldedActionNote],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        let (mut drive_operations, _new_supply) = self.add_to_token_total_supply_operations(
            token_id,
            amount,
            allow_first_mint,
            false,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?;

        // An outputs-only bundle spends nothing, but its dummy nullifiers are recorded all the
        // same. Each note takes its `rho` from its action's dummy nullifier, so the same bundle
        // entering this pool again would land notes with the same commitments and nullifiers;
        // the record is what lets validation refuse it.
        let dummy_nullifiers: Vec<[u8; 32]> = notes.iter().map(|note| note.nullifier).collect();
        drive_operations.extend(self.token_shielded_pool_update_operations(
            token_id,
            TokenPoolBalanceChange::Add(amount),
            &dummy_nullifiers,
            notes,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )?);

        Ok(drive_operations)
    }
}
