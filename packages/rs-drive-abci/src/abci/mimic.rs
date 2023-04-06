use crate::abci::server::AbciApplication;
use crate::abci::AbciError;
use crate::error::Error;
use crate::rpc::core::CoreRPCLike;
use dpp::state_transition::StateTransition;
use dpp::util::deserializer::ProtocolVersion;
use drive::drive::block_info::BlockInfo;
use tenderdash_abci::proto::abci::{
    CommitInfo, RequestFinalizeBlock, RequestPrepareProposal, ResponsePrepareProposal,
};
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::Application;

impl<'a, C: CoreRPCLike> AbciApplication<'a, C> {
    /// Execute a block with various state transitions
    pub fn mimic_execute_block(
        &self,
        proposer_pro_tx_hash: [u8; 32],
        quorum_hash: [u8; 32],
        proposed_version: ProtocolVersion,
        total_hpmns: u32,
        block_info: BlockInfo,
        expect_validation_errors: bool,
        state_transitions: Vec<StateTransition>,
    ) -> Result<(), Error> {
        let serialized_state_transitions = state_transitions
            .into_iter()
            .map(|st| st.serialize().map_err(Error::Protocol))
            .collect::<Result<Vec<Vec<u8>>, Error>>()?;

        let BlockInfo {
            time_ms,
            height,
            core_height,
            epoch,
        } = block_info;

        let request_prepare_proposal = RequestPrepareProposal {
            max_tx_bytes: 0,
            txs: serialized_state_transitions,
            local_last_commit: None,
            misbehavior: vec![],
            height: height as i64,
            time: Some(Timestamp {
                seconds: (time_ms / 1000) as i64,
                nanos: ((time_ms % 1000) * 1000) as i32,
            }),
            next_validators_hash: vec![],
            round: 0,
            core_chain_locked_height: 0,
            proposer_pro_tx_hash: proposer_pro_tx_hash.to_vec(),
            proposed_app_version: proposed_version as u64,
            version: None,
            quorum_hash: quorum_hash.to_vec(),
        };

        let response_prepare_proposal = self
            .prepare_proposal(request_prepare_proposal)
            .unwrap_or_else(|e| {
                panic!(
                    "should prepare and process block #{} at time #{} : {:?}",
                    block_info.height, block_info.time_ms, e
                )
            });
        let ResponsePrepareProposal {
            tx_records,
            app_hash,
            tx_results,
            consensus_param_updates,
            core_chain_lock_update,
            validator_set_update,
        } = response_prepare_proposal;

        if expect_validation_errors == false {
            if tx_results.len() != tx_records.len() {
                return Err(Error::Abci(AbciError::GenericWithCode(0)));
            }
            tx_results.into_iter().try_for_each(|tx_result| {
                if tx_result.code > 0 {
                    Err(Error::Abci(AbciError::GenericWithCode(tx_result.code)))
                } else {
                    Ok(())
                }
            })?;
        }

        let request_finalize_block = RequestFinalizeBlock {
            commit: Some(CommitInfo {
                round: 0,
                quorum_hash: quorum_hash.to_vec(),
                block_signature: vec![],
                threshold_vote_extensions: vec![],
            }),
            misbehavior: vec![],
            hash: app_hash,
            height: height as i64,
            round: 0,
            block: None,
            block_id: None,
        };

        self.finalize_block(request_finalize_block)
            .unwrap_or_else(|e| {
                panic!(
                    "should finalize block #{} at time #{} : {:?}",
                    block_info.height, block_info.time_ms, e
                )
            });

        Ok(())
    }
}
