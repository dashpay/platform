use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dapi_grpc::platform::v0::ResponseMetadata;

impl<C> Platform<C> {
    pub(in crate::query) fn response_metadata_v0(&self, state: &PlatformState) -> ResponseMetadata {
        ResponseMetadata {
            height: state.height(),
            core_chain_locked_height: state.core_height(),
            time_ms: state.last_block_time_ms().unwrap_or_default(),
            chain_id: self.config.abci.chain_id.clone(),
            protocol_version: state.current_protocol_version_in_consensus(),
        }
    }
}
