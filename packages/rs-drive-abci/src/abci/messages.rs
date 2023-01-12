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
use crate::execution::fee_pools::process_block_fees::ProcessedBlockFeesOutcome;
use drive::fee::epoch::CreditsPerEpoch;
use drive::fee::result::FeeResult;
use serde::{Deserialize, Serialize};

/// A struct for handling chain initialization requests
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitChainRequest {}

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
    /// Validator set quorum hash
    pub validator_set_quorum_hash: [u8; 32],
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
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BlockFees {
    /// Processing fee
    pub processing_fee: u64,
    /// Storage fee
    pub storage_fee: u64,
    /// Fee refunds
    pub fee_refunds: CreditsPerEpoch,
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
    /// Get block fees from fee results
    pub fn from_fee_result(fee_result: FeeResult) -> Self {
        Self {
            storage_fee: fee_result.storage_fee,
            processing_fee: fee_result.processing_fee,
            fee_refunds: fee_result.fee_refunds.sum_per_epoch(),
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
}

impl BlockEndResponse {
    /// Retrieves fee info for the block to be implemented in the BlockEndResponse
    pub(crate) fn from_process_block_fees_outcome(
        process_block_fees_result: &ProcessedBlockFeesOutcome,
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
pub trait Serializable<'a>: Serialize + Deserialize<'a> {
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
