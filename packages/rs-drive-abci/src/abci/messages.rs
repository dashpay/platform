// MIT LICENSE
//
// Copyright (c) 2021 Dash Core Group
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.
//

//! Tenderdash ABCI Messages.
//!
//! This module defines the structs used for handling ABCI messages
//! as well as defining and implementing the trait for serializing/deserializing them.
//!

use crate::error::serialization::SerializationError;
use crate::error::Error;
use crate::execution::fee_pools::epoch::EpochInfo;
use crate::execution::fee_pools::fee_distribution::FeesInPools;
use crate::execution::fee_pools::process_block_fees::ProcessedBlockFeesOutcome;
use drive::dpp::identity::TimestampMillis;
use drive::dpp::util::deserializer::ProtocolVersion;
use drive::fee::epoch::CreditsPerEpoch;
use drive::fee::result::FeeResult;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tenderdash_abci::proto::abci::RequestInitChain;
use tenderdash_abci::proto::google::protobuf::Timestamp;
use tenderdash_abci::proto::serializers::timestamp::ToMilis;

use super::AbciError;

/// A struct for handling chain initialization requests
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitChainRequest {
    /// The genesis time in milliseconds
    pub genesis_time: TimestampMillis,

    /// Initial core chain lock height.
    pub initial_core_height: Option<u32>,
}

impl TryFrom<RequestInitChain> for InitChainRequest {
    type Error = AbciError;
    fn try_from(request: RequestInitChain) -> Result<Self, Self::Error> {
        let genesis_time = request
            .time
            .ok_or(AbciError::BadRequest(
                "genesis time is required in init chain".to_string(),
            ))?
            .to_milis() as TimestampMillis;
        let initial_core_height = match request.initial_core_height {
            0 => None,
            h => Some(h),
        };

        Ok(Self {
            genesis_time,
            initial_core_height,
        })
    }
}

impl From<InitChainRequest> for RequestInitChain {
    fn from(value: InitChainRequest) -> Self {
        let InitChainRequest {
            genesis_time: genesis_time_ms,
            initial_core_height,
        } = value;
        RequestInitChain {
            time: Some(Timestamp {
                seconds: (genesis_time_ms / 1000) as i64,
                nanos: ((genesis_time_ms % 1000) * 1000) as i32,
            }),
            chain_id: "".to_string(),
            consensus_params: None,
            validator_set: None,
            app_state_bytes: vec![],
            initial_height: 0,
            initial_core_height: initial_core_height.unwrap_or_default(),
        }
    }
}

/// System identity public keys
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SystemIdentityPublicKeys {
    /// Required public key set for masternode reward shares contract owner identity
    pub masternode_reward_shares_contract_owner: RequiredIdentityPublicKeysSet,
    /// Required public key set for feature flags contract owner identity
    pub feature_flags_contract_owner: RequiredIdentityPublicKeysSet,
    /// Required public key set for dpns contract owner identity
    pub dpns_contract_owner: RequiredIdentityPublicKeysSet,
    /// Required public key set for withdrawals contract owner identity
    pub withdrawals_contract_owner: RequiredIdentityPublicKeysSet,
    /// Required public key set for dashpay contract owner identity
    pub dashpay_contract_owner: RequiredIdentityPublicKeysSet,
}

// impl Default for SystemIdentityPublicKeys {}

/// Required public key set for an identity
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct RequiredIdentityPublicKeysSet {
    /// Authentication key with master security level
    pub master: Vec<u8>,
    /// Authentication key with high security level
    pub high: Vec<u8>,
}

/// A struct for handling chain initialization responses
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitChainResponse {}

/// A struct for handling block begin requests
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockBeginRequest {
    /// Block height
    pub block_height: u64,
    /// Block time in ms
    pub block_time_ms: u64,
    /// Previous block time in ms
    pub previous_block_time_ms: Option<u64>,
    /// The block proposer's proTxHash
    pub proposer_pro_tx_hash: [u8; 32],
    /// The block proposer's proposed version
    pub proposed_app_version: ProtocolVersion,
    /// Validator set quorum hash
    pub validator_set_quorum_hash: [u8; 32],
    /// Last synced core height
    pub last_synced_core_height: u32,
    /// Core chain locked height
    pub core_chain_locked_height: u32,
    /// The total number of HPMNs in the system
    pub total_hpmns: u32,
}

