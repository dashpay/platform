mod data_contract_based_queries;
mod document_query;
mod identity_based_queries;
mod proofs;
mod response_metadata;
mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::error::execution::ExecutionError;
use dpp::validation::ValidationResult;
use dpp::version::FeatureVersion;
use dpp::version::PlatformVersion;

/// A query validation result
pub type QueryValidationResult<TData> = ValidationResult<TData, QueryError>;

impl<C> Platform<C> {
    /// Querying
    pub fn query(
        &self,
        query_path: &str,
        query_data: &[u8],
        version: Option<FeatureVersion>,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        match platform_version.drive_abci.query.base_query_structure {
            0 => self.query_v0(query_path, query_data, version, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "Platform::query".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
