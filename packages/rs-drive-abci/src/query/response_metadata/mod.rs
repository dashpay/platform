mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    pub(in crate::query) fn response_metadata(
        &self,
        platform_state: &PlatformState,
        platform_version: &PlatformVersion,
    ) -> Result<ResponseMetadata, Error> {
        match platform_version.drive_abci.query.response_metadata {
            0 => Ok(self.response_metadata_v0(platform_state)),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "response_metadata".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
