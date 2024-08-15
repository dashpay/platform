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

/// Dash network types.
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum NetworkType {
    /// Mock implementation; in practice, feaults to Devnet config for Mock mode. Errors when used in non-mock mode.
    Mock,
    /// Mainnet network, used for production.
    Mainnet,
    /// Testnet network, used for testing and development.
    Testnet,
    /// Devnet network, used local for development.
    Devnet,
    /// Custom network configuration.
    Custom(QuorumParams),
}

impl NetworkType {
    pub fn instant_lock_quorum_type(&self) -> QuorumType {
        self.to_quorum_params().instant_lock_quorum_type
    }

    pub(crate) fn to_quorum_params(&self) -> QuorumParams {
        match self {
            NetworkType::Mainnet => QuorumParams::new_mainnet(),
            NetworkType::Testnet => QuorumParams::new_testnet(),
            NetworkType::Devnet => QuorumParams::new_devnet(),
            NetworkType::Custom(config) => config.clone(),
            NetworkType::Mock => QuorumParams::new_mock(),
        }
    }
}

/// Configuration of Dash Core Quorums.
///
/// In most cases, you should use the [`new_mainnet`] or [`new_testnet`] functions to create a new instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuorumParams {
    pub instant_lock_quorum_type: QuorumType,
}

impl QuorumParams {
    pub fn new_mainnet() -> Self {
        QuorumParams {
            instant_lock_quorum_type: QuorumType::Llmq400_60,
        }
    }

    pub fn new_testnet() -> Self {
        QuorumParams {
            instant_lock_quorum_type: QuorumType::Llmq50_60,
        }
    }

    pub fn new_devnet() -> Self {
        QuorumParams {
            instant_lock_quorum_type: QuorumType::LlmqDevnet,
        }
    }

    pub fn new_mock() -> Self {
        Self::new_devnet()
    }
}
