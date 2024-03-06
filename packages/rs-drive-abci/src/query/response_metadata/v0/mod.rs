use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};

impl<C> Platform<C> {
    pub(in crate::query) fn response_metadata_v0(&self) -> ResponseMetadata {
        // TODO: We should pass state here
        let state = self.state.load();

        ResponseMetadata {
            height: state.last_committed_height(),
            core_chain_locked_height: state.last_committed_core_height(),
            epoch: state.last_committed_block_epoch().index as u32,
            time_ms: state.last_committed_block_time_ms().unwrap_or_default(),
            chain_id: self.config.abci.chain_id.clone(),
            protocol_version: state.current_protocol_version_in_consensus(),
        }
    }

    pub(in crate::query) fn response_metadata_and_proof_v0(
        &self,
        proof: Vec<u8>,
    ) -> (ResponseMetadata, Proof) {
        let state = self.state.load();

        let metadata = ResponseMetadata {
            height: state.last_committed_height(),
            core_chain_locked_height: state.last_committed_core_height(),
            epoch: state.last_committed_block_epoch().index as u32,
            time_ms: state.last_committed_block_time_ms().unwrap_or_default(),
            chain_id: self.config.abci.chain_id.clone(),
            protocol_version: state.current_protocol_version_in_consensus(),
        };

        let proof = Proof {
            grovedb_proof: proof,
            quorum_hash: state.last_committed_quorum_hash().to_vec(),
            quorum_type: self.config.validator_set_quorum_type() as u32,
            block_id_hash: state.last_committed_block_id_hash().to_vec(),
            signature: state.last_committed_block_signature().to_vec(),
            round: state.last_committed_block_round(),
        };

        (metadata, proof)
    }
}
