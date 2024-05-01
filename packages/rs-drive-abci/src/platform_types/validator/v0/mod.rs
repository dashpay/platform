use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use dashcore_rpc::dashcore::hashes::Hash;
use dashcore_rpc::dashcore::{ProTxHash, PubkeyHash};
use dashcore_rpc::dashcore_rpc_json::{DMNState, MasternodeListItem};
use std::fmt::{Debug, Formatter};

use crate::platform_types::platform_state::PlatformState;
use dpp::bls_signatures::PublicKey as BlsPublicKey;
use serde::{Deserialize, Serialize};

/// A validator in the context of a quorum
#[derive(Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct ValidatorV0 {
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

impl Debug for ValidatorV0 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatorV0")
            .field("pro_tx_hash", &self.pro_tx_hash.to_string())
            .field("public_key", &self.public_key)
            .field("node_ip", &self.node_ip)
            .field("node_id", &self.node_id)
            .field("core_port", &self.core_port)
            .field("platform_http_port", &self.platform_http_port)
            .field("platform_p2p_port", &self.platform_p2p_port)
            .field("is_banned", &self.is_banned)
            .finish()
    }
}

impl ValidatorV0 {
    /// Makes a validator if the masternode is in the list and is valid
    pub fn new_validator_if_masternode_in_state(
        pro_tx_hash: ProTxHash,
        public_key: Option<BlsPublicKey>,
        state: &PlatformState,
    ) -> Option<Self> {
        let MasternodeListItem { state, .. } = state.hpmn_masternode_list().get(&pro_tx_hash)?;

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
        Some(ValidatorV0 {
            pro_tx_hash,
            public_key,
            node_ip: service.ip().to_string(),
            node_id: PubkeyHash::from_byte_array(platform_node_id),
            core_port: service.port(),
            platform_http_port: *platform_http_port as u16,
            platform_p2p_port: *platform_p2p_port as u16,
            is_banned: pose_ban_height.is_some(),
        })
    }
}

/// Traits to get properties of a validator.
pub trait ValidatorV0Getters {
    /// Returns the proTxHash of the validator.
    fn pro_tx_hash(&self) -> &ProTxHash;
    /// Returns the public key share of this validator for this quorum.
    fn public_key(&self) -> &Option<BlsPublicKey>;
    /// Returns the node address of the validator.
    fn node_ip(&self) -> &String;
    /// Returns the node id of the validator.
    fn node_id(&self) -> &PubkeyHash;
    /// Returns the core port of the validator.
    fn core_port(&self) -> u16;
    /// Returns the Http port of the validator.
    fn platform_http_port(&self) -> u16;
    /// Returns the Tenderdash port of the validator.
    fn platform_p2p_port(&self) -> u16;
    /// Returns the status of the validator whether it's banned or not.
    fn is_banned(&self) -> bool;
}

/// Traits to set properties of a validator.
pub trait ValidatorV0Setters {
    /// Sets the proTxHash of the validator.
    fn set_pro_tx_hash(&mut self, pro_tx_hash: ProTxHash);
    /// Sets the public key share of this validator for this quorum.
    fn set_public_key(&mut self, public_key: Option<BlsPublicKey>);
    /// Sets the node address of the validator.
    fn set_node_ip(&mut self, node_ip: String);
    /// Sets the node id of the validator.
    fn set_node_id(&mut self, node_id: PubkeyHash);
    /// Sets the core port of the validator.
    fn set_core_port(&mut self, core_port: u16);
    /// Sets the Http port of the validator.
    fn set_platform_http_port(&mut self, platform_http_port: u16);
    /// Sets the Tenderdash port of the validator.
    fn set_platform_p2p_port(&mut self, platform_p2p_port: u16);
    /// Sets the status of the validator whether it's banned or not.
    fn set_is_banned(&mut self, is_banned: bool);
}

impl ValidatorV0Getters for ValidatorV0 {
    fn pro_tx_hash(&self) -> &ProTxHash {
        &self.pro_tx_hash
    }

    fn public_key(&self) -> &Option<BlsPublicKey> {
        &self.public_key
    }

    fn node_ip(&self) -> &String {
        &self.node_ip
    }

    fn node_id(&self) -> &PubkeyHash {
        &self.node_id
    }

    fn core_port(&self) -> u16 {
        self.core_port
    }

    fn platform_http_port(&self) -> u16 {
        self.platform_http_port
    }

    fn platform_p2p_port(&self) -> u16 {
        self.platform_p2p_port
    }

    fn is_banned(&self) -> bool {
        self.is_banned
    }
}

impl ValidatorV0Setters for ValidatorV0 {
    fn set_pro_tx_hash(&mut self, pro_tx_hash: ProTxHash) {
        self.pro_tx_hash = pro_tx_hash;
    }

    fn set_public_key(&mut self, public_key: Option<BlsPublicKey>) {
        self.public_key = public_key;
    }

    fn set_node_ip(&mut self, node_ip: String) {
        self.node_ip = node_ip;
    }

    fn set_node_id(&mut self, node_id: PubkeyHash) {
        self.node_id = node_id;
    }

    fn set_core_port(&mut self, core_port: u16) {
        self.core_port = core_port;
    }

    fn set_platform_http_port(&mut self, platform_http_port: u16) {
        self.platform_http_port = platform_http_port;
    }

    fn set_platform_p2p_port(&mut self, platform_p2p_port: u16) {
        self.platform_p2p_port = platform_p2p_port;
    }

    fn set_is_banned(&mut self, is_banned: bool) {
        self.is_banned = is_banned;
    }
}
