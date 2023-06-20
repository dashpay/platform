
use crate::error::execution::ExecutionError;
use crate::error::Error;
use crate::platform::state::PlatformState;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::{ProTxHash, PubkeyHash, QuorumHash};
use dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem};
use dashcore_rpc::json::QuorumInfoResult;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tenderdash_abci::proto::abci::ValidatorSetUpdate;
use tenderdash_abci::proto::crypto::public_key::Sum::Bls12381;
use tenderdash_abci::proto::{abci, crypto};

/// A validator in the context of a quorum
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Validator {
    /// The proTxHash
    pub pro_tx_hash: ProTxHash,
    /// The public key share of this validator for this quorum
    pub public_key: Option<BlsPublicKey>,
    /// The node address
    pub node_ip: String,
    /// The node id
    pub node_id: PubkeyHash,
    /// Core port
    pub core_port: u16,
    /// Http port
    pub platform_http_port: u16,
    /// Tenderdash port
    pub platform_p2p_port: u16,
    /// Is the validator banned
    pub is_banned: bool,
}

impl Validator {
    /// Makes a validator if the masternode is in the list and is valid
    pub fn new_validator_if_masternode_in_state(
        pro_tx_hash: ProTxHash,
        public_key: Option<BlsPublicKey>,
        state: &PlatformState,
    ) -> Option<Self> {
        let MasternodeListItem { state, .. } = state.hpmn_masternode_list.get(&pro_tx_hash)?;

        let DMNState {
            service,
            platform_node_id,
            pose_ban_height,
            platform_p2p_port,
            platform_http_port,
            ..
        } = state;
        let Some(platform_http_port) = platform_http_port else {
            return None;
        };
        let Some(platform_p2p_port) = platform_p2p_port else {
            return None;
        };
        let platform_node_id = (*platform_node_id)?;
        Some(Validator {
            pro_tx_hash,
            public_key,
            node_ip: service.ip().to_string(),
            node_id: PubkeyHash::from_inner(platform_node_id),
            core_port: service.port(),
            platform_http_port: *platform_http_port as u16,
            platform_p2p_port: *platform_p2p_port as u16,
            is_banned: pose_ban_height.is_some(),
        })
    }
}