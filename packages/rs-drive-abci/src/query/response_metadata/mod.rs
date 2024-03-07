mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;

use dapi_grpc::platform::v0::{ResponseMetadata};
use dpp::version::PlatformVersion;

impl<C> Platform<C> {
    #[allow(dead_code)] #[deprecated(note = "This function is marked as unused.")] #[allow(deprecated)]
    pub(in crate::query) fn response_metadata(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<ResponseMetadata, Error> {
        match platform_version.drive_abci.query.response_metadata {
            0 => Ok(self.response_metadata_v0()),
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "response_metadata".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
