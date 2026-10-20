use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use dpp::version::PlatformVersion;
use drive::dpp::util::deserializer::ProtocolVersion;

impl<C> Platform<C> {
    /// checks for a network upgrade and resets activation window
    /// this should only be called on epoch change
    pub(super) fn check_for_desired_protocol_upgrade_v1(
        &self,
        active_hpmns: u32,
        platform_version: &PlatformVersion,
    ) -> Result<Option<ProtocolVersion>, Error> {
        let upgrade_percentage_needed = platform_version
            .drive_abci
            .methods
            .protocol_upgrade
            .protocol_version_upgrade_percentage_needed;

        let required_upgraded_hpmns = 1
            + (active_hpmns as u64)
                .checked_mul(upgrade_percentage_needed)
                .and_then(|product| product.checked_div(100))
                .ok_or(Error::Execution(ExecutionError::Overflow(
                    "overflow for required block count",
                )))?;

        // At an epoch change, find desired versions whose validator count
        // exceeds the effective upgrade percentage.
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
    fn test_v1_no_upgrade_when_no_votes() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let result = platform
            .check_for_desired_protocol_upgrade_v1(100, platform_version)
            .expect("expected no error");

        assert_eq!(result, None);
    }

    #[test]
    fn test_v1_upgrade_when_sufficient_votes() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, 80);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v1(100, platform_version)
            .expect("expected no error");

        assert_eq!(result, Some(next_version));
    }

    #[test]
    fn test_v1_no_upgrade_when_insufficient_votes() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, 50);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v1(100, platform_version)
            .expect("expected no error");

        assert_eq!(result, None);
    }

    #[test]
    fn test_v1_error_when_multiple_versions_pass_threshold() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;
        let another_version = platform_version.protocol_version + 2;

        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, 80);
            counter.global_cache.insert(another_version, 80);
        }

        let result = platform.check_for_desired_protocol_upgrade_v1(100, platform_version);

        assert!(result.is_err());
        match result {
            Err(Error::Execution(ExecutionError::ProtocolUpgradeIncoherence(_))) => {}
            _ => panic!("expected ProtocolUpgradeIncoherence error"),
        }
    }

    #[test]
    fn test_v1_upgrade_with_votes_exactly_at_threshold() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

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
            .check_for_desired_protocol_upgrade_v1(100, platform_version)
            .expect("expected no error");

        assert_eq!(result, Some(next_version));
    }

    #[test]
    fn test_v1_no_upgrade_with_one_below_threshold() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

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
            .check_for_desired_protocol_upgrade_v1(100, platform_version)
            .expect("expected no error");

        assert_eq!(result, None);
    }

    #[test]
    fn test_v1_block_cache_votes_count_towards_threshold() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        // versions_passing_threshold looks at both global and block caches
        // Put enough votes only in block_cache to pass threshold
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.set_block_cache_version_count(next_version, 80);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v1(100, platform_version)
            .expect("expected no error");

        assert_eq!(result, Some(next_version));
    }

    #[test]
    fn test_v1_upgrade_with_large_number_of_hpmns() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        // With 10000 active hpmns and 75% threshold:
        // required = 1 + (10000 * 75 / 100) = 7501
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, 7501);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v1(10000, platform_version)
            .expect("expected no error");

        assert_eq!(result, Some(next_version));
    }

    /// With no active hpmns AND no votes, `required_upgraded_hpmns` is 1
    /// (the `1 +` floor) and every version in the counter will pass the
    /// threshold only if it has >= 1 vote. With zero total votes, no
    /// version passes.
    #[test]
    fn test_v1_zero_hpmns_zero_votes_returns_none() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let result = platform
            .check_for_desired_protocol_upgrade_v1(0, platform_version)
            .expect("no error");

        assert_eq!(result, None);
    }

    /// Block cache and global cache are merged by `versions_passing_threshold`.
    /// When the SAME version has votes in BOTH caches, the block cache
    /// overrides the global cache's value in the final merged map (HashMap
    /// `extend` semantics). We document that behaviour here so a future
    /// switch to additive merging would be caught.
    #[test]
    fn test_v1_block_cache_overrides_global_cache_for_same_version() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let next_version = platform_version.protocol_version + 1;

        // Global cache says 1 vote (way below threshold of 76 for 100 hpmns).
        // Block cache says 80 (above threshold).
        // `extend` means block_cache's 80 wins, so the merged count >= threshold.
        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            counter.global_cache.insert(next_version, 1);
            counter.set_block_cache_version_count(next_version, 80);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v1(100, platform_version)
            .expect("no error");

        assert_eq!(
            result,
            Some(next_version),
            "block cache must override global cache when merging versions"
        );
    }

    /// When multiple versions have votes but NONE reach the threshold,
    /// `versions_passing_threshold` returns an empty vector and the
    /// function must return `Ok(None)`. This catches the branch where
    /// the counter has entries but none qualify.
    #[test]
    fn test_v1_multiple_versions_all_below_threshold_returns_none() {
        let platform_version = PlatformVersion::latest();
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_initial_state_structure();

        let pct = platform_version
            .drive_abci
            .methods
            .protocol_upgrade
            .protocol_version_upgrade_percentage_needed;
        let required = 1 + (100u64 * pct / 100);

        {
            let mut counter = platform.drive.cache.protocol_versions_counter.write();
            // Seed several versions each strictly below `required`.
            counter
                .global_cache
                .insert(platform_version.protocol_version + 1, required / 4);
            counter
                .global_cache
                .insert(platform_version.protocol_version + 2, required / 2);
            counter
                .global_cache
                .insert(platform_version.protocol_version + 3, required - 1);
        }

        let result = platform
            .check_for_desired_protocol_upgrade_v1(100, platform_version)
            .expect("no error");

        assert_eq!(result, None);
    }
}
