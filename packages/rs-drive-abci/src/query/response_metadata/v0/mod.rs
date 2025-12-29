use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::query::response_metadata::CheckpointUsed;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use drive::error::drive::DriveError;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Returns response metadata for the given GroveDB that was used.
    ///
    /// This function should be called with the `GroveDBUsed` returned from `response_proof_v0`
    /// to ensure consistency between the proof and metadata.
    pub(in crate::query) fn response_metadata_v0(
        &self,
        platform_state: &PlatformState,
        grovedb_used: CheckpointUsed,
    ) -> ResponseMetadata {
        let state: &PlatformState = match &grovedb_used {
            CheckpointUsed::Current => platform_state,
            CheckpointUsed::Checkpoint(checkpoint_state) => checkpoint_state.as_ref(),
        };

        ResponseMetadata {
            height: state.last_committed_block_height(),
            core_chain_locked_height: state.last_committed_core_height(),
            epoch: state.last_committed_block_epoch().index as u32,
            time_ms: state.last_committed_block_time_ms().unwrap_or_default(),
            chain_id: self.config.abci.chain_id.clone(),
            protocol_version: state.current_protocol_version_in_consensus(),
        }
    }

    /// Returns response proof for the requested GroveDB along with which GroveDB was actually used.
    ///
    /// Returns a tuple of (GroveDBUsed, Proof) so the caller can pass the same GroveDBUsed
    /// to `response_metadata_v0` for consistency.
    ///
    /// Returns an error if a checkpoint was requested but not found.
    pub(in crate::query) fn response_proof_v0(
        &self,
        platform_state: &PlatformState,
        proof: Vec<u8>,
        grovedb_to_use: GroveDBToUse,
    ) -> Result<(CheckpointUsed, Proof), Error> {
        match grovedb_to_use {
            GroveDBToUse::Current => {
                let proof = Proof {
                    grovedb_proof: proof,
                    quorum_hash: platform_state.last_committed_quorum_hash().to_vec(),
                    quorum_type: self.config.validator_set.quorum_type as u32,
                    block_id_hash: platform_state.last_committed_block_id_hash().to_vec(),
                    signature: platform_state.last_committed_block_signature().to_vec(),
                    round: platform_state.last_committed_block_round(),
                };
                Ok((CheckpointUsed::Current, proof))
            }
            GroveDBToUse::LatestCheckpoint => {
                let checkpoints = self.drive.checkpoints.load();
                let (&height, _) = checkpoints.last_key_value().ok_or_else(|| {
                    Error::Drive(drive::error::Error::Drive(
                        DriveError::NoCheckpointsAvailable,
                    ))
                })?;

                let checkpoint_states = self.checkpoint_platform_states.load();
                let checkpoint_state = checkpoint_states
                    .get(&height)
                    .ok_or_else(|| {
                        Error::Drive(drive::error::Error::Drive(DriveError::CheckpointNotFound(
                            height,
                        )))
                    })?
                    .clone();

                let proof = Proof {
                    grovedb_proof: proof,
                    quorum_hash: checkpoint_state.last_committed_quorum_hash().to_vec(),
                    quorum_type: self.config.validator_set.quorum_type as u32,
                    block_id_hash: checkpoint_state.last_committed_block_id_hash().to_vec(),
                    signature: checkpoint_state.last_committed_block_signature().to_vec(),
                    round: checkpoint_state.last_committed_block_round(),
                };
                Ok((CheckpointUsed::Checkpoint(checkpoint_state), proof))
            }
            GroveDBToUse::Checkpoint(block_height) => {
                let checkpoint_states = self.checkpoint_platform_states.load();
                let checkpoint_state = checkpoint_states
                    .get(&block_height)
                    .ok_or_else(|| {
                        Error::Drive(drive::error::Error::Drive(DriveError::CheckpointNotFound(
                            block_height,
                        )))
                    })?
                    .clone();

                let proof = Proof {
                    grovedb_proof: proof,
                    quorum_hash: checkpoint_state.last_committed_quorum_hash().to_vec(),
                    quorum_type: self.config.validator_set.quorum_type as u32,
                    block_id_hash: checkpoint_state.last_committed_block_id_hash().to_vec(),
                    signature: checkpoint_state.last_committed_block_signature().to_vec(),
                    round: checkpoint_state.last_committed_block_round(),
                };
                Ok((CheckpointUsed::Checkpoint(checkpoint_state), proof))
            }
        }
    }
}
