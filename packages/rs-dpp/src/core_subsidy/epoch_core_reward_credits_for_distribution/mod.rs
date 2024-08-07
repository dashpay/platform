mod v0;

use crate::core_subsidy::epoch_core_reward_credits_for_distribution::v0::epoch_core_reward_credits_for_distribution_v0;
use crate::fee::Credits;
use crate::ProtocolError;
use platform_version::version::PlatformVersion;

/// Gets the amount of core reward fees to be distributed for the Epoch.
///
/// # Arguments
///
/// * `from_start_block_core_height` - The height of the core block at the start of the epoch.
/// * `to_end_block_core_height_included` - The height of the core block at the start of the next epoch.
/// * `core_subsidy_halving_interval` - The halving interval set by the Core network.
/// * `platform_version` - The platform version.
///
/// # Returns
///
/// * `Result<Credits, Error>` - If the operation is successful, it returns `Ok(Credits)`. If there is an error, it returns `Error`.
pub fn epoch_core_reward_credits_for_distribution(
    from_start_block_core_height: u32,
    to_end_block_core_height_included: u32,
    core_subsidy_halving_interval: u32,
    platform_version: &PlatformVersion,
) -> Result<Credits, ProtocolError> {
    match platform_version
        .dpp
        .methods
        .epoch_core_reward_credits_for_distribution
    {
        0 => epoch_core_reward_credits_for_distribution_v0(
            from_start_block_core_height,
            to_end_block_core_height_included,
            core_subsidy_halving_interval,
        ),
        version => Err(ProtocolError::UnknownVersionMismatch {
            method: "epoch_core_reward_credits_for_distribution".to_string(),
            known_versions: vec![0],
            received: version,
        }),
    }
}
