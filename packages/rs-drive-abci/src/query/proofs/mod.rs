use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use dapi_grpc::drive::v0::{GetProofsRequest, GetProofsResponse};
use dpp::version::PlatformVersion;

mod v0;

impl<C> Platform<C> {
    /// Querying of platform proofs
    pub fn query_proofs(
        &self,
        request: GetProofsRequest,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<QueryValidationResult<GetProofsResponse>, Error> {
        match platform_version.drive_abci.query.proofs_query {
            0 => self.query_proofs_v0(request, platform_state, platform_version),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "query_proofs".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
