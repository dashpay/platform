use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};

impl<C> Platform<C> {
    pub(in crate::query) fn response_metadata_v0(
        &self,
        platform_state: &PlatformState,
    ) -> ResponseMetadata {
        ResponseMetadata {
            height: platform_state.last_committed_block_height(),
            core_chain_locked_height: platform_state.last_committed_core_height(),
            epoch: platform_state.last_committed_block_epoch().index as u32,
            time_ms: platform_state
                .last_committed_block_time_ms()
                .unwrap_or_default(),
            chain_id: self.config.abci.chain_id.clone(),
            protocol_version: platform_state.current_protocol_version_in_consensus(),
        }
    }

    pub(in crate::query) fn response_proof_v0(
        &self,
        platform_state: &PlatformState,
        proof: Vec<u8>,
    ) -> Proof {
        Proof {
            grovedb_proof: proof,
            quorum_hash: platform_state.last_committed_quorum_hash().to_vec(),
            quorum_type: self.config.validator_set_quorum_type() as u32,
            block_id_hash: platform_state.last_committed_block_id_hash().to_vec(),
            signature: platform_state.last_committed_block_signature().to_vec(),
            round: platform_state.last_committed_block_round(),
        }
    }
}
