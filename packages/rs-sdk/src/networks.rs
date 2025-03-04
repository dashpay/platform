//! Configuration of dash networks (devnet, testnet, mainnet, etc.).
//!
//! See also:
//! * https://github.com/dashpay/dash/blob/develop/src/chainparams.cpp

/*
Mainnet:
    consensus.llmqTypeChainLocks = Consensus::LLMQType::LLMQ_400_60;
    consensus.llmqTypeDIP0024InstantSend = Consensus::LLMQType::LLMQ_60_75;
    consensus.llmqTypePlatform = Consensus::LLMQType::LLMQ_100_67;
    consensus.llmqTypeMnhf = Consensus::LLMQType::LLMQ_400_85;

Testnet:
    consensus.llmqTypeChainLocks = Consensus::LLMQType::LLMQ_50_60;
    consensus.llmqTypeDIP0024InstantSend = Consensus::LLMQType::LLMQ_60_75;
    consensus.llmqTypePlatform = Consensus::LLMQType::LLMQ_25_67;
    consensus.llmqTypeMnhf = Consensus::LLMQType::LLMQ_50_60;

Devnet:
    consensus.llmqTypeChainLocks = Consensus::LLMQType::LLMQ_DEVNET;
    consensus.llmqTypeDIP0024InstantSend = Consensus::LLMQType::LLMQ_DEVNET_DIP0024;
    consensus.llmqTypePlatform = Consensus::LLMQType::LLMQ_DEVNET_PLATFORM;
    consensus.llmqTypeMnhf = Consensus::LLMQType::LLMQ_DEVNET;

*/

use dashcore_rpc::json::QuorumType;
use dpp::dashcore::Network;

/// Official production network (mainnet)
pub const NETWORK_MAINNET: Network = Network::Dash;

/// Official testnet network
pub const NETWORK_TESTNET: Network = Network::Testnet;

/// Local development network, run in containers on a local machine for development purposess
pub const NETWORK_LOCAL: Network = Network::Regtest;

/// Configuration of the Dash Platform network.
///
/// In most cases, you should use [NETWORK_MAINNET], [NETWORK_TESTNET], or [NETWORK_LOCAL] constants.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NetworkSettings {
    Default(Network),
    Custom {
        core_network: Network,
        instant_lock_quorum_type: QuorumType,
    },
    /// Mock network for testing purposes
    #[cfg(feature = "mocks")]
    Mock,
}

impl NetworkSettings {
    pub fn core_network(&self) -> Network {
        match self {
            Self::Default(network) => *network,
            Self::Custom { core_network, .. } => *core_network,
            #[cfg(feature = "mocks")]
            Self::Mock => NETWORK_LOCAL,
        }
    }

    pub fn chain_locks_type(&self) -> QuorumType {
        let llmq_type = match self {
            Self::Default(network) => network.chain_locks_type() as u32,
            Self::Custom {
                instant_lock_quorum_type,
                ..
            } => *instant_lock_quorum_type as u32,
            #[cfg(feature = "mocks")]
            Self::Mock => self.core_network().chain_locks_type() as u32,
        };
        QuorumType::from(llmq_type)
    }
}

impl From<Network> for NetworkSettings {
    fn from(network: Network) -> Self {
        NetworkSettings::Default(network)
    }
}
