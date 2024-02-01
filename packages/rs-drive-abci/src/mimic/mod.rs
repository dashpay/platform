use crate::abci::server::AbciApplication;
use crate::abci::AbciError;
use crate::error::execution::ExecutionError;
use crate::error::Error;
use bytes::Buf;

use crate::rpc::core::CoreRPCLike;
use dashcore_rpc::dashcore::blockdata::transaction::special_transaction::asset_unlock::qualified_asset_unlock::AssetUnlockPayload;
use dashcore_rpc::dashcore::blockdata::transaction::special_transaction::asset_unlock::request_info::AssetUnlockRequestInfo;
use dashcore_rpc::dashcore::blockdata::transaction::special_transaction::asset_unlock::unqualified_asset_unlock::AssetUnlockBaseTransactionInfo;
use dashcore_rpc::dashcore::blockdata::transaction::special_transaction::TransactionPayload::AssetUnlockPayloadType;
use dashcore_rpc::dashcore::bls_sig_utils::BLSSignature;
use dashcore_rpc::dashcore::consensus::Decodable;
use dashcore_rpc::dashcore;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use dpp::dashcore::hashes::Hash;
use dpp::block::block_info::BlockInfo;
use dpp::serialization::PlatformSerializable;
use dpp::state_transition::StateTransition;
use dpp::util::deserializer::ProtocolVersion;
use tenderdash_abci::proto::abci::response_verify_vote_extension::VerifyStatus;
use tenderdash_abci::proto::abci::{CommitInfo, ExecTxResult, RequestExtendVote, RequestFinalizeBlock, RequestPrepareProposal, RequestProcessProposal, RequestVerifyVoteExtension, ResponsePrepareProposal, ValidatorSetUpdate};
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::serializers::timestamp::ToMilis;
use tenderdash_abci::proto::types::{
    Block, BlockId, Data, EvidenceList, Header, PartSetHeader, VoteExtension, VoteExtensionType, StateId, CanonicalVote, SignedMsgType,
};
use tenderdash_abci::signatures::Hashable;
use tenderdash_abci::{signatures::Signable, proto::version::Consensus, Application};
use tenderdash_abci::proto::abci::tx_record::TxAction;
use crate::execution::types::block_execution_context::v0::BlockExecutionContextV0Getters;
use crate::execution::types::block_state_info::v0::BlockStateInfoV0Getters;
use crate::mimic::test_quorum::TestQuorumInfo;

/// Test quorum for mimic block execution
pub mod test_quorum;

/// Chain ID used in tests
pub const CHAIN_ID: &str = "strategy_tests";

/// The outcome struct when mimicking block execution
pub struct MimicExecuteBlockOutcome {
    /// state transaction results
    pub state_transaction_results: Vec<(StateTransition, ExecTxResult)>,
    /// withdrawal transactions
    pub withdrawal_transactions: Vec<dashcore_rpc::dashcore::Transaction>,
    /// The next validators
    pub validator_set_update: Option<ValidatorSetUpdate>,
    /// The next validators hash
    pub next_validator_set_hash: Vec<u8>,
    /// Root App hash
    pub root_app_hash: [u8; 32],
    /// State ID needed to verify the block, for example height, app version, etc.
    pub state_id: StateId,
    /// Hash of CanonicalBlockId
    pub block_id_hash: [u8; 32],
    /// Block signature
    pub signature: [u8; 96],
    /// Version of Drive app used to generate this block
    pub app_version: u64,
}

/// Options for execution
pub struct MimicExecuteBlockOptions {
    /// don't finalize block
    pub dont_finalize_block: bool,
    /// rounds before finalization
    pub rounds_before_finalization: Option<u32>,
    /// max tx bytes per block
    pub max_tx_bytes_per_block: u64,
    /// run process proposal independently
    pub independent_process_proposal_verification: bool,
}

