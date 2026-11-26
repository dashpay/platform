mod v0;

use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::version::PlatformVersion;
use drive::util::grove_operations::GroveDBToUse;
use std::sync::Arc;

/// Specifies which GroveDB instance was used for a query, along with the associated platform state
#[derive(Debug, Clone)]
pub enum CheckpointUsed {
    /// The current (main) GroveDB was used
    Current,
    /// A checkpoint was used, with the platform state at that checkpoint
    Checkpoint(Arc<PlatformState>),
}

impl<C> Platform<C> {
    /// `grovedb_proof` signed off with the last committed block of `platform_state`, and that
    /// block's metadata, made by the metadata generation `platform_version` selects. The caller
    /// passes the version of that block, which across an upgrade is not the version of the
    /// block after it.
    pub(crate) fn committed_block_proof_and_metadata(
        &self,
        platform_state: &PlatformState,
        grovedb_proof: Vec<u8>,
        platform_version: &PlatformVersion,
    ) -> Result<(Proof, ResponseMetadata), Error> {
        match platform_version.drive_abci.query.response_metadata {
            0 => {
                let (checkpoint_used, proof) =
                    self.response_proof_v0(platform_state, grovedb_proof, GroveDBToUse::Current)?;
                Ok((
                    proof,
                    self.response_metadata_v0(platform_state, checkpoint_used),
                ))
            }
            version => Err(Error::Execution(ExecutionError::UnknownVersionMismatch {
                method: "committed_block_proof_and_metadata".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::version::PlatformVersion;

    #[test]
    fn should_refuse_a_metadata_generation_this_build_does_not_have() {
        let platform = TestPlatformBuilder::new()
            .with_latest_protocol_version()
            .build_with_mock_rpc();
        let platform_state = platform.state.load();
        let mut unknown = PlatformVersion::latest().clone();
        unknown.drive_abci.query.response_metadata = 1;

        assert!(platform
            .committed_block_proof_and_metadata(&platform_state, vec![], PlatformVersion::latest())
            .is_ok());
        assert!(platform
            .committed_block_proof_and_metadata(&platform_state, vec![], &unknown)
            .is_err());
    }
}
