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
pub trait NetworkSettings {
    fn core_network(&self) -> Network;

    fn chain_locks_quorum_type(&self) -> QuorumType;
}

impl NetworkSettings for Network {
    fn core_network(&self) -> Network {
        *self
    }

    fn chain_locks_quorum_type(&self) -> QuorumType {
        let llmq_type: u8 = self.chain_locks_type().into();
        QuorumType::from(llmq_type as u32)
    }
}
