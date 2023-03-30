use crate::constants::PROTOCOL_VERSION_UPGRADE_PERCENTAGE_NEEDED;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::Platform;
use drive::dpp::util::deserializer::ProtocolVersion;
use drive::grovedb::TransactionArg;

impl<CoreRPCLike> Platform<CoreRPCLike> {
    /// checks for a network upgrade and resets activation window
    /// this should only be called on epoch change
    /// this will change backing state, but does not change drive cache
    pub fn check_for_desired_protocol_upgrade(
        &self,
        total_hpmns: u32,
        transaction: TransactionArg,
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

        if versions_passing_threshold.len() == 1 {
            let new_version = versions_passing_threshold.remove(0);
            // Persist current and next epoch protocol versions
            // we also drop all protocol version votes information
            self.drive
                .change_to_new_version_and_clear_version_information(
                    self.state.read().unwrap().current_protocol_version_in_consensus,
                    new_version,
                    transaction,
                )
                .map_err(Error::Drive)?;

            Ok(Some(new_version))
        } else {
            // we need to drop all version information
            self.drive
                .clear_version_information(transaction)
                .map_err(Error::Drive)?;

            Ok(None)
        }
    }
}
