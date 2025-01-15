#[cfg(test)]
mod tests {
    use crate::execution::run_chain_for_strategy;
    use crate::strategy::NetworkStrategy;
    use dpp::data_contract::accessors::v0::DataContractV0Getters;
    use dpp::data_contract::document_type::random_document::{
        DocumentFieldFillSize, DocumentFieldFillType,
    };
    use dpp::tests::json_document::json_document_to_created_contract;
    use drive_abci::abci::app::{FullAbciApplication, PlatformApplication};
    use drive_abci::abci::config::StateSyncAbciConfig;
    use drive_abci::config::{ExecutionConfig, PlatformConfig};
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    use std::collections::VecDeque;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};
    use strategy_tests::frequency::Frequency;
    use strategy_tests::operations::{DocumentAction, DocumentOp, Operation, OperationType};
    use strategy_tests::{IdentityInsertInfo, StartIdentities, Strategy};
    use tenderdash_abci::proto::abci::{
        RequestApplySnapshotChunk, RequestListSnapshots, RequestLoadSnapshotChunk,
        RequestOfferSnapshot,
    };
    use tenderdash_abci::Application;

    fn generate_random_path(prefix: &str, suffix: &str, len: usize) -> String {
        let random_string: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(len)
            .map(char::from)
            .collect();
        format!("{}/{}{}", prefix, random_string, suffix)
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

    /*fn get_target_folder() -> PathBuf {
        // Use the environment variable `CARGO_MANIFEST_DIR` to locate the current package
        let manifest_dir = env!("CARGO_MANIFEST_DIR");

        // Traverse up the directory tree to find the workspace root
        let mut current_dir = PathBuf::from(manifest_dir);
        while !current_dir.join("Cargo.lock").exists() {
            current_dir.pop(); // Go up one level
        }

        // The `target` folder is located at the workspace root
        current_dir.join("target")
    }*/

    fn get_target_tmp_folder() -> PathBuf {
        PathBuf::from("/Users/odysseasg/Downloads/state_sync/").join("tmp")
        //get_target_folder().join("tmp")
    }

    #[test]
    fn run_state_sync() {
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

        let strategy = NetworkStrategy {
            strategy: Strategy {
                start_contracts: vec![(created_contract, None)],
                operations: vec![Operation {
                    op_type: OperationType::Document(document_insertion_op),
                    frequency: Frequency {
                        times_per_block_range: 50000..50002,
                        chance_per_block: None,
                    },
                }],
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

        let base_test_directory = PathBuf::from(generate_random_path(
            get_target_tmp_folder().to_str().unwrap(),
            "",
            12,
        ));

        let mut checkpoint_test_directory = base_test_directory.clone();
        checkpoint_test_directory.push("checkpoints");

        create_dir_if_not_exists(&checkpoint_test_directory)
            .expect("should create checkpoint directory");
        println!(
            "checkpoint_test_directory: {}",
            checkpoint_test_directory.to_str().unwrap()
        );

        let mut db_test_directory = base_test_directory.clone();
        db_test_directory.push("db");

        create_dir_if_not_exists(&db_test_directory).expect("should create db directory");
        println!("db_test_directory: {}", db_test_directory.to_str().unwrap());

        let local_state_sync_config = StateSyncAbciConfig {
            snapshots_enabled: true,
            checkpoints_path: checkpoint_test_directory,
            snapshots_frequency: 10,
            max_num_snapshots: 3,
        };

        let config = PlatformConfig {
            execution: ExecutionConfig {
                verify_sum_trees: true,
                ..Default::default()
            },
            block_spacing_ms: day_in_ms,
            db_path: db_test_directory,
            state_sync_config: local_state_sync_config,
            ..Default::default()
        };

        let block_count = 50;
        let mut platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();

        let source_outcome = run_chain_for_strategy(
            &mut platform,
            block_count,
            strategy,
            config.clone(),
            15,
            &mut None,
        );
        let source_snapshots = source_outcome
            .abci_app
            .list_snapshots(RequestListSnapshots::default())
            .expect("should expected snapshots");
        for s in &source_snapshots.snapshots {
            println!(
                "snapshot height:{} app_hash:{}",
                s.height,
                hex::encode(&s.hash)
            );
        }
        let best_snapshot = match source_snapshots.snapshots.iter().max_by_key(|s| s.height) {
            Some(s) => s,
            None => {
                println!("no snapshots available. exit");
                return; // Return early if no item is found
            }
        };
        println!(
            "best_snapshot height:{} app_hash:{}",
            best_snapshot.height,
            hex::encode(&best_snapshot.hash)
        );

        let target_platform = TestPlatformBuilder::new()
            .with_config(config.clone())
            .build_with_mock_rpc();
        let target_abci_app = FullAbciApplication::new(&target_platform);

        let offer_snapshot_request = RequestOfferSnapshot {
            snapshot: Some(best_snapshot.clone()),
            app_hash: best_snapshot.hash.to_vec(),
        };

        let _ = target_abci_app
            .offer_snapshot(offer_snapshot_request)
            .expect("should offer_snapshot succeed");

        let mut chunk_queue: VecDeque<Vec<u8>> = VecDeque::new();
        chunk_queue.push_back(best_snapshot.hash.to_vec());

        let start_time = Instant::now();

        let mut duration_sum_fetch: Duration = Duration::ZERO;
        let mut duration_sum_apply: Duration = Duration::ZERO;

        let mut chunk_counter = 0;
        let mut ops_counter = 0;
        while let Some(chunk_id) = chunk_queue.pop_front() {
            let request_load_chunk = RequestLoadSnapshotChunk {
                height: best_snapshot.height,
                version: best_snapshot.version,
                chunk_id: chunk_id.clone(),
            };
            let start_time_fetch = Instant::now();
            let load_chunk_response = source_outcome
                .abci_app
                .load_snapshot_chunk(request_load_chunk)
                .expect("should fetch chunk");
            duration_sum_fetch += start_time_fetch.elapsed();

            let request_apply_chunk = RequestApplySnapshotChunk {
                chunk_id,
                chunk: load_chunk_response.chunk,
                ..Default::default()
            };
            let request_apply_num_ops = request_apply_chunk.chunk.len();
            ops_counter += request_apply_num_ops;

            let elapsed = start_time.elapsed();
            let chunk_id_hex = hex::encode(&request_apply_chunk.chunk_id);
            let start_time_apply = Instant::now();
            let apply_chunk_response = target_abci_app
                .apply_snapshot_chunk(request_apply_chunk)
                .expect("should apply chunk succeed");
            duration_sum_apply += start_time_apply.elapsed();
            println!(
                "#{} apply:{} num_ops:{} returned:{} queue:{} {:.2?}",
                chunk_counter,
                chunk_id_hex,
                request_apply_num_ops,
                apply_chunk_response.next_chunks.len(),
                chunk_queue.len(),
                elapsed
            );
            chunk_queue.extend(apply_chunk_response.next_chunks);
            chunk_counter += 1;
        }
        println!("total chunks:{} ops:{}", chunk_counter, ops_counter);
        println!("duration_sum_fetch: {}", duration_sum_fetch.as_secs_f64());
        println!("duration_sum_apply: {}", duration_sum_apply.as_secs_f64());

        println!(
            "source app_hash:{}",
            hex::encode(
                source_outcome
                    .abci_app
                    .platform
                    .drive
                    .grove
                    .root_hash(None, &PlatformVersion::latest().drive.grove_version)
                    .unwrap()
                    .unwrap()
            )
        );
        println!(
            "target app_hash:{}",
            hex::encode(
                target_abci_app
                    .platform()
                    .drive
                    .grove
                    .root_hash(None, &PlatformVersion::latest().drive.grove_version)
                    .unwrap()
                    .unwrap()
            )
        );
    }
}
