//! The authenticated state a proof starts from and ends at (Core's `ProofState`).
use crate::wire::{Hash, Reader, Result};
use crate::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Core network a [`State`] belongs to. Only these two networks have the
/// fixed ChainLock quorum types the proof format relies on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProofNetwork {
    /// Mainnet, ChainLocks signed by `LLMQ_400_60`.
    Mainnet,
    /// Testnet, ChainLocks signed by `LLMQ_50_60`.
    Testnet,
}

impl ProofNetwork {
    fn from_wire(network: u8) -> Result<Self> {
        match network {
            0 => Ok(Self::Mainnet),
            1 => Ok(Self::Testnet),
            _ => Err(Error("unsupported proof network")),
        }
    }

    fn to_wire(self) -> u8 {
        match self {
            Self::Mainnet => 0,
            Self::Testnet => 1,
        }
    }

    /// The LLMQ type that signs this network's ChainLocks.
    pub fn chainlock_quorum_type(self) -> u8 {
        match self {
            Self::Mainnet => 2,
            Self::Testnet => 1,
        }
    }

    /// First height of the era the format covers: v20 (Basic BLS, coinbase
    /// ChainLocks) is buried at this height. Core rejects earlier checkpoints.
    fn first_height(self) -> u32 {
        match self {
            Self::Mainnet => 1_987_776,
            Self::Testnet => 905_100,
        }
    }
}

/// A block's height and hash and the masternode and quorum Merkle roots its
/// coinbase commits to.
///
/// As a checkpoint, it is trust configuration the application fixes; a relay
/// never chooses it. The JSON form matches Core's `ProofState::ToJson`, so the
/// `target` that `getquorumproofchain` returns is a valid checkpoint file:
/// `network` is 0 for mainnet and 1 for testnet, and hashes are hex in RPC
/// display order. Unlike Core's `FromJson`, deserialization rejects unknown
/// fields and other networks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct State {
    /// Network the block belongs to.
    pub network: ProofNetwork,
    /// Block height.
    pub height: u32,
    /// Block hash, wire order.
    pub block_hash: Hash,
    /// The coinbase's `merkleRootMNList`, wire order.
    pub masternode_root: Hash,
    /// The coinbase's `merkleRootQuorums`, wire order.
    pub quorum_root: Hash,
}

impl State {
    /// Check that the state is one a proof can start from.
    pub fn validate(&self) -> Result<()> {
        if self.height < self.network.first_height() || self.height > i32::MAX as u32 {
            return Err(Error("height outside supported era"));
        }
        if self.block_hash == [0; 32] || self.quorum_root == [0; 32] {
            return Err(Error("empty authenticated state"));
        }
        Ok(())
    }

    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        out.push(self.network.to_wire());
        out.extend(self.height.to_le_bytes());
        out.extend(self.block_hash);
        out.extend(self.masternode_root);
        out.extend(self.quorum_root);
    }

    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self> {
        Ok(Self {
            network: ProofNetwork::from_wire(r.u8()?)?,
            height: r.u32()?,
            block_hash: r.array()?,
            masternode_root: r.array()?,
            quorum_root: r.array()?,
        })
    }
}

/// A hash as Core's RPC displays it: hex of the reversed wire bytes.
pub fn rpc_hex(hash: &Hash) -> String {
    let mut reversed = *hash;
    reversed.reverse();
    hex::encode(reversed)
}

/// Parse a hash in Core's RPC display order into wire order.
pub fn from_rpc_hex(text: &str) -> Result<Hash> {
    let mut hash: Hash = hex::decode(text)
        .ok()
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or(Error("hash must be 64 hex characters"))?;
    hash.reverse();
    Ok(hash)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StateJson {
    network: u8,
    height: u32,
    block_hash: String,
    masternode_root: String,
    quorum_root: String,
}

impl Serialize for State {
    fn serialize<S: Serializer>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error> {
        StateJson {
            network: self.network.to_wire(),
            height: self.height,
            block_hash: rpc_hex(&self.block_hash),
            masternode_root: rpc_hex(&self.masternode_root),
            quorum_root: rpc_hex(&self.quorum_root),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for State {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> core::result::Result<Self, D::Error> {
        let json = StateJson::deserialize(deserializer)?;
        let hash = |text: &str| from_rpc_hex(text).map_err(serde::de::Error::custom);
        Ok(Self {
            network: ProofNetwork::from_wire(json.network).map_err(serde::de::Error::custom)?,
            height: json.height,
            block_hash: hash(&json.block_hash)?,
            masternode_root: hash(&json.masternode_root)?,
            quorum_root: hash(&json.quorum_root)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(network: ProofNetwork, height: u32) -> State {
        State {
            network,
            height,
            block_hash: [1; 32],
            masternode_root: [2; 32],
            quorum_root: [3; 32],
        }
    }

    #[test]
    fn should_accept_checkpoints_from_the_v20_height_as_core_does() {
        assert!(state(ProofNetwork::Mainnet, 1_987_776).validate().is_ok());
        assert!(state(ProofNetwork::Mainnet, 1_987_775).validate().is_err());
        assert!(state(ProofNetwork::Testnet, 905_100).validate().is_ok());
        assert!(state(ProofNetwork::Testnet, 905_099).validate().is_err());
        assert!(state(ProofNetwork::Testnet, i32::MAX as u32 + 1)
            .validate()
            .is_err());
        let mut empty = state(ProofNetwork::Testnet, 905_100);
        empty.quorum_root = [0; 32];
        assert!(empty.validate().is_err());
    }

    #[test]
    fn should_round_trip_json_in_rpc_byte_order() {
        let mut original = state(ProofNetwork::Testnet, 1_000_000);
        original.block_hash[0] = 0xab;
        let json = serde_json::to_value(&original).unwrap();
        assert_eq!(json["network"], 1);
        assert!(json["block_hash"].as_str().unwrap().ends_with("ab"));
        assert_eq!(serde_json::from_value::<State>(json).unwrap(), original);
        let unknown = serde_json::json!({
            "network": 2, "height": 1_000_000,
            "block_hash": "00".repeat(32), "masternode_root": "00".repeat(32),
            "quorum_root": "00".repeat(32),
        });
        assert!(serde_json::from_value::<State>(unknown).is_err());
    }
}
