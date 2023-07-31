mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::execution::platform_events::core_subsidy::{
    CORE_GENESIS_BLOCK_SUBSIDY, CORE_SUBSIDY_HALVING_INTERVAL,
};
use crate::platform_types::platform::Platform;
use dpp::block::epoch::EpochIndex;
use dpp::fee::Credits;
use dpp::version::PlatformVersion;

use lazy_static::lazy_static;
use std::collections::HashMap;

impl<C> Platform<C> {
    /// Gets the amount of core reward fees to be distributed for the Epoch.
    ///
    /// # Arguments
    ///
    /// * `epoch_start_block_core_height` - The height of the core block at the start of the epoch.
    /// * `next_epoch_start_block_core_height` - The height of the core block at the start of the next epoch.
    ///
    /// # Returns
    ///
    /// * `Result<Credits, Error>` - If the operation is successful, it returns `Ok(Credits)`. If there is an error, it returns `Error`.
    pub fn epoch_core_reward_credits_for_distribution(
        epoch_start_block_core_height: u32,
        next_epoch_start_block_core_height: u32,
        platform_version: &PlatformVersion,
    ) -> Result<Credits, Error> {
        match platform_version
            .drive_abci
            .methods
            .core_subsidy
            .epoch_core_reward_credits_for_distribution
        {
            0 => Self::epoch_core_reward_credits_for_distribution_v0(
                epoch_start_block_core_height,
                next_epoch_start_block_core_height,
            ),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "epoch_core_reward_credits_for_distribution_v0".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