impl<'a, C: CoreRPCLike> AbciApplication<'a, C> {
    /// Execute a block with various state transitions
    /// Returns the withdrawal transactions that were signed in the block
    pub fn mimic_execute_block(
        &self,
        proposer_pro_tx_hash: [u8; 32],
        current_quorum: &TestQuorumInfo,
        proposed_version: ProtocolVersion,
        block_info: BlockInfo,
        round: u32,
        expect_validation_errors: &[u32],
        expect_vote_extension_errors: bool,
        state_transitions: Vec<StateTransition>,
        options: MimicExecuteBlockOptions,
    ) -> Result<MimicExecuteBlockOutcome, Error> {
        // This will be NONE, except on init chain
        let original_block_execution_context = self
            .platform
            .block_execution_context
            .read()
            .unwrap()
            .as_ref()
            .cloned();

        let transaction_guard = self.transaction.read().unwrap();

        let init_chain_root_hash = transaction_guard.as_ref().map(|transaction| {
            self.platform
                .drive
                .grove
                .root_hash(Some(transaction))
                .unwrap()
                .unwrap()
        });

        drop(transaction_guard);

        const APP_VERSION: u64 = 0;

        let mut rng = StdRng::seed_from_u64(block_info.height);

        let next_validators_hash: [u8; 32] = rng.gen(); // We fake a block hash for the test
        let serialized_state_transitions = state_transitions
            .iter()
            .map(|st| st.serialize_to_bytes().map_err(Error::Protocol))
            .collect::<Result<Vec<Vec<u8>>, Error>>()?;

        let BlockInfo {
            time_ms,
            height,
            mut core_height,
            epoch: _,
        } = block_info;
        let time = Timestamp {
            seconds: (time_ms / 1000) as i64,
            nanos: ((time_ms % 1000) * 1000) as i32,
        };
        // PREPARE (also processes internally)

        let request_prepare_proposal = RequestPrepareProposal {
            max_tx_bytes: options.max_tx_bytes_per_block as i64,
            txs: serialized_state_transitions.clone(),
            local_last_commit: None,
            misbehavior: vec![],
            height: height as i64,
            time: Some(time.clone()),
            next_validators_hash: next_validators_hash.to_vec(),
            round: round as i32,
            core_chain_locked_height: core_height,
            proposer_pro_tx_hash: proposer_pro_tx_hash.to_vec(),
            proposed_app_version: proposed_version as u64,
            version: Some(Consensus {
                block: 0,
                app: APP_VERSION,
            }),
            quorum_hash: current_quorum.quorum_hash.to_byte_array().to_vec(),
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
            core_chain_lock_update,
            validator_set_update,
        } = response_prepare_proposal;

        if let Some(core_chain_lock_update) = core_chain_lock_update.as_ref() {
            core_height = core_chain_lock_update.core_block_height;
        }

        tx_results.iter().try_for_each(|tx_result| {
            if tx_result.code > 0 && !expect_validation_errors.contains(&tx_result.code) {
                Err(Error::Abci(AbciError::GenericWithCode(tx_result.code)))
            } else {
                Ok(())
            }
        })?;

        let state_transactions_to_process = tx_records
            .into_iter()
            .filter_map(|tx_record| {
                if tx_record.action == TxAction::Removed as i32
                    || tx_record.action == TxAction::Delayed as i32
                {
                    None
                } else {
                    Some(tx_record.tx)
                }
            })
            .collect::<Vec<_>>();

        let state_transaction_results = state_transitions.into_iter().zip(tx_results).collect();

        // PROCESS

        let state_id = StateId {
            app_hash: app_hash.clone(),
            app_version: APP_VERSION,
            core_chain_locked_height: core_height,
            height,
            time: time.to_milis(),
        };
        let state_id_hash = state_id
            .calculate_msg_hash(CHAIN_ID, height as i64, round as i32)
            .expect("cannot hash state id");

        let block_header_hash: [u8; 32] = rng.gen();
        let block_id = BlockId {
            hash: block_header_hash.to_vec(),
            part_set_header: Some(PartSetHeader {
                total: 0,
                hash: vec![0u8; 32],
            }),
            state_id: state_id_hash,
        };
        let block_id_hash = block_id
            .calculate_msg_hash(CHAIN_ID, height as i64, round as i32)
            .expect("cannot hash block id");

        let request_process_proposal = RequestProcessProposal {
            txs: state_transactions_to_process.clone(),
            proposed_last_commit: None,
            misbehavior: vec![],
            hash: block_header_hash.to_vec(),
            height: height as i64,
            time: Some(Timestamp {
                seconds: (time_ms / 1000) as i64,
                nanos: ((time_ms % 1000) * 1000) as i32,
            }),
            next_validators_hash: next_validators_hash.to_vec(),
            round: round as i32,
            core_chain_locked_height: core_height,
            core_chain_lock_update,
            proposer_pro_tx_hash: proposer_pro_tx_hash.to_vec(),
            proposed_app_version: proposed_version as u64,
            version: Some(Consensus {
                block: 0,
                app: APP_VERSION,
            }),
            quorum_hash: current_quorum.quorum_hash.to_byte_array().to_vec(),
        };

        if !options.independent_process_proposal_verification {
            //we just check as if we were the proposer
            //we must call process proposal so the app hash is set
            self.process_proposal(request_process_proposal)
                .unwrap_or_else(|e| {
                    panic!(
                        "should skip processing (because we prepared it) block #{} at time #{} : {:?}",
                        block_info.height, block_info.time_ms, e
                    )
                });
        } else {
            //we first call process proposal as the proposer
            //we must call process proposal so the app hash is set
            self.process_proposal(request_process_proposal.clone())
                .unwrap_or_else(|e| {
                    panic!(
                        "should skip processing (because we prepared it) block #{} at time #{} : {:?}",
                        block_info.height, block_info.time_ms, e
                    )
                });

            let mut block_execution_context =
                self.platform.block_execution_context.write().unwrap();

            let application_hash = block_execution_context
                .as_ref()
                .expect("expected a block execution context")
                .block_state_info()
                .app_hash()
                .expect("expected an application hash after process proposal");

            *block_execution_context = original_block_execution_context.clone();
            drop(block_execution_context);

            if let Some(init_chain_root_hash) = init_chain_root_hash
            //we are in init chain
            {
                // special logic on init chain
                let transaction = self.transaction.write().unwrap();

                let transaction = transaction.as_ref().ok_or(Error::Execution(
                    ExecutionError::NotInTransaction(
                        "trying to finalize block without a current transaction",
                    ),
                ))?;

                transaction
                    .rollback_to_savepoint()
                    .expect("expected to rollback to savepoint");

                let start_root_hash = self
                    .platform
                    .drive
                    .grove
                    .root_hash(Some(transaction))
                    .unwrap()
                    .unwrap();
                assert_eq!(start_root_hash, init_chain_root_hash);
                // this is just to verify that the rollback worked.
            };

            //we call process proposal as if we are a processor
            self.process_proposal(request_process_proposal)
                .unwrap_or_else(|e| {
                    panic!(
                        "should skip processing (because we prepared it) block #{} at time #{} : {:?}",
                        block_info.height, block_info.time_ms, e
                    )
                });

            let block_execution_context = self.platform.block_execution_context.read().unwrap();

            let process_proposal_application_hash = block_execution_context
                .as_ref()
                .expect("expected a block execution context")
                .block_state_info()
                .app_hash()
                .expect("expected an application hash after process proposal");

            assert_eq!(
                application_hash, process_proposal_application_hash,
                "the application hashed are not valid for height {}",
                block_info.height
            );

            let transaction_guard = self.transaction.read().unwrap();

            let transaction = transaction_guard.as_ref().ok_or(Error::Execution(
                ExecutionError::NotInTransaction(
                    "trying to finalize block without a current transaction",
                ),
            ))?;

            let direct_root_hash = self
                .platform
                .drive
                .grove
                .root_hash(Some(transaction))
                .unwrap()
                .unwrap();
            assert_eq!(
                application_hash, direct_root_hash,
                "the application hashed are not valid for height {}",
                block_info.height
            );
        }

        let request_extend_vote = RequestExtendVote {
            hash: block_header_hash.to_vec(),
            height: height as i64,
            round: round as i32,
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
                hash: block_header_hash.to_vec(),
                validator_pro_tx_hash: validator.pro_tx_hash.to_byte_array().to_vec(),
                height: height as i64,
                round: round as i32,
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
            if !expect_vote_extension_errors
                && response_validate_vote_extension.status != VerifyStatus::Accept as i32
            {
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
                    "block execution context must be set in block begin handler for mimic block execution",
                )))?.v0()?;

