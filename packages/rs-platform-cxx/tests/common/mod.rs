// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Shared test setup: a [`Client`] driven by `dash-sdk`'s mock transport
//! (no network), configured the way an embedder configures the real one.

#![allow(dead_code)]

use std::sync::Arc;

use dash_platform_cxx::client::Client;
use dash_platform_cxx::provider::{Context, QuorumKey};
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::{Sdk, SdkBuilder};
use platform_version::version::PlatformVersion;

/// The Platform LLMQ type of the fixture network.
pub const QUORUM_TYPE: u32 = 106;
/// Tenderdash chain id the fixtures were signed for.
pub const CHAIN_ID: &str = "dash-testnet-51";
/// Protocol version the fixtures were generated under.
pub const PROTOCOL_VERSION: u32 = 12;

pub fn hex_vec(s: &str) -> Vec<u8> {
    hex::decode(s).expect("fixture hex")
}

pub fn hex32(s: &str) -> [u8; 32] {
    hex_vec(s).try_into().expect("32 bytes")
}

pub fn fixture_context() -> Context {
    Context {
        network: Network::Testnet,
        platform_quorum_type: QUORUM_TYPE,
        tenderdash_chain_id: CHAIN_ID.to_string(),
        protocol_version: PROTOCOL_VERSION,
        platform_activation_height: 0,
    }
}

/// A client whose SDK replays mock expectations instead of talking to a
/// node. The mock SDK shares the client's context provider, so quorum keys
/// pushed through the client reach the verifier exactly as in production.
pub fn mock_client() -> Client {
    let client = Client::new();
    client
        .set_context(fixture_context())
        .expect("fixture context");
    let sdk: Sdk = SdkBuilder::new_mock()
        .with_network(Network::Testnet)
        .with_context_provider(Arc::clone(client.provider()))
        .with_version(PlatformVersion::get(PROTOCOL_VERSION).expect("fixture version"))
        .build()
        .expect("mock sdk");
    client.set_sdk(sdk).expect("install mock sdk");
    client
}

pub fn push_quorum_key(client: &Client, quorum_hash: [u8; 32], public_key: [u8; 48]) {
    client
        .provider()
        .update_quorum_keys(vec![QuorumKey {
            quorum_hash,
            public_key,
        }])
        .expect("quorum keys");
}
