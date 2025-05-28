mod mainnet_for_tests_core_rpc_like;

#[cfg(test)]
mod tests {
    use crate::mainnet_for_tests_core_rpc_like::MainnetForTestsCoreRpcLike;
    use drive_abci::config::PlatformConfig;
    use drive_abci::test::helpers::setup::TestPlatformBuilder;
    use platform_version::version::PlatformVersion;
    use std::fs::File;
    use tenderdash_abci::proto::abci::{CommitInfo, RequestInitChain, RequestProcessProposal};
    use tenderdash_abci::proto::google::protobuf::{Duration, Timestamp};
    use tenderdash_abci::proto::types::version_params::ConsensusVersion::ConsensusVersion0;
    use tenderdash_abci::proto::types::{
        AbciParams, BlockParams, ConsensusParams, CoreChainLock, EvidenceParams, SynchronyParams,
        TimeoutParams, ValidatorParams, VersionParams,
    };
    use tenderdash_abci::proto::version::Consensus;
    use tracing_subscriber::fmt::writer::BoxMakeWriter;

    #[test]
    fn test_mainnet_genesis_block() {
        // Create the output file (append mode)
        let file = File::create("test_log.txt").expect("Failed to create log file");
        let writer = BoxMakeWriter::new(file);

        tracing_subscriber::fmt::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::new(
                "error,drive::util::grove_operations=trace,drive_grovedb_operations=trace",
            ))
            .json()
            .pretty()
            .with_file(false)
            .with_line_number(false)
            .with_ansi(false)
            .without_time()
            .with_level(false)
            .with_target(false)
            .with_writer(writer)
            .try_init()
            .ok();

        let platform_version = PlatformVersion::first();
        let mainnet_for_tests_core_rpc_like = MainnetForTestsCoreRpcLike::default();
        let mut platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default_mainnet())
            .build_with_rpc(mainnet_for_tests_core_rpc_like, Some(1));

        let request_init_chain = RequestInitChain {
            time: Some(Timestamp {
                seconds: 1748255670,
                nanos: 751000000,
            }),
            chain_id: "evo1".to_string(),
            consensus_params: Some(ConsensusParams {
                block: Some(BlockParams {
                    max_bytes: 2097152,
                    max_gas: 57631392000,
                }),
                evidence: Some(EvidenceParams {
                    max_age_num_blocks: 100000,
                    max_age_duration: Some(Duration {
                        seconds: 172800,
                        nanos: 0,
                    }),
                    max_bytes: 0,
                }),
                validator: Some(ValidatorParams {
                    pub_key_types: vec!["bls12381".to_string()],
                }),
                version: Some(VersionParams {
                    app_version: 1,
                    consensus_version: ConsensusVersion0 as i32,
                }),
                synchrony: Some(SynchronyParams {
                    message_delay: Some(Duration {
                        seconds: 70,
                        nanos: 0,
                    }),
                    precision: Some(Duration {
                        seconds: 1,
                        nanos: 0,
                    }),
                }),
                timeout: Some(TimeoutParams {
                    propose: Some(Duration {
                        seconds: 50,
                        nanos: 0,
                    }),
                    propose_delta: Some(Duration {
                        seconds: 5,
                        nanos: 0,
                    }),
                    vote: Some(Duration {
                        seconds: 10,
                        nanos: 0,
                    }),
                    vote_delta: Some(Duration {
                        seconds: 1,
                        nanos: 0,
                    }),
                }),
                abci: Some(AbciParams { recheck_tx: true }),
            }),
            validator_set: None,
            app_state_bytes: vec![],
            initial_height: 1,
            initial_core_height: 0,
        };
        let transaction = platform.drive.grove.start_transaction();
        platform
            .init_chain(request_init_chain, &transaction)
            .expect("expected to initialize chain");

        // assert_eq!(
        //     hex::encode(
        //         platform
        //             .drive
        //             .grove
        //             .root_hash(Some(&transaction), &platform_version.drive.grove_version)
        //             .unwrap()
        //             .expect("expected root hash")
        //     ),
        //     "e8c91db2206527b96e722812bbb9f9513c544fd8c01722208c2d183d5dc62477"
        // );

        let process_proposal_request = RequestProcessProposal {
            txs: vec![],
            proposed_last_commit: Some(CommitInfo {
                round: 0,
                quorum_hash: vec![],
                block_signature: vec![],
                threshold_vote_extensions: vec![],
            }),
            misbehavior: vec![],
            hash: vec![
                32, 254, 0, 241, 13, 89, 185, 197, 72, 126, 225, 140, 26, 41, 138, 4, 217, 222, 88,
                244, 20, 224, 77, 97, 119, 158, 88, 47, 70, 43, 173, 127,
            ],
            height: 1,
            round: 3,
            time: Some(Timestamp {
                seconds: 1724795532,
                nanos: 0,
            }),
            next_validators_hash: vec![
                93, 233, 176, 110, 95, 191, 179, 215, 221, 251, 217, 108, 202, 129, 143, 45, 206,
                192, 77, 218, 189, 2, 68, 20, 9, 150, 34, 18, 133, 59, 60, 46,
            ],
            core_chain_locked_height: 2132092,
            core_chain_lock_update: Some(CoreChainLock {
                core_block_height: 2132092,
                core_block_hash: vec![
                    238, 174, 184, 224, 32, 157, 22, 225, 82, 228, 83, 27, 30, 70, 104, 80, 140,
                    179, 156, 48, 53, 32, 174, 181, 7, 0, 0, 0, 0, 0, 0, 0,
                ],
                signature: vec![
                    134, 115, 36, 11, 6, 184, 97, 239, 190, 41, 54, 70, 129, 153, 191, 10, 193, 81,
                    51, 135, 90, 31, 241, 184, 114, 33, 26, 166, 107, 202, 228, 232, 253, 234, 180,
                    67, 45, 98, 134, 80, 193, 131, 221, 92, 247, 246, 157, 224, 5, 245, 4, 169, 39,
                    97, 226, 154, 220, 155, 251, 177, 59, 135, 39, 89, 164, 17, 29, 25, 196, 120,
                    235, 59, 65, 222, 15, 155, 198, 182, 64, 191, 160, 81, 142, 79, 97, 86, 105,
                    208, 243, 133, 22, 67, 15, 35, 213, 169,
                ],
            }),
            proposer_pro_tx_hash: vec![
                9, 60, 121, 36, 116, 61, 115, 91, 162, 247, 61, 108, 13, 173, 163, 11, 171, 173,
                216, 208, 131, 253, 53, 16, 106, 179, 21, 97, 248, 80, 211, 26,
            ],
            proposed_app_version: 1,
            version: Some(Consensus { block: 14, app: 1 }),
            quorum_hash: vec![
                0, 0, 0, 0, 0, 0, 0, 16, 95, 45, 28, 237, 163, 198, 61, 43, 103, 122, 34, 125, 126,
                215, 124, 91, 173, 55, 118, 114, 92, 173, 0, 2,
            ],
        };

        let platform_state = platform.state.load();

        let result = platform
            .run_block_proposal(
                (&process_proposal_request)
                    .try_into()
                    .expect("expected request to be valid"),
                false,
                &platform_state,
                &transaction,
                None,
            )
            .expect("expected to run block proposal")
            .into_data()
            .expect("expected block to be valid");

        assert_eq!(
            hex::encode(result.app_hash).to_ascii_uppercase(),
            "36BA84B1DA372A1E200B9AF8830B7811E3BA23DF00AAD92E79540DEE20A02C48"
        );
    }
}