        let extensions = block_execution_context
            .withdrawal_transactions
            .keys()
            .map(|tx_id| {
                VoteExtension {
                    r#type: VoteExtensionType::ThresholdRecover as i32,
                    extension: tx_id.to_byte_array().to_vec(),
                    signature: vec![], //todo: signature
                    sign_request_id: None,
                }
            })
            .collect();

        //todo: tidy up and fix
        let withdrawals = block_execution_context
            .withdrawal_transactions
            .values()
            .map(|transaction| {
                let AssetUnlockBaseTransactionInfo {
                    version,
                    lock_time,
                    output,
                    base_payload,
                } = Decodable::consensus_decode(&mut transaction.reader()).expect("a");
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
                        quorum_sig: BLSSignature::from([0; 96]),
                    })),
                }
            })
            .collect();

        drop(guarded_block_execution_context);

        // We need to sign the block

        let quorum_type = self.platform.config.validator_set_quorum_type();
        let state_id_hash = state_id
            .calculate_msg_hash(CHAIN_ID, height as i64, round as i32)
            .expect("cannot calculate state id hash");

        let commit = CanonicalVote {
            block_id: block_id_hash.clone(),
            state_id: state_id_hash,
            chain_id: CHAIN_ID.to_string(),
            height: height as i64,
            round: round as i64,
            r#type: SignedMsgType::Precommit.into(),
        };

        let quorum_hash = current_quorum.quorum_hash.to_byte_array().to_vec();

        let mut commit_info = CommitInfo {
            round: round as i32,
            quorum_hash: quorum_hash.clone(),
            block_signature: Default::default(),
            threshold_vote_extensions: extensions,
        };
        //if not in testing this will default to true
        if self.platform.config.testing_configs.block_signing {
            let quorum_hash: [u8; 32] = quorum_hash.try_into().expect("wrong quorum hash len");
            let digest = commit
                .sign_digest(
                    CHAIN_ID,
                    quorum_type as u8,
                    &quorum_hash,
                    height as i64,
                    round as i32,
                )
                .expect("expected to sign digest");

            tracing::trace!(
            digest=hex::encode(&digest),
                        ?state_id,
                        ?commit,
                        ?quorum_type,
                        ?quorum_hash,
                        CHAIN_ID,
                        height,
                        round,
                        public_key = ?current_quorum.public_key,
                        "Signing block"
                    );
            let block_signature = current_quorum.private_key.sign(digest.as_slice());

            commit_info.block_signature = block_signature.to_bytes().to_vec();
        } else {
            commit_info.block_signature = [0u8; 96].to_vec();
        }

        let next_validator_set_hash = validator_set_update
            .as_ref()
            .map(|update| update.quorum_hash.clone())
            .unwrap_or(current_quorum.quorum_hash.to_byte_array().to_vec());

        let block = Block {
            header: Some(Header {
                version: Some(Consensus {
                    block: 0, //todo
                    app: APP_VERSION,
                }),
                chain_id: CHAIN_ID.to_string(),
                height: height as i64,
                time: Some(time),
                last_block_id: None,
                last_commit_hash: [0; 32].to_vec(),
                data_hash: [0; 32].to_vec(),
                validators_hash: current_quorum.quorum_hash.to_byte_array().to_vec(),
                next_validators_hash: next_validator_set_hash.clone(),
                consensus_hash: [0; 32].to_vec(),
                next_consensus_hash: [0; 32].to_vec(),
                app_hash: app_hash.clone(),
                results_hash: [0; 32].to_vec(),
                evidence_hash: vec![],
                proposed_app_version: proposed_version as u64,
                proposer_pro_tx_hash: proposer_pro_tx_hash.to_vec(),
                core_chain_locked_height: core_height,
            }),
            data: Some(Data {
                txs: state_transactions_to_process,
            }),
            evidence: Some(EvidenceList { evidence: vec![] }),
            last_commit: None,
            core_chain_lock: None,
        };

        let request_finalize_block = RequestFinalizeBlock {
            commit: Some(commit_info.clone()),
            misbehavior: vec![],
            hash: block_header_hash.to_vec(),
            height: height as i64,
            round: round as i32,
            block: Some(block),
            block_id: Some(block_id),
        };

        let transaction_guard = self.transaction.read().unwrap();

        let transaction = transaction_guard.as_ref().ok_or(Error::Execution(
            ExecutionError::NotInTransaction(
                "trying to finalize block without a current transaction",
            ),
        ))?;

        let root_hash_before_finalization = self
            .platform
            .drive
            .grove
            .root_hash(Some(transaction))
            .unwrap()
            .unwrap();
        assert_eq!(app_hash, root_hash_before_finalization);
        drop(transaction_guard);

        if !options.dont_finalize_block
            && options.rounds_before_finalization.unwrap_or_default() <= round
        {
            self.finalize_block(request_finalize_block)
                .unwrap_or_else(|e| {
                    panic!(
                        "should finalize block #{} round#{} at time #{} : {:?}",
                        block_info.height, round, block_info.time_ms, e
                    )
                });
            let root_hash_after_finalization =
                self.platform.drive.grove.root_hash(None).unwrap().unwrap();
            assert_eq!(app_hash, root_hash_after_finalization);
        }

        Ok(MimicExecuteBlockOutcome {
            state_transaction_results,
            app_version: APP_VERSION,
            withdrawal_transactions: withdrawals,
            validator_set_update,
            next_validator_set_hash,
            root_app_hash: app_hash
                .try_into()
                .expect("expected 32 bytes for the root hash"),
            state_id,
            block_id_hash: block_id_hash.try_into().expect("invalid block id hash len"),
            signature: commit_info
                .block_signature
                .try_into()
                .expect("signature mut be 96 bytes long"),
        })
    }
}
