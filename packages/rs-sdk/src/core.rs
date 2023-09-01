//! Core RPC client
//!
//! TODO: This is a temporary implementation, effective until we integrate SPV
//! into rs-sdk.

use dashcore_rpc::{
    dashcore::{hashes::Hash, QuorumHash},
    dashcore_rpc_json::QuorumType,
};
use drive_abci::rpc::core::{CoreRPCLike, DefaultCoreRPC};
use drive_proof_verifier::QuorumInfoProvider;
use std::sync::RwLock;

use crate::error::Error;

pub struct CoreClient {
    // TODO implement async core client
    core: RwLock<Box<dyn CoreRPCLike + Send + Sync>>,
}

impl CoreClient {
    pub fn new(
        server_address: &str,
        core_port: u16,
        core_user: &str,
        core_password: &str,
    ) -> Result<Self, Error> {
        let core_addr = format!("http://{}:{}", server_address, core_port);
        let core =
            DefaultCoreRPC::open(&core_addr, core_user.to_string(), core_password.to_string())?;

        Ok(Self {
            core: RwLock::new(Box::new(core)),
        })
    }
}

impl QuorumInfoProvider for CoreClient {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], drive_proof_verifier::Error> {
        let quorum_hash = QuorumHash::from_slice(&quorum_hash).map_err(|e| {
            drive_proof_verifier::Error::InvalidQuorum {
                error: e.to_string(),
            }
        })?;

        let core = self.core.write().expect("Core lock poisoned");
        let quorum_info = core
            .get_quorum_info(QuorumType::from(quorum_type), &quorum_hash, None)
            .map_err(
                |e: dashcore_rpc::Error| drive_proof_verifier::Error::InvalidQuorum {
                    error: e.to_string(),
                },
            )?;
        let key = quorum_info.quorum_public_key;
        let pubkey = <Vec<u8> as TryInto<[u8; 48]>>::try_into(key).map_err(|e| {
            drive_proof_verifier::Error::InvalidQuorum {
                error: "quorum public key is not 48 bytes long".to_string(),
            }
        })?;
        Ok(pubkey)
    }
}
