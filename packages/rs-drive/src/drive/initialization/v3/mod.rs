//! Drive Initialization v3 — adds shielded pool trees.

use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Creates the initial state structure (v3).
    ///
    /// Extends v2 by initializing shielded pool trees (commitment tree,
    /// nullifier set, anchor history).
    pub(super) fn create_initial_state_structure_v3(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        // v3 currently delegates to v2.
        // Shielded pool tree initialization will be added here once the
        // RootTree variants for the shielded pool are defined.
        self.create_initial_state_structure_v2(transaction, platform_version)
    }
}
