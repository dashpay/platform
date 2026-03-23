use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dpp::dashcore::Network;

use dpp::version::PlatformVersion;
use drive::dpp::util::deserializer::ProtocolVersion;

impl<C> Platform<C> {
    /// checks for a network upgrade and resets activation window
    /// this should only be called on epoch change
    pub(super) fn check_for_desired_protocol_upgrade_v0(
        &self,
        active_hpmns: u32,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ProtocolVersion>, Error> {
        let upgrade_percentage_needed = if (self.config.network == Network::Mainnet
            && platform_version.protocol_version == 1)
            || (self.config.network == Network::Testnet && platform_version.protocol_version == 2)
        {
            // This is a solution for the emergency update to version 3
            // We clean this up immediately though as we transition to check_for_desired_protocol_upgrade_v1
            u64::min(
                51,
                platform_version
                    .drive_abci
                    .methods
                    .protocol_upgrade
                    .protocol_version_upgrade_percentage_needed,
            )
        } else {
            platform_version
                .drive_abci
                .methods
                .protocol_upgrade
                .protocol_version_upgrade_percentage_needed
        };

        let required_upgraded_hpmns = 1
            + (active_hpmns as u64)
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
            active_hpmns,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;

    #[test]
    fn test_no_upgrade_when_no_votes() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        // No votes have been cast, so no version should pass threshold
        let result = platform
            .check_for_desired_protocol_upgrade_v0(100, platform_version)
            .expect("expected no error");

        assert_eq!(result, None);
    }

    #[test]
    fn test_upgrade_when_sufficient_votes() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        // Simulate votes: put enough votes for the next version in the global cache
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            // With 100 active hpmns and 75% threshold, we need 76 votes (1 + 100*75/100 = 76)
            counter.global_cache.insert(next_version, 80);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v0(100, platform_version)
            .expect("expected no error");

        assert_eq!(result, Some(next_version));
    }

    #[test]
    fn test_no_upgrade_when_insufficient_votes() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        // Simulate votes below threshold
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            // With 100 active hpmns and 75% threshold, we need 76 votes
            // Only supply 50
            counter.global_cache.insert(next_version, 50);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v0(100, platform_version)
            .expect("expected no error");

        assert_eq!(result, None);
    }

    #[test]
    fn test_error_when_multiple_versions_pass_threshold() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;
        let another_version = platform_version.protocol_version + 2;

        // Simulate two versions both passing the threshold (an incoherence situation)
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, 80);
            counter.global_cache.insert(another_version, 80);
        }

        let result = platform.check_for_desired_protocol_upgrade_v0(100, platform_version);

        assert!(result.is_err());
        match result {
            Err(Error::Execution(ExecutionError::ProtocolUpgradeIncoherence(_))) => {}
            _ => panic!("expected ProtocolUpgradeIncoherence error"),
        }
    }

    #[test]
    fn test_upgrade_with_votes_exactly_at_threshold() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        // Calculate the exact threshold
        let pct = platform_version
            .drive_abci
            .methods
            .protocol_upgrade
            .protocol_version_upgrade_percentage_needed;
        let required = 1 + (100u64 * pct / 100);

        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, required);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v0(100, platform_version)
            .expect("expected no error");

        assert_eq!(result, Some(next_version));
    }

    #[test]
    fn test_no_upgrade_with_one_below_threshold() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        // Calculate the exact threshold and supply one less
        let pct = platform_version
            .drive_abci
            .methods
            .protocol_upgrade
            .protocol_version_upgrade_percentage_needed;
        let required = 1 + (100u64 * pct / 100);

        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, required - 1);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v0(100, platform_version)
            .expect("expected no error");

        assert_eq!(result, None);
    }

    #[test]
    fn test_upgrade_with_zero_active_hpmns() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        // With 0 active hpmns, required = 1 + 0 = 1
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, 1);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v0(0, platform_version)
            .expect("expected no error");

        assert_eq!(result, Some(next_version));
    }

    #[test]
    fn test_upgrade_with_single_active_hpmn() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        // With 1 active hpmn and 75% threshold: required = 1 + (1 * 75 / 100) = 1
        // Since 1*75/100 = 0 in integer division, required = 1
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, 1);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v0(1, platform_version)
            .expect("expected no error");

        assert_eq!(result, Some(next_version));
    }
}
