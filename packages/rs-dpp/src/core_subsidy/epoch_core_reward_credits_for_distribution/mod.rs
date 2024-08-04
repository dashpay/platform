mod v0;

use crate::core_subsidy::epoch_core_reward_credits_for_distribution::v0::epoch_core_reward_credits_for_distribution_v0;
use crate::fee::Credits;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

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
    core_subsidy_halving_interval: u32,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version
        .dpp
        .methods
        .epoch_core_reward_credits_for_distribution
    {
        0 => Ok(epoch_core_reward_credits_for_distribution_v0(
            epoch_start_block_core_height,
            next_epoch_start_block_core_height,
            core_subsidy_halving_interval,
        )),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "epoch_core_reward_credits_for_distribution".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
