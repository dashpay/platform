//! Configuration of dash networks (devnet, testnet, mainnet, etc.).

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

pub enum NetworkType {
    Mainnet,
    Testnet,
    Devnet,
    Mock,
    Custom(NetworkConfig),
}

impl NetworkType {
    pub fn instant_lock_quorum_type(&self) -> QuorumType {
        self.to_network_config().instant_lock
    }

    fn to_network_config(&self) -> NetworkConfig {
        match self {
            NetworkType::Mainnet => NetworkConfig::new_mainnet(),
            NetworkType::Testnet => NetworkConfig::new_testnet(),
            NetworkType::Devnet => NetworkConfig::new_devnet(),
            NetworkType::Mock => NetworkConfig::new_mock(),
            NetworkType::Custom(config) => config.clone(),
        }
    }
}

/// Configuration of Dash Core Quorums.
///
/// In most cases, you should use the [`new_mainnet`] or [`new_testnet`] functions to create a new instance.
#[derive(Clone, Debug)]
pub struct NetworkConfig {
    pub instant_lock: QuorumType,
}

impl NetworkConfig {
    pub fn new_mainnet() -> Self {
        NetworkConfig {
            instant_lock: QuorumType::Llmq400_60,
        }
    }

    pub fn new_testnet() -> Self {
        NetworkConfig {
            instant_lock: QuorumType::Llmq50_60,
        }
    }

    pub fn new_devnet() -> Self {
        NetworkConfig {
            instant_lock: QuorumType::LlmqDevnet,
        }
    }

    pub fn new_mock() -> Self {
        Self::new_devnet()
    }
}
