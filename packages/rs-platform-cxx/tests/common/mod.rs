// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! Fixture access shared by the bridge's integration tests.
//!
//! Proofs, quorum signature and quorum key come from drive-proof-verifier's
//! proof-vector corpus (`../rs-drive-proof-verifier/tests/vectors`), so the
//! bridge verifies exactly the bytes the upstream verifier is pinned against.
//! Every fixture proof commits to the same root hash and the fixture quorum
//! signed that root, so positive cases run grovedb replay plus the Tenderdash
//! BLS check end to end.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Once;

use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use serde::Deserialize;

/// The Platform LLMQ type the corpus quorum belongs to.
pub const QUORUM_TYPE: u32 = 106;

#[derive(Deserialize)]
pub struct Manifest {
    pub request: serde_json::Value,
    pub expected: serde_json::Value,
    pub block: BlockMeta,
    pub proof_meta: ProofMeta,
    #[serde(default)]
    pub expected_root_hash_hex: Option<String>,
}

#[derive(Deserialize)]
pub struct BlockMeta {
    pub height: u64,
    pub core_chain_locked_height: u32,
    pub epoch: u16,
    pub time_ms: u64,
    pub protocol_version: u32,
    pub chain_id: String,
}

#[derive(Deserialize)]
pub struct ProofMeta {
    pub round: u32,
    pub quorum_type: u32,
    pub quorum_hash_hex: String,
    pub block_id_hash_hex: String,
}

pub struct Case {
    pub name: String,
    pub manifest: Manifest,
    pub grovedb_proof: Vec<u8>,
    pub signature: Vec<u8>,
    pub quorum_pubkey: [u8; 48],
}

pub fn hex_vec(s: &str) -> Vec<u8> {
    hex::decode(s).expect("corpus hex")
}

pub fn hex32(s: &str) -> [u8; 32] {
    hex_vec(s).try_into().expect("32 bytes")
}

pub fn load_case(name: &str) -> Case {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../rs-drive-proof-verifier/tests/vectors")
        .join(name);
    let read = |file: &str| {
        std::fs::read_to_string(dir.join(file))
            .unwrap_or_else(|e| panic!("read corpus file {name}/{file}: {e}"))
    };
    let manifest: Manifest =
        serde_json::from_str(&read("manifest.json")).expect("parse corpus manifest");
    Case {
        name: name.to_string(),
        grovedb_proof: hex_vec(read("proof.hex").trim()),
        signature: hex_vec(read("signature.hex").trim()),
        quorum_pubkey: hex_vec(read("quorum_pubkey.hex").trim())
            .try_into()
            .expect("48-byte quorum key"),
        manifest,
    }
}

impl Case {
    pub fn proof(&self) -> Proof {
        Proof {
            grovedb_proof: self.grovedb_proof.clone(),
            quorum_hash: hex_vec(&self.manifest.proof_meta.quorum_hash_hex),
            signature: self.signature.clone(),
            round: self.manifest.proof_meta.round,
            block_id_hash: hex_vec(&self.manifest.proof_meta.block_id_hash_hex),
            quorum_type: self.manifest.proof_meta.quorum_type,
        }
    }

    pub fn metadata(&self) -> ResponseMetadata {
        ResponseMetadata {
            height: self.manifest.block.height,
            core_chain_locked_height: self.manifest.block.core_chain_locked_height,
            epoch: u32::from(self.manifest.block.epoch),
            time_ms: self.manifest.block.time_ms,
            protocol_version: self.manifest.block.protocol_version,
            chain_id: self.manifest.block.chain_id.clone(),
        }
    }

    pub fn identity_id(&self) -> Vec<u8> {
        hex_vec(
            self.manifest.request["identity_id"]
                .as_str()
                .expect("identity_id"),
        )
    }

    pub fn check_meta(&self, meta: &dash_platform_cxx::types::Meta) {
        assert_eq!(meta.height, self.manifest.block.height);
        assert_eq!(
            meta.core_chain_locked_height,
            self.manifest.block.core_chain_locked_height
        );
        assert_eq!(meta.time_ms, self.manifest.block.time_ms);
        assert_eq!(meta.protocol_version, self.manifest.block.protocol_version);
        assert_eq!(meta.chain_id, self.manifest.block.chain_id);
    }
}

/// Installs the fixture context exactly once per test binary: the corpus
/// network, quorum type, protocol version and the corpus quorum key. Tests
/// that need a failing key lookup tamper the response instead of mutating
/// this shared store.
pub fn setup() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let case = load_case("quorum-sig-valid");
        dash_platform_cxx::provider::set_context(
            "test",
            case.manifest.proof_meta.quorum_type,
            case.manifest.block.protocol_version,
            0,
        )
        .expect("set_context");
        dash_platform_cxx::provider::update_quorum_keys(vec![
            dash_platform_cxx::provider::QuorumKey {
                quorum_hash: hex32(&case.manifest.proof_meta.quorum_hash_hex),
                public_key: case.quorum_pubkey,
            },
        ])
        .expect("update_quorum_keys");
    });
}
