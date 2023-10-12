mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use dpp::validation::ValidationResult;
use dpp::version::PlatformVersion;

/// A query validation result
pub type QueryValidationResult<TData> = ValidationResult<TData, QueryError>;

impl<C> Platform<C> {
    /// Querying
    pub fn query(
        &self,
        query_path: &str,
        query_data: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        //todo: choose based on protocol version
        self.query_v0(query_path, query_data, platform_version)
    }
}

#[cfg(test)]
mod tests {
    mod query {
        use crate::config::PlatformConfig;
        use crate::error::query::QueryError;
        use crate::rpc::core::MockCoreRPCLike;
        use crate::test::helpers::setup::{TempPlatform, TestPlatformBuilder};
        use platform_version::version::PlatformVersion;

        fn setup_platform<'a>() -> (TempPlatform<MockCoreRPCLike>, &'a PlatformVersion) {
            (
                TestPlatformBuilder::new()
                    .with_config(PlatformConfig {
                        ..Default::default()
                    })
                    .build_with_mock_rpc(),
                PlatformVersion::latest(),
            )
        }
        #[test]
        /// Tests for query handler
        fn test_invalid_path() {
            let (platform, platform_version) = setup_platform();

            let data = vec![0; 32];
            let result = platform.query("/invalid_path", &data, &platform_version);
            assert!(result.is_ok());

            let validation_result = result.unwrap();
            assert!(matches!(
                validation_result.first_error().unwrap(),
                QueryError::InvalidArgument(msg) if msg == "query path '/invalid_path' is not supported"
            ));
        }
    }
}
