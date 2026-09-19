use crate::drive::tokens::shielded_pool::TokenPoolBalanceChange;
use crate::drive::Drive;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::version::PlatformVersion;
use grovedb::batch::KeyInfoPath;
use grovedb::{EstimatedLayerInformation, TransactionArg};
use std::collections::HashMap;

impl Drive {
    /// Version 0: record the spent nullifiers and append the output notes; nothing leaves or
    /// enters the pool, so the balance is untouched.
    pub(super) fn token_shielded_transfer_operations_v0(
        &self,
        token_id: [u8; 32],
        nullifiers: &[[u8; 32]],
        notes: &[ShieldedActionNote],
        estimated_costs_only_with_layer_info: &mut Option<
            HashMap<KeyInfoPath, EstimatedLayerInformation>,
        >,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<LowLevelDriveOperation>, Error> {
        self.token_shielded_pool_update_operations(
            token_id,
            TokenPoolBalanceChange::Unchanged,
            nullifiers,
            notes,
            estimated_costs_only_with_layer_info,
            transaction,
            platform_version,
        )
    }
}
