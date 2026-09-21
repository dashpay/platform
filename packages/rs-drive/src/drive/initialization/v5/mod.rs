//! Initialization of token shielded pools in protocol version 15.

use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// Creates protocol 14 storage followed by the token shielded pools root.
    pub(super) fn create_initial_state_structure_v5(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        self.create_initial_state_structure_v4(transaction, platform_version)?;
        self.insert_token_shielded_pools_root_tree(transaction, platform_version)?;
        Ok(())
    }
}
