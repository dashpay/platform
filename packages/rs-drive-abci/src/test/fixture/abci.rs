// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Execution Tests
//!

use crate::config::PlatformConfig;
use crate::platform_types::required_identity_public_key_set::v0::RequiredIdentityPublicKeysSet;
use crate::platform_types::system_identity_public_keys::v0::SystemIdentityPublicKeysV0;
use dpp::version::PlatformVersion;
use drive::dpp::identity::KeyType::ECDSA_SECP256K1;
use rand::rngs::StdRng;
use rand::SeedableRng;
use tenderdash_abci::proto::abci::RequestInitChain;
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::types::{ConsensusParams, VersionParams};

/// Creates static init chain request fixture
pub fn static_init_chain_request(config: &PlatformConfig) -> RequestInitChain {
    RequestInitChain {
        time: Some(Timestamp {
            seconds: 0,
            nanos: 0,
        }),
        chain_id: "strategy_tests".to_string(),
        consensus_params: Some(ConsensusParams {
            version: Some(VersionParams {
                app_version: config.initial_protocol_version as u64,
            }),
            ..Default::default()
        }),
        validator_set: None,
        app_state_bytes: [0u8; 32].to_vec(),
        initial_height: config.abci.genesis_height as i64,
        initial_core_height: config.abci.genesis_core_height,
    }
}

/// Creates static system identity public keys fixture
pub fn static_system_identity_public_keys_v0() -> SystemIdentityPublicKeysV0 {
    SystemIdentityPublicKeysV0 {
        masternode_reward_shares_contract_owner: RequiredIdentityPublicKeysSet {
            master: vec![
                3, 51, 164, 44, 98, 142, 140, 147, 206, 3, 134, 133, 111, 143, 34, 57, 200, 75,
                248, 22, 207, 133, 144, 113, 108, 120, 145, 253, 201, 129, 164, 223, 11,
            ],
            high: vec![
                3, 163, 0, 40, 86, 173, 145, 102, 45, 195, 75, 102, 80, 162, 199, 248, 178, 114,
                108, 148, 122, 65, 158, 11, 136, 15, 179, 172, 195, 135, 99, 162, 113,
            ],
        },
        feature_flags_contract_owner: RequiredIdentityPublicKeysSet {
            master: vec![
                2, 35, 147, 72, 99, 130, 165, 187, 38, 40, 86, 196, 159, 134, 152, 39, 161, 199,
                154, 58, 60, 56, 116, 127, 60, 184, 195, 45, 215, 189, 25, 23, 151,
            ],
            high: vec![
                3, 193, 10, 192, 138, 119, 223, 223, 205, 199, 6, 234, 67, 217, 101, 26, 192, 134,
                97, 129, 184, 53, 65, 21, 135, 236, 164, 210, 213, 71, 127, 57, 247,
            ],
        },
        dpns_contract_owner: RequiredIdentityPublicKeysSet {
            master: vec![
                3, 125, 7, 78, 176, 10, 162, 134, 196, 56, 181, 209, 43, 124, 108, 162, 81, 4, 214,
                27, 3, 230, 96, 27, 106, 206, 125, 94, 176, 54, 251, 188, 35,
            ],
            high: vec![
                2, 88, 82, 223, 97, 26, 34, 139, 14, 127, 188, 207, 244, 234, 161, 23, 80, 14, 173,
                132, 98, 40, 9, 234, 127, 192, 93, 207, 109, 45, 187, 193, 212,
            ],
        },
        withdrawals_contract_owner: RequiredIdentityPublicKeysSet {
            master: vec![
                2, 197, 113, 255, 12, 219, 114, 99, 77, 228, 253, 35, 244, 12, 78, 213, 48, 179,
                211, 29, 239, 201, 135, 165, 84, 121, 214, 94, 126, 140, 30, 36, 154,
            ],
            high: vec![
                3, 131, 79, 146, 162, 19, 46, 85, 39, 60, 183, 19, 232, 85, 166, 251, 242, 23, 151,
                4, 131, 12, 25, 9, 68, 112, 114, 13, 100, 52, 206, 69, 71,
            ],
        },
        dashpay_contract_owner: RequiredIdentityPublicKeysSet {
            master: vec![
                2, 238, 109, 155, 21, 237, 28, 49, 5, 53, 41, 119, 57, 230, 153, 115, 64, 109, 189,
                26, 103, 155, 231, 250, 210, 189, 210, 224, 134, 133, 3, 48, 119,
            ],
            high: vec![
                2, 113, 28, 225, 254, 218, 253, 230, 118, 148, 215, 113, 149, 12, 71, 79, 227, 0,
                228, 100, 212, 246, 124, 42, 100, 71, 249, 182, 30, 144, 250, 1, 243,
            ],
        },
    }
}

/// Creates random system identity public keys fixture
pub fn random_system_identity_public_keys(seed: Option<u64>) -> SystemIdentityPublicKeysV0 {
    let platform_version = PlatformVersion::latest();
    let mut rng = match seed {
        None => StdRng::from_entropy(),
        Some(seed_value) => StdRng::seed_from_u64(seed_value),
    };

    SystemIdentityPublicKeysV0 {
        masternode_reward_shares_contract_owner: RequiredIdentityPublicKeysSet {
            master: ECDSA_SECP256K1
                .random_public_key_data(&mut rng, platform_version)
                .unwrap(),
            high: ECDSA_SECP256K1
                .random_public_key_data(&mut rng, platform_version)
                .unwrap(),
        },
        feature_flags_contract_owner: RequiredIdentityPublicKeysSet {
            master: ECDSA_SECP256K1
                .random_public_key_data(&mut rng, platform_version)
                .unwrap(),
            high: ECDSA_SECP256K1
                .random_public_key_data(&mut rng, platform_version)
                .unwrap(),
        },
        dpns_contract_owner: RequiredIdentityPublicKeysSet {
            master: ECDSA_SECP256K1
                .random_public_key_data(&mut rng, platform_version)
                .unwrap(),
            high: ECDSA_SECP256K1
                .random_public_key_data(&mut rng, platform_version)
                .unwrap(),
        },
        withdrawals_contract_owner: RequiredIdentityPublicKeysSet {
            master: ECDSA_SECP256K1
                .random_public_key_data(&mut rng, platform_version)
                .unwrap(),
            high: ECDSA_SECP256K1
                .random_public_key_data(&mut rng, platform_version)
                .unwrap(),
        },
        dashpay_contract_owner: RequiredIdentityPublicKeysSet {
            master: ECDSA_SECP256K1
                .random_public_key_data(&mut rng, platform_version)
                .unwrap(),
            high: ECDSA_SECP256K1
                .random_public_key_data(&mut rng, platform_version)
                .unwrap(),
        },
    }
}
