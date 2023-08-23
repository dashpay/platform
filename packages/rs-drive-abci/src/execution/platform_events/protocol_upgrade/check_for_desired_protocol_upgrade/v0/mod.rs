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
    /// this will change backing state, but does not change drive cache
    pub(super) fn check_for_desired_protocol_upgrade_v0(
        &self,
        total_hpmns: u32,
        current_protocol_version_in_consensus: ProtocolVersion,
        transaction: &Transaction,
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
        let mut cache = self.drive.cache.write().unwrap();
        let mut versions_passing_threshold = cache
            .protocol_versions_counter
            .take()
            .map(|version_counter| {
                version_counter
                    .into_iter()
                    .filter_map(|(protocol_version, count)| {
                        if count >= required_upgraded_hpns {
                            Some(protocol_version)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<ProtocolVersion>>()
            })
            .unwrap_or_default();

        if versions_passing_threshold.len() > 1 {
            return Err(Error::Execution(
                ExecutionError::ProtocolUpgradeIncoherence(
                    "only at most 1 version should be able to pass the threshold to upgrade",
                ),
            ));
        }

        if !versions_passing_threshold.is_empty() {
            // same as equals 1
            let new_version = versions_passing_threshold.remove(0);
            // Persist current and next epoch protocol versions
            // we also drop all protocol version votes information
            self.drive
                .change_to_new_version_and_clear_version_information(
                    current_protocol_version_in_consensus,
                    new_version,
                    Some(transaction),
                )
                .map_err(Error::Drive)?;

            Ok(Some(new_version))
        } else {
            // we need to drop all version information
            let current_platform_version =
                PlatformVersion::get(current_protocol_version_in_consensus)?;
            self.drive
                .clear_version_information(Some(transaction), &current_platform_version.drive)
                .map_err(Error::Drive)?;

            Ok(None)
        }
    }
}
