use crate::abci::AbciError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::PlatformState;
use crate::platform_types::platform_state::PlatformStateV0Methods;
use crate::query::response_metadata::CheckpointUsed;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use drive::error::drive::DriveError;
use drive::util::grove_operations::GroveDBToUse;

impl<C> Platform<C> {
    /// Refuses to build a proof from a state that has no block proof metadata.
    ///
    /// A state restored via state sync has an all-zero block id hash and quorum signature
    /// at the snapshot height: the reduced platform state is written into grovedb BEFORE
    /// the block's root hash exists, and the block's commit signature signs that root
    /// hash, so the signature can never be part of the state it signs.
    /// `rs-drive-proof-verifier` (correctly) rejects an all-zero signature, meaning a
    /// proof built from such a state could never authenticate — refuse it with a
    /// retryable error instead. The first block finalized after the restore stores real
    /// metadata and reopens proof serving.
    ///
    /// Height 0 is exempt: a chain that has not committed a block yet has no signature
    /// either, which predates state sync and is left as is.
    fn ensure_block_proof_metadata_is_available(&self, state: &PlatformState) -> Result<(), Error> {
        // A test chain running with block signing disabled finalizes every block with an
        // all-zero signature; its proofs were never verifiable, and gating them would
        // break the strategy test harness's proof plumbing checks.
        #[cfg(feature = "testing-config")]
        if !self.config.testing_configs.block_signing {
            return Ok(());
        }

        if state.last_committed_block_height() > 0
            && state.last_committed_block_signature() == [0u8; 96]
        {
            return Err(AbciError::StateSyncProofMetadataUnavailable(format!(
                "the state at height {} was restored via state sync and its block signature \
                 only becomes known when the next block is finalized; retry shortly, or \
                 repeat the query without requesting a proof",
                state.last_committed_block_height()
            ))
            .into());
        }
        Ok(())
    }
}

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
                self.ensure_block_proof_metadata_is_available(platform_state)?;
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

                self.ensure_block_proof_metadata_is_available(&checkpoint_state)?;
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

                self.ensure_block_proof_metadata_is_available(&checkpoint_state)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::block::extended_block_info::v0::ExtendedBlockInfoV0;
    use dpp::block::extended_block_info::ExtendedBlockInfo;

    fn block_info_with_signature(height: u64, signature: [u8; 96]) -> ExtendedBlockInfo {
        ExtendedBlockInfo::V0(ExtendedBlockInfoV0 {
            basic_info: BlockInfo {
                time_ms: 1_000_000,
                height,
                core_height: 42,
                epoch: Default::default(),
            },
            app_hash: [1u8; 32],
            quorum_hash: [2u8; 32],
            block_id_hash: [3u8; 32],
            proposer_pro_tx_hash: [4u8; 32],
            signature,
            round: 0,
        })
    }

    /// A state restored via state sync stores an all-zero block signature until the next
    /// block finalizes; a proof built from it can never authenticate (the verifier
    /// rejects an all-zero signature), so it must be refused rather than served.
    #[test]
    fn should_refuse_a_proof_from_a_state_without_block_proof_metadata() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut state = platform.state.load().as_ref().clone();
        state.set_last_committed_block_info(Some(block_info_with_signature(10, [0u8; 96])));

        let result = platform.response_proof_v0(&state, vec![], GroveDBToUse::Current);
        let error = result.expect_err("a zero-signature state must not produce a proof");
        assert!(
            matches!(
                error,
                Error::Abci(AbciError::StateSyncProofMetadataUnavailable(_))
            ),
            "expected StateSyncProofMetadataUnavailable, got: {error}"
        );
    }

    /// A normally finalized block always carries a real signature; proofs must be served.
    #[test]
    fn should_serve_a_proof_from_a_state_with_block_proof_metadata() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let mut state = platform.state.load().as_ref().clone();
        state.set_last_committed_block_info(Some(block_info_with_signature(10, [5u8; 96])));

        let (_, proof) = platform
            .response_proof_v0(&state, vec![], GroveDBToUse::Current)
            .expect("a signed state must produce a proof");
        assert_eq!(proof.signature, vec![5u8; 96]);
        assert_eq!(proof.block_id_hash, vec![3u8; 32]);
    }

    /// A chain that has not committed a block yet has no signature for anyone — that
    /// predates state sync and stays as it was.
    #[test]
    fn should_leave_the_pre_genesis_state_exempt() {
        let platform = TestPlatformBuilder::new()
            .build_with_mock_rpc()
            .set_genesis_state();

        let state = platform.state.load();
        assert_eq!(state.last_committed_block_height(), 0, "sanity: no blocks");

        platform
            .response_proof_v0(&state, vec![], GroveDBToUse::Current)
            .expect("the pre-genesis state must remain servable");
    }
}
