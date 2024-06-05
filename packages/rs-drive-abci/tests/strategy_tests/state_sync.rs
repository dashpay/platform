#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::fs;
use std::path::PathBuf;
    use std::time::Instant;
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use tenderdash_abci::Application;
    use tenderdash_abci::proto::abci::{RequestListSnapshots, RequestOfferSnapshot, RequestLoadSnapshotChunk, RequestApplySnapshotChunk};
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{DocumentFieldFillSize, DocumentFieldFillType};
    use dpp::tests::json_document::json_document_to_created_contract;
    use drive_abci::abci::app::{FullAbciApplication, StateSyncAbciApplication};
    use drive_abci::abci::config::StateSyncAbciConfig;
    use drive_abci::config::{ExecutionConfig, PlatformConfig, PlatformTestConfig};
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::{ChainExecutionOutcome, NetworkStrategy};

    fn generate_random_path(prefix: &str, suffix: &str, len: usize) -> String {
        let random_string: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(len)
            .map(char::from)
            .collect();
        format!("{}{}{}", prefix, random_string, suffix)
    }

    fn create_dir_if_not_exists(path: &PathBuf) -> std::io::Result<()> {
        if !path.exists() {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }

    fn remove_dir(path: &PathBuf) -> std::io::Result<()> {
        if path.exists() {
            fs::remove_dir_all(path)?;
        }
        Ok(())
    }

    #[test]
    fn run_state_sync_0(
    ) {
        let platform_version = PlatformVersion::latest();
        let created_contract = json_document_to_created_contract(
            "tests/supporting_files/contract/dashpay/dashpay-contract-all-mutable.json",
            1,
            true,
            platform_version,
        )
            .expect("expected to get contract from a json document");

        let contract = created_contract.data_contract();

        let document_insertion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionInsertRandom(
                DocumentFieldFillType::FillIfNotRequired,
                DocumentFieldFillSize::AnyDocumentFillSize,
            ),
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let document_deletion_op = DocumentOp {
            contract: contract.clone(),
            action: DocumentAction::DocumentActionDelete,
            document_type: contract
                .document_type_for_name("contactRequest")
                .expect("expected a profile document type")
                .to_owned_document_type(),
        };

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![
                    Operation {
                        op_type: OperationType::Document(document_insertion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..7,
                            chance_per_block: None,
                        },
                    },
                    Operation {
                        op_type: OperationType::Document(document_deletion_op),
                        frequency: Frequency {
                            times_per_block_range: 1..7,
                            chance_per_block: None,
                        },
                    },
                ],
                start_identities: StartIdentities::default(),
                identity_inserts: IdentityInsertInfo {
                    frequency: Frequency {
                        times_per_block_range: 1..2,
                        chance_per_block: None,
                    },
                    ..Default::default()
                },

                identity_contract_nonce_gaps: None,
                signer: None,
            },
            total_hpmns: 100,
            extra_normal_mns: 0,
            validator_quorum_count: 24,
            chain_lock_quorum_count: 24,
            upgrading_info: None,

            proposer_strategy: Default::default(),
            rotate_quorums: false,
            failure_testing: None,
            query_testing: None,
            verify_state_transition_results: true,
            ..Default::default()
        };
        let day_in_ms = 1000 * 60 * 60 * 24;

        let base_test_directory = PathBuf::from(generate_random_path("/Users/odysseasg/Development/platform/target/tmp/", "", 12));

        let mut checkpoint_test_directory = base_test_directory.clone();
        checkpoint_test_directory.push("checkpoints");

        create_dir_if_not_exists(&checkpoint_test_directory).expect("should create checkpoint directory");
        println!("checkpoint_test_directory: {}", checkpoint_test_directory.to_str().unwrap().to_string());

        let mut db_test_directory = base_test_directory.clone();
        db_test_directory.push("db");

        create_dir_if_not_exists(&db_test_directory).expect("should create db directory");
        println!("db_test_directory: {}", db_test_directory.to_str().unwrap().to_string());

        let mut local_state_sync_config = StateSyncAbciConfig::default();
        local_state_sync_config.snapshots_frequency = 10;
        local_state_sync_config.max_num_snapshots = 3;
        local_state_sync_config.snapshots_enabled = true;
        local_state_sync_config.checkpoints_path = checkpoint_test_directory;

        let config = PlatformConfig {
            validator_set_quorum_size: 100,
            validator_set_quorum_type: "llmq_100_67".to_string(),
            chain_lock_quorum_type: "llmq_100_67".to_string(),
            execution: ExecutionConfig {
                verify_sum_trees: true,
                validator_set_rotation_block_count: 100,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            testing_configs: PlatformTestConfig::default_with_no_block_signing(),
            db_path: db_test_directory,
            state_sync_config: local_state_sync_config,
            ..Default::default()
        };

        let block_count = 120;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let source_outcome = run_chain_for_strategy(&mut platform, block_count, strategy, config.clone(), 15);
        let source_snapshots = source_outcome.abci_app.list_snapshots(RequestListSnapshots::default()).expect("should expected snapshots_1");
        for s in &source_snapshots.snapshots {
            println!("snapshot height:{} app_hash:{}", s.height, hex::encode(&s.hash));
        }
        let best_snapshot = match source_snapshots.snapshots.iter().max_by_key(|s| s.height) {
            Some(s) => s,
            None => {
                println!("no snapshots available. exit");
                return; // Return early if no item is found
            }
        };
        println!("best_snapshot height:{} app_hash:{}", best_snapshot.height, hex::encode(&best_snapshot.hash));

        println!("source subtrees metadata:");
        let source_metadata = source_outcome
            .abci_app
            .platform
            .drive
            .grove
            .get_subtrees_metadata(None)
            .expect("should get subtrees metadata");
        println!("num_subtrees:{:?}", source_metadata.data.len());

        let target_platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let target_abci_app = FullAbciApplication::new(&target_platform);

        let mut offer_snapshot_request = RequestOfferSnapshot::default();
        offer_snapshot_request.snapshot = Some(best_snapshot.clone());
        offer_snapshot_request.app_hash = best_snapshot.hash.to_vec();

        let _ = target_abci_app.offer_snapshot(offer_snapshot_request).expect("should offer_snapshot succeed");

        let mut chunk_queue : VecDeque<Vec<u8>> = VecDeque::new();
        chunk_queue.push_back(best_snapshot.hash.to_vec());

        let start_time = Instant::now();

        let mut counter = 0;
        while let Some(chunk_id) = chunk_queue.pop_front() {
            let mut request_load_chunk = RequestLoadSnapshotChunk::default();
            request_load_chunk.chunk_id = chunk_id.to_vec();
            request_load_chunk.height = best_snapshot.height;
            request_load_chunk.version = best_snapshot.version;

            let load_chunk_response = source_outcome.abci_app.load_snapshot_chunk(request_load_chunk).expect("should fetch chunk");

            let mut request_apply_chunk = RequestApplySnapshotChunk::default();
            request_apply_chunk.chunk_id = chunk_id.to_vec();
            request_apply_chunk.chunk = load_chunk_response.chunk;

            let elapsed = start_time.elapsed();
            let chunk_id_cloned = request_apply_chunk.chunk_id.to_vec();
            let apply_chunk_response = target_abci_app.apply_snapshot_chunk(request_apply_chunk).expect("should apply chunk succeed");
            println!("#{} apply:{} returned:{} queue:{} {:.2?}", counter, hex::encode(chunk_id_cloned), apply_chunk_response.next_chunks.len(), chunk_queue.len(), elapsed);
            chunk_queue.extend(apply_chunk_response.next_chunks);
            counter += 1;
        }

        println!("source app_hash:{}", hex::encode(
            source_outcome
                .abci_app
                .platform
                .drive
                .grove
                .root_hash(None)
                .unwrap()
                .unwrap()
        ));
        println!("target app_hash:{}", hex::encode(
            target_abci_app
                .platform
                .drive
                .grove
                .root_hash(None)
                .unwrap()
                .unwrap()
        ));
    }
}