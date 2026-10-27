//! Loader for the proof-vector regression corpus in `tests/vectors/`.
//!
//! Each case directory carries a `manifest.json` (request parameters, block
//! metadata, expected outcome) plus the raw blobs (`proof.hex`,
//! `signature.hex`, `quorum_pubkey.hex`). The corpus was generated from the
//! Dash Core fixture set (`drive_query_vectors.json` /
//! `quorum_sig_vectors.json`, platform v4.0.0 state, protocol version 12);
//! every grovedb proof commits to the same root hash, which the fixture
//! quorum signed, so positive cases run the full grovedb + tenderdash
//! verification pipeline with real key material.

// Each integration-test binary compiles its own copy of this module and uses
// a different subset of it.
#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::TokenConfiguration;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::PlatformVersion;
use drive_proof_verifier::{ContextProvider, ContextProviderError};
use serde::Deserialize;

/// The network the fixture chain id (`dash-testnet-51`) belongs to.
pub const NETWORK: Network = Network::Testnet;

#[derive(Deserialize)]
pub struct Manifest {
    pub description: String,
    pub request: RequestSpec,
    pub block: BlockMeta,
    pub proof_meta: ProofMeta,
    pub expected: Expected,
    #[serde(default)]
    pub expected_root_hash_hex: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestSpec {
    IdentityBalance {
        identity_id: String,
    },
    IdentityNonce {
        identity_id: String,
    },
    IdentityContractNonce {
        identity_id: String,
        contract_id: String,
    },
    IdentityKeys {
        identity_id: String,
    },
    DocumentsDpnsExact {
        normalized_label: String,
        limit: u16,
    },
    DocumentsDpnsPrefix {
        normalized_prefix: String,
        limit: u16,
    },
    DocumentsDashpayProfile {
        owner_id: String,
    },
    DocumentsDashpayContacts {
        identity_id: String,
        to_identity: bool,
        limit: u16,
    },
    ContestedVoteState {
        contract_id: String,
        document_type_name: String,
        index_name: String,
        index_values: Vec<String>,
        count: u16,
    },
}

#[derive(Deserialize)]
pub struct BlockMeta {
    pub height: u64,
    pub core_chain_locked_height: u32,
    pub epoch: u32,
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

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expected {
    IdentityBalance {
        balance: u64,
    },
    IdentityNonce {
        nonce: u64,
    },
    IdentityContractNonce {
        nonce: u64,
    },
    IdentityKeys {
        serialized_keys: Vec<String>,
    },
    /// The fixture grovedb state stores placeholder payloads at document
    /// positions; the grovedb layer must verify and yield exactly these
    /// bytes, and decoding them as DPP documents must fail cleanly.
    DocumentsPlaceholder {
        serialized_documents: Vec<String>,
    },
    Contested {
        contenders: Vec<ExpectedContender>,
        abstain_votes: Option<u32>,
        lock_votes: Option<u32>,
        finished: bool,
        winner_identity_id: Option<String>,
    },
    ContestedAbsent,
    Error {
        class: ErrorClass,
    },
}

#[derive(Deserialize)]
pub struct ExpectedContender {
    pub identity_id: String,
    pub votes: Option<u32>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ErrorClass {
    InvalidSignature,
    ProofInvalid,
}

pub struct Case {
    pub name: String,
    pub manifest: Manifest,
    pub grovedb_proof: Vec<u8>,
    pub signature: Vec<u8>,
    pub quorum_pubkey: [u8; 48],
}

pub fn hex_vec(s: &str) -> Vec<u8> {
    hex::decode(s.trim()).expect("corpus hex blob must decode")
}

pub fn hex32(s: &str) -> [u8; 32] {
    hex_vec(s).try_into().expect("expected 32 bytes of hex")
}

pub fn identifier(s: &str) -> Identifier {
    Identifier::from_bytes(&hex_vec(s)).expect("corpus identifier must be 32 bytes")
}

pub fn load_case(name: &str) -> Case {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/vectors")
        .join(name);
    let read = |file: &str| {
        std::fs::read_to_string(dir.join(file))
            .unwrap_or_else(|e| panic!("read corpus file {name}/{file}: {e}"))
    };
    let manifest: Manifest =
        serde_json::from_str(&read("manifest.json")).expect("parse corpus manifest");
    Case {
        name: name.to_string(),
        grovedb_proof: hex_vec(&read("proof.hex")),
        signature: hex_vec(&read("signature.hex")),
        quorum_pubkey: hex_vec(&read("quorum_pubkey.hex"))
            .try_into()
            .expect("quorum public key must be 48 bytes"),
        manifest,
    }
}

impl Case {
    /// The tenderdash proof envelope for the DAPI response.
    pub fn grpc_proof(&self) -> Proof {
        Proof {
            grovedb_proof: self.grovedb_proof.clone(),
            quorum_hash: hex_vec(&self.manifest.proof_meta.quorum_hash_hex),
            signature: self.signature.clone(),
            round: self.manifest.proof_meta.round,
            block_id_hash: hex_vec(&self.manifest.proof_meta.block_id_hash_hex),
            quorum_type: self.manifest.proof_meta.quorum_type,
        }
    }

    /// The response metadata (block context the quorum signed over).
    pub fn metadata(&self) -> ResponseMetadata {
        ResponseMetadata {
            height: self.manifest.block.height,
            core_chain_locked_height: self.manifest.block.core_chain_locked_height,
            epoch: self.manifest.block.epoch,
            time_ms: self.manifest.block.time_ms,
            protocol_version: self.manifest.block.protocol_version,
            chain_id: self.manifest.block.chain_id.clone(),
        }
    }

    /// The platform version the vectors were generated with. The proofs are
    /// self-contained, so they must keep verifying under this version even
    /// as the crate's latest version moves on.
    pub fn platform_version(&self) -> &'static PlatformVersion {
        PlatformVersion::get(self.manifest.block.protocol_version)
            .expect("corpus protocol version must be known")
    }

    /// A [ContextProvider] serving this case's quorum public key and the
    /// system data contracts referenced by the fixture proofs.
    pub fn provider(&self) -> VectorContextProvider {
        VectorContextProvider {
            quorum_type: self.manifest.proof_meta.quorum_type,
            quorum_hash: hex32(&self.manifest.proof_meta.quorum_hash_hex),
            quorum_pubkey: self.quorum_pubkey,
        }
    }
}

/// [ContextProvider] backed by the per-case corpus quorum key material.
pub struct VectorContextProvider {
    quorum_type: u32,
    quorum_hash: [u8; 32],
    quorum_pubkey: [u8; 48],
}

impl ContextProvider for VectorContextProvider {
    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        for system_contract in [SystemDataContract::DPNS, SystemDataContract::Dashpay] {
            let contract = load_system_data_contract(system_contract, platform_version)
                .map_err(|e| ContextProviderError::DataContractFailure(e.to_string()))?;
            if contract.id() == *id {
                return Ok(Some(Arc::new(contract)));
            }
        }
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        _token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        Ok(None)
    }

    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        if quorum_type != self.quorum_type || quorum_hash != self.quorum_hash {
            return Err(ContextProviderError::InvalidQuorum(format!(
                "unexpected quorum requested: type {quorum_type}, hash {}",
                hex::encode(quorum_hash)
            )));
        }
        Ok(self.quorum_pubkey)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        Ok(1)
    }
}
