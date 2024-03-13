use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::version::PlatformVersion;

use drive::dpp::util::deserializer::ProtocolVersion;
use drive::grovedb::Transaction;

/// The percentage needed of HPMNs to upgrade the protocol
/// It always needs to be higher than the rounded amount after applying the percentage
const PROTOCOL_VERSION_UPGRADE_PERCENTAGE_NEEDED: u64 = 75;

impl<C> Platform<C> {
    /// checks for a network upgrade and resets activation window
    /// this should only be called on epoch change
    /// this will change backing state and drive cache
    pub(super) fn check_for_desired_protocol_upgrade_and_reset_v0(
        &self,
        total_hpmns: u32,
        transaction: &Transaction,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ProtocolVersion>, Error> {
        let required_upgraded_hpns = 1
            + (total_hpmns as u64)
                .checked_mul(PROTOCOL_VERSION_UPGRADE_PERCENTAGE_NEEDED)
                .and_then(|product| product.checked_div(100))
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "overflow for required block count",
                )))?;

        // if we are at an epoch change, check to see if over 75% of blocks of previous epoch
        // were on the future version
        // This also clears the cache
        let mut protocol_versions_counter = self.drive.cache.protocol_versions_counter.write();
        let mut versions_passing_threshold = protocol_versions_counter
            .aggregate_into_versions_passing_threshold(required_upgraded_hpns);
        drop(protocol_versions_counter);

        if versions_passing_threshold.len() > 1 {
            return Err(Error::Execution(
                ExecutionError::ProtocolUpgradeIncoherence(
                    "only at most 1 version should be able to pass the threshold to upgrade",
                ),
            ));
        }

        // we need to drop all version information
        self.drive
            .clear_version_information(Some(transaction), &platform_version.drive)
            .map_err(Error::Drive)?;

        if !versions_passing_threshold.is_empty() {
            // same as equals 1
            let next_version = versions_passing_threshold.remove(0);

            // TODO: We stored next version here previously.
            //  It was never used so we can temporary remove it from here and move it to Epoch trees in upcoming PR

            Ok(Some(next_version))
        } else {
            Ok(None)
        }
    }
}
