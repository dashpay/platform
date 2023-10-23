mod v0;

use crate::error::execution::ExecutionError;
use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dpp::version::FeatureVersion;
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    /// Querying of version upgrade state
    pub(in crate::query) fn query_epoch_infos(
        &self,
        state: &PlatformState,
        query_data: &[u8],
        version: Option<FeatureVersion>,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        let feature_version_bounds = &platform_version
            .drive_abci
            .query
            .system
            .version_upgrade_state;
        let version = version.unwrap_or(feature_version_bounds.default_current_version);
        if !feature_version_bounds.check_version(version) {
            return Ok(QueryValidationResult::new_with_error(
                QueryError::UnsupportedQueryVersion(
                    "epoch_infos".to_string(),
                    feature_version_bounds.min_version,
                    feature_version_bounds.max_version,
                    platform_version.protocol_version,
                    version,
                ),
            ));
        }
        match version {
            0 => self.epoch_infos_v0(state, query_data, platform_version),
            version => Err(ExecutionError::UnknownVersionMismatch {
                method: "Platform::query_epoch_infos".to_string(),
                known_versions: vec![0],
                received: version,
            }
            .into()),
        }
    }
}
