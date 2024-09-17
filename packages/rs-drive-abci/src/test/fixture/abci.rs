//! Execution Tests
//!

use crate::config::PlatformConfig;
use dpp::version::PlatformVersion;
use tenderdash_abci::proto::abci::RequestInitChain;
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::types::{ConsensusParams, VersionParams};

/// Creates static init chain request fixture
pub fn static_init_chain_request(config: &PlatformConfig) -> RequestInitChain {
    let platform_version = PlatformVersion::get(config.initial_protocol_version)
        .expect("expected to get platform version");
    RequestInitChain {
        time: Some(Timestamp {
            seconds: 0,
            nanos: 0,
        }),
        chain_id: "strategy_tests".to_string(),
        consensus_params: Some(ConsensusParams {
            version: Some(VersionParams {
                app_version: config.initial_protocol_version as u64,
                consensus_version: platform_version.consensus.tenderdash_consensus_version as i32,
            }),
            ..Default::default()
        }),
        validator_set: None,
        app_state_bytes: [0u8; 32].to_vec(),
        initial_height: config.abci.genesis_height as i64,
        initial_core_height: config.abci.genesis_core_height,
    }
}
