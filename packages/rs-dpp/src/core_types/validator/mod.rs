use crate::bls_signatures::{Bls12381G2Impl, PublicKey as BlsPublicKey};
use crate::core_types::validator::v0::{ValidatorV0, ValidatorV0Getters, ValidatorV0Setters};
use dashcore::{ProTxHash, PubkeyHash};
#[cfg(feature = "core-types-serde-conversion")]
use serde::{Deserialize, Serialize};

/// Version 0
pub mod v0;

/// A validator in the context of a quorum
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "core-types-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub enum Validator {
    /// Version 0
    V0(ValidatorV0),
}

impl ValidatorV0Getters for Validator {
    fn pro_tx_hash(&self) -> &ProTxHash {
        match self {
            Validator::V0(v0) => v0.pro_tx_hash(),
        }
    }

    fn public_key(&self) -> &Option<BlsPublicKey<Bls12381G2Impl>> {
        match self {
            Validator::V0(v0) => v0.public_key(),
        }
    }

    fn node_ip(&self) -> &String {
        match self {
            Validator::V0(v0) => v0.node_ip(),
        }
    }

    fn node_id(&self) -> &PubkeyHash {
        match self {
            Validator::V0(v0) => v0.node_id(),
        }
    }

    fn core_port(&self) -> u16 {
        match self {
            Validator::V0(v0) => v0.core_port(),
        }
    }

    fn platform_http_port(&self) -> u16 {
        match self {
            Validator::V0(v0) => v0.platform_http_port(),
        }
    }

    fn platform_p2p_port(&self) -> u16 {
        match self {
            Validator::V0(v0) => v0.platform_p2p_port(),
        }
    }

    fn is_banned(&self) -> bool {
        match self {
            Validator::V0(v0) => v0.is_banned(),
        }
    }
}

impl ValidatorV0Setters for Validator {
    fn set_pro_tx_hash(&mut self, pro_tx_hash: ProTxHash) {
        match self {
            Validator::V0(v0) => v0.set_pro_tx_hash(pro_tx_hash),
        }
    }

    fn set_public_key(&mut self, public_key: Option<BlsPublicKey<Bls12381G2Impl>>) {
        match self {
            Validator::V0(v0) => v0.set_public_key(public_key),
        }
    }

    fn set_node_ip(&mut self, node_ip: String) {
        match self {
            Validator::V0(v0) => v0.set_node_ip(node_ip),
        }
    }

    fn set_node_id(&mut self, node_id: PubkeyHash) {
        match self {
            Validator::V0(v0) => v0.set_node_id(node_id),
        }
    }

    fn set_core_port(&mut self, core_port: u16) {
        match self {
            Validator::V0(v0) => v0.set_core_port(core_port),
        }
    }

    fn set_platform_http_port(&mut self, platform_http_port: u16) {
        match self {
            Validator::V0(v0) => v0.set_platform_http_port(platform_http_port),
        }
    }

    fn set_platform_p2p_port(&mut self, platform_p2p_port: u16) {
        match self {
            Validator::V0(v0) => v0.set_platform_p2p_port(platform_p2p_port),
        }
    }

    fn set_is_banned(&mut self, is_banned: bool) {
        match self {
            Validator::V0(v0) => v0.set_is_banned(is_banned),
        }
    }
}
