use crate::abci::server::AbciApplication;
use crate::abci::AbciError;
use crate::error::Error;
use crate::execution::quorum::Quorum;
use crate::rpc::core::CoreRPCLike;
use dashcore::hashes::Hash;
use dashcore_rpc::json::{MasternodeListItem, QuorumMasternodeListItem};
use dpp::state_transition::StateTransition;
use dpp::util::deserializer::ProtocolVersion;
use drive::drive::block_info::BlockInfo;
use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;
use tenderdash_abci::proto::abci::{
    CommitInfo, RequestExtendVote, RequestFinalizeBlock, RequestPrepareProposal,
    RequestVerifyVoteExtension, ResponsePrepareProposal,
};
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::types::{
    Block, BlockId, Data, Evidence, EvidenceList, Header, PartSetHeader,
};
use tenderdash_abci::proto::version::Consensus;
use tenderdash_abci::Application;

impl<'a, C: CoreRPCLike> AbciApplication<'a, C> {
    /// Execute a block with various state transitions
    /// Returns the withdrawal transactions that were signed in the block
    pub fn mimic_execute_block(
        &self,
        proposer_pro_tx_hash: [u8; 32],
        current_quorum: &Quorum,
        next_quorum: &Quorum,
        proposed_version: ProtocolVersion,
        total_hpmns: u32,
        block_info: BlockInfo,
        expect_validation_errors: bool,
        state_transitions: Vec<StateTransition>,
    ) -> Result<Vec<dashcore::Transaction>, Error> {
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
            core_chain_locked_height: core_height,
            proposer_pro_tx_hash: proposer_pro_tx_hash.to_vec(),
            proposed_app_version: proposed_version as u64,
            version: Some(Consensus { block: 0, app: 0 }),
            quorum_hash: current_quorum.quorum_hash.to_vec(),
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

        let tx_order_for_finalize_block = tx_records.into_iter().map(|record| record.tx).collect();

        let request_extend_vote = RequestExtendVote {
            hash: [0; 32].to_vec(), //todo
            height: height as i64,
            round: 0,
        };

        let response_extend_vote = self.extend_vote(request_extend_vote).unwrap_or_else(|e| {
            panic!(
                "should extend vote #{} at time #{} : {:?}",
                block_info.height, block_info.time_ms, e
            )
        });

        let vote_extensions = response_extend_vote.vote_extensions;

        // for all proposers in the quorum we much verify each vote extension

        for validator in current_quorum.validator_set.values() {
            let request_verify_vote_extension = RequestVerifyVoteExtension {
                hash: [0; 32].to_vec(), //todo
                validator_pro_tx_hash: validator.pro_tx_hash.to_vec(),
                height: height as i64,
                round: 0,
                vote_extensions: vote_extensions.clone(),
            };
            let response_validate_vote_extension = self
                .verify_vote_extension(request_verify_vote_extension)
                .unwrap_or_else(|e| {
                    panic!(
                        "should verify vote extension #{} at time #{} : {:?}",
                        block_info.height, block_info.time_ms, e
                    )
                });
            if expect_validation_errors == false {
                if response_validate_vote_extension.status != VerifyStatus::Accept as i32 {
                    return Err(Error::Abci(AbciError::GenericWithCode(1)));
                }
            }
        }

        let request_finalize_block = RequestFinalizeBlock {
            commit: Some(CommitInfo {
                round: 0,
                quorum_hash: current_quorum.quorum_hash.to_vec(),
                block_signature: [0; 96].to_vec(),
                threshold_vote_extensions: vec![],
            }),
            misbehavior: vec![],
            hash: app_hash.clone(), //todo: change this to block hash
            height: height as i64,
            round: 0,
            block: Some(Block {
                header: Some(Header {
                    version: Some(Consensus {
                        block: 0, //todo
                        app: 0,   //todo
                    }),
                    chain_id: "strategy_tests".to_string(),
                    height: height as i64,
                    time: Some(Timestamp {
                        seconds: (time_ms / 1000) as i64,
                        nanos: ((time_ms % 1000) * 1000) as i32,
                    }),
                    last_block_id: None,
                    last_commit_hash: [0; 32].to_vec(),
                    data_hash: [0; 32].to_vec(),
                    validators_hash: current_quorum.quorum_hash.to_vec(),
                    next_validators_hash: next_quorum.quorum_hash.to_vec(),
                    consensus_hash: [0; 32].to_vec(),
                    next_consensus_hash: [0; 32].to_vec(),
                    app_hash,
                    results_hash: [0; 32].to_vec(),
                    evidence_hash: vec![],
                    proposed_app_version: 0,
                    proposer_pro_tx_hash: proposer_pro_tx_hash.to_vec(),
                    core_chain_locked_height: core_height,
                }),
                data: Some(Data {
                    txs: tx_order_for_finalize_block,
                }),
                evidence: Some(EvidenceList { evidence: vec![] }),
                last_commit: None,
                core_chain_lock: None,
            }),
            block_id: Some(BlockId {
                hash: [0; 32].to_vec(),                          //todo
                part_set_header: Some(PartSetHeader::default()), // todo
                state_id: [0; 32].to_vec(),                      //todo
            }),
        };

        self.finalize_block(request_finalize_block)
            .unwrap_or_else(|e| {
                panic!(
                    "should finalize block #{} at time #{} : {:?}",
                    block_info.height, block_info.time_ms, e
                )
            });

        Ok(vec![])
    }
}
