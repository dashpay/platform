//! Platform's minimum asset-lock value for an identity top-up, owned by the
//! asset-lock builder, which holds every top-up lock it builds to it.

use dpp::balances::credits::CREDITS_PER_DUFF;
use dpp::state_transition::identity_topup_transition::v0::IdentityTopUpTransitionV0;
use dpp::state_transition::identity_topup_transition::IdentityTopUpTransition;
use dpp::state_transition::StateTransitionEstimatedFeeValidation;
use dpp::version::{PlatformVersion, PLATFORM_VERSIONS};

use crate::error::PlatformWalletError;

/// The minimum asset-lock funding, in duffs, Platform accepts for an
/// identity top-up under every protocol version this build supports.
///
/// Platform rejects an `IdentityTopUp` whose asset-lock output value (in
/// credits) is below `IdentityTopUpTransition::calculate_min_required_fee`,
/// so that calculation is the minimum: 50_000 duffs under fee v0, and 50_500
/// under v1 (protocol v11+), which adds `identity_topup_base_cost`. Below it,
/// a lock built and broadcast is accepted by Core (spending real UTXOs) but
/// rejected by Platform, stranding the funds in a lock that can never
/// complete the top-up.
///
/// The largest minimum across the versions, not the SDK's current one: the
/// SDK's version can move during the top-up (a proved fetch ratchets it up,
/// and a pinned SDK can lag the network), so an amount held to the version
/// read first could clear a lower floor and then be submitted under a higher
/// one, after its Core funds are locked.
pub fn min_top_up_duffs() -> Result<u64, PlatformWalletError> {
    PLATFORM_VERSIONS
        .iter()
        .try_fold(0, |max, platform_version| {
            Ok(max.max(min_top_up_duffs_at(platform_version)?))
        })
}

/// Platform's top-up minimum, in duffs, under `platform_version` alone.
fn min_top_up_duffs_at(platform_version: &PlatformVersion) -> Result<u64, PlatformWalletError> {
    let credits = IdentityTopUpTransition::V0(IdentityTopUpTransitionV0::default())
        .calculate_min_required_fee(platform_version)
        .map_err(dash_sdk::Error::Protocol)?;
    Ok(credits.div_ceil(CREDITS_PER_DUFF))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The minimum is Platform's own top-up fee floor, the largest across the
    /// protocol versions this build supports, so it holds whichever of them
    /// the top-up is submitted under: the bare asset-lock floor under fee v0,
    /// plus the top-up base cost under v1.
    #[test]
    fn should_take_the_top_up_minimum_from_every_protocol_version() {
        assert_eq!(
            min_top_up_duffs_at(PlatformVersion::first()).unwrap(),
            50_000
        );
        assert_eq!(
            min_top_up_duffs_at(PlatformVersion::latest()).unwrap(),
            50_500
        );
        let min = min_top_up_duffs().unwrap();
        assert_eq!(min, 50_500);
        for platform_version in PLATFORM_VERSIONS {
            assert!(
                min >= min_top_up_duffs_at(platform_version).unwrap(),
                "protocol version {}",
                platform_version.protocol_version
            );
        }
    }
}
