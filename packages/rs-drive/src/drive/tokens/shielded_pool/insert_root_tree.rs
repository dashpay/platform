use crate::drive::tokens::paths::{tokens_root_path, TOKEN_SHIELDED_POOLS_KEY};
use crate::drive::Drive;
use crate::error::Error;
use dpp::version::PlatformVersion;
use grovedb::{Element, TransactionArg};
use grovedb_path::SubtreePath;

impl Drive {
    /// Inserts the token shielded pools root tree `[Tokens, TOKEN_SHIELDED_POOLS_KEY]` (an empty
    /// BigSumTree) if it does not exist yet.
    ///
    /// CONSENSUS-CRITICAL: both the genesis path (`Drive::create_initial_state_structure_v4`)
    /// and the in-place upgrade path (`Platform::transition_to_version_14`) call this one helper
    /// so a chain born at protocol version 14 and a chain upgraded to it build a byte-identical
    /// `[Tokens]` subtree. The tree stays empty until a token with a shielded pool is registered.
    pub fn insert_token_shielded_pools_root_tree(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> Result<(), Error> {
        let tokens_root = tokens_root_path();
        self.grove_insert_if_not_exists(
            SubtreePath::from(&tokens_root),
            &[TOKEN_SHIELDED_POOLS_KEY],
            Element::empty_big_sum_tree(),
            transaction,
            None,
            &platform_version.drive,
        )?;
        Ok(())
    }
}
