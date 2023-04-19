
use crate::abci::server::AbciApplication;
use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;

use crate::execution::test_quorum::TestQuorumInfo;
use crate::rpc::core::CoreRPCLike;
use dashcore::blockdata::transaction::special_transaction::asset_unlock::qualified_asset_unlock::AssetUnlockPayload;
use dashcore::blockdata::transaction::special_transaction::asset_unlock::request_info::AssetUnlockRequestInfo;
use dashcore::blockdata::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::AssetUnlockBaseTransactionInfo;
use dashcore::blockdata::transaction::special_transaction::TransactionPayload::AssetUnlockPayloadType;
use dashcore::bls_sig_utils::BLSSignature;
use dashcore::consensus::Decodable;
use dpp::block::block_info::BlockInfo;
use dpp::state_transition::StateTransition;
use dpp::util::deserializer::ProtocolVersion;
use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;
use tenderdash_abci::proto::abci::{
    CommitInfo, RequestExtendVote, RequestFinalizeBlock, RequestPrepareProposal,
    RequestVerifyVoteExtension, ResponsePrepareProposal,
};
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::types::{
    Block, BlockId, Data, EvidenceList, Header, PartSetHeader, VoteExtension, VoteExtensionType,
};
use tenderdash_abci::proto::version::Consensus;
use tenderdash_abci::{
    proto::{self, signatures::SignDigest},
    Application,
};

impl<'a, C: CoreRPCLike> AbciApplication<'a, C> {
    /// Execute a block with various state transitions
    /// Returns the withdrawal transactions that were signed in the block
    pub fn mimic_execute_block(
        &self,
        proposer_pro_tx_hash: [u8; 32],
        current_quorum: &TestQuorumInfo,
        next_quorum: &TestQuorumInfo,
        proposed_version: ProtocolVersion,
        _total_hpmns: u32,
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
            epoch: _,
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
            consensus_param_updates: _,
            core_chain_lock_update: _,
            validator_set_update: _,
        } = response_prepare_proposal;

        if !expect_validation_errors {
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

        for validator in current_quorum.validator_set.iter() {
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
            if !expect_validation_errors && response_validate_vote_extension.status != VerifyStatus::Accept as i32 {
                return Err(Error::Abci(AbciError::GenericWithCode(1)));
            }
        }

        //FixMe: This is not correct for the threshold vote extension (we need to sign and do
        // things differently

        let guarded_block_execution_context = self.platform.block_execution_context.read().unwrap();
        let block_execution_context =
            guarded_block_execution_context
                .as_ref()
                .ok_or(Error::Execution(ExecutionError::CorruptedCodeExecution(
                    "block execution context must be set in block begin handler",
                )))?;

        let extensions = block_execution_context
            .withdrawal_transactions
            .keys()
            .map(|tx_id| {
                VoteExtension {
                    r#type: VoteExtensionType::ThresholdRecover as i32,
                    extension: tx_id.to_vec(),
                    signature: vec![], //todo: signature
                }
            })
            .collect();

        //todo: tidy up and fix
        let withdrawals = block_execution_context
            .withdrawal_transactions.values().map(|transaction| {
                let AssetUnlockBaseTransactionInfo {
                    version,
                    lock_time,
                    output,
                    base_payload,
                } = AssetUnlockBaseTransactionInfo::consensus_decode(transaction.as_slice())
                    .expect("a");
                dashcore::Transaction {
                    version,
                    lock_time,
                    input: vec![],
                    output,
                    special_transaction_payload: Some(AssetUnlockPayloadType(AssetUnlockPayload {
                        base: base_payload,
                        request_info: AssetUnlockRequestInfo {
                            request_height: core_height,
                            quorum_hash: current_quorum.quorum_hash,
                        },
                        quorum_sig: BLSSignature::from([0; 96].as_slice()),
                    })),
                }
            })
            .collect();

        drop(guarded_block_execution_context);

        // We need to sign the block hash

        let block_hash = [0; 32]; //todo
        let chain_id = "strategy_tests".to_string();
        let quorum_type = self.platform.config.quorum_type();

        let block_id = BlockId {
            hash: block_hash.to_vec(),                       //todo
            part_set_header: Some(PartSetHeader::default()), // todo
            state_id: [0; 32].to_vec(),                      //todo
        };

        let mut commit_info = CommitInfo {
            round: 0,
            quorum_hash: current_quorum.quorum_hash.to_vec(),
            block_signature: Default::default(),
            threshold_vote_extensions: extensions,
        };

        let commit = proto::types::Commit {
            block_id: Some(block_id.clone()),
            height: height as i64,
            round: 0,
            quorum_hash: current_quorum.quorum_hash.to_vec(),
            threshold_block_signature: Default::default(),
            threshold_vote_extensions: Default::default(),
        };

        //if not in testing this will default to true
        if self.platform.config.testing_configs.block_signing {
            let digest = commit
                .sign_digest(
                    &chain_id,
                    quorum_type as u8,
                    &current_quorum.quorum_hash,
                    height as i64,
                    0,
                )
                .expect("expected to sign digest");

            let block_signature = current_quorum.private_key.sign(digest.as_slice());

            commit_info.block_signature = block_signature.to_bytes().to_vec();
        } else {
            commit_info.block_signature = [0u8; 96].to_vec();
        }

        let request_finalize_block = RequestFinalizeBlock {
            commit: Some(commit_info),
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
                    chain_id,
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
            block_id: Some(block_id),
        };

        self.finalize_block(request_finalize_block)
            .unwrap_or_else(|e| {
                panic!(
                    "should finalize block #{} at time #{} : {:?}",
                    block_info.height, block_info.time_ms, e
                )
            });

        Ok(withdrawals)
    }
}
