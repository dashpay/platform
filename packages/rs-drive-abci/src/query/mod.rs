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
