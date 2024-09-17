use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use dpp::version::PlatformVersion;
use drive::dpp::util::deserializer::ProtocolVersion;

impl<C> Platform<C> {
    /// checks for a network upgrade and resets activation window
    /// this should only be called on epoch change
    pub(super) fn check_for_desired_protocol_upgrade_v0(
        &self,
        total_hpmns: u32,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ProtocolVersion>, Error> {
        let upgrade_percentage_needed = platform_version
            .drive_abci
            .methods
            .protocol_upgrade
            .protocol_version_upgrade_percentage_needed;

        let required_upgraded_hpmns = 1
            + (total_hpmns as u64)
                .checked_mul(upgrade_percentage_needed)
                .and_then(|product| product.checked_div(100))
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "overflow for required block count",
                )))?;

        // if we are at an epoch change, check to see if over 75% of blocks of previous epoch
        // were on the future version
        let protocol_versions_counter = self.drive.cache.protocol_versions_counter.read();

        let mut versions_passing_threshold =
            protocol_versions_counter.versions_passing_threshold(required_upgraded_hpmns);

        if versions_passing_threshold.len() > 1 {
            return Err(Error::Execution(
                ExecutionError::ProtocolUpgradeIncoherence(
                    "only at most 1 version should be able to pass the threshold to upgrade",
                ),
            ));
        }

        tracing::debug!(
            total_hpmns,
            required_upgraded_hpmns,
            all_votes = ?protocol_versions_counter.global_cache,
            ?versions_passing_threshold,
            "Protocol version voting is finished. we require {} upgraded, {} versions passing the threshold: {:?}",
            required_upgraded_hpmns,
            versions_passing_threshold.len(),
            versions_passing_threshold
        );

        if !versions_passing_threshold.is_empty() {
            // same as equals 1
            let next_version = versions_passing_threshold.remove(0);
            Ok(Some(next_version))
        } else {
            Ok(None)
        }
    }
}