/// A struct for handling block begin responses
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlockBeginResponse {
    /// Fee epoch info
    pub epoch_info: EpochInfo,
    /// List of unsigned withdrawal transaction bytes
    pub unsigned_withdrawal_transactions: Vec<Vec<u8>>,
}

/// A struct for handling block end requests
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockEndRequest {
    /// The fees for the block
    /// Avoid of serialization to optimize transfer through Node.JS binding
    pub fees: BlockFees,
}

/// Aggregated fees after block execution
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlockFees {
    /// Processing fee
    pub processing_fee: u64,
    /// Storage fee
    pub storage_fee: u64,
    /// Fee refunds per epoch
    pub refunds_per_epoch: CreditsPerEpoch,
}

impl BlockFees {
    /// Create block fee result from fees
    pub fn from_fees(storage_fee: u64, processing_fee: u64) -> Self {
        Self {
            storage_fee,
            processing_fee,
            ..Default::default()
        }
    }
}

impl From<FeeResult> for BlockFees {
    fn from(value: FeeResult) -> Self {
        Self {
            storage_fee: value.storage_fee,
            processing_fee: value.processing_fee,
            refunds_per_epoch: value.fee_refunds.sum_per_epoch(),
        }
    }
}

/// A struct for handling block end responses
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlockEndResponse {
    /// Number of proposers to be paid
    pub proposers_paid_count: Option<u16>,
    /// Index of the last epoch that marked as paid
    pub paid_epoch_index: Option<u16>,
    /// A number of epochs which had refunded
    pub refunded_epochs_count: Option<u16>,
    /// Amount of fees in the storage and processing fee distribution pools
    pub fees_in_pools: FeesInPools,
    /// Next protocol app version
    pub changed_protocol_app_version: Option<ProtocolVersion>,
}

impl BlockEndResponse {
    /// Retrieves fee info for the block to be implemented in the BlockEndResponse
    pub(crate) fn from_outcomes(
        process_block_fees_result: &ProcessedBlockFeesOutcome,
        changed_protocol_app_version: Option<ProtocolVersion>,
    ) -> Self {
        let (proposers_paid_count, paid_epoch_index) = process_block_fees_result
            .payouts
            .as_ref()
            .map_or((None, None), |proposer_payouts| {
                (
                    Some(proposer_payouts.proposers_paid_count),
                    Some(proposer_payouts.paid_epoch_index),
                )
            });

        Self {
            proposers_paid_count,
            paid_epoch_index,
            refunded_epochs_count: process_block_fees_result.refunded_epochs_count,
            fees_in_pools: process_block_fees_result.fees_in_pools,
            changed_protocol_app_version,
        }
    }
}

/// A struct for handling finalize block responses
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterFinalizeBlockRequest {
    /// List of updated contract ids
    pub updated_data_contract_ids: Vec<[u8; 32]>,
}

/// A struct for handling finalize block responses
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterFinalizeBlockResponse {}

impl<'a> Serializable<'a> for InitChainRequest {}
impl<'a> Serializable<'a> for InitChainResponse {}
impl<'a> Serializable<'a> for BlockBeginRequest {}
impl<'a> Serializable<'a> for BlockBeginResponse {}
impl<'a> Serializable<'a> for BlockEndRequest {}
impl<'a> Serializable<'a> for BlockEndResponse {}
impl<'a> Serializable<'a> for AfterFinalizeBlockRequest {}
impl<'a> Serializable<'a> for AfterFinalizeBlockResponse {}

/// A trait for serializing or deserializing ABCI messages
pub trait Serializable<'a>: Serialize + DeserializeOwned {
    /// Serialize ABCI message
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = vec![];

        ciborium::ser::into_writer(&self, &mut bytes).map_err(|e| {
            let message = format!("can't deserialize ABCI message: {}", e);

            Error::Serialization(SerializationError::CorruptedSerialization(message))
        })?;

        Ok(bytes)
    }

    /// Deserialize ABCI message
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        ciborium::de::from_reader(bytes).map_err(|e| {
            let message = format!("can't deserialize ABCI message: {}", e);

            Error::Serialization(SerializationError::CorruptedDeserialization(message))
        })
    }
}
