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

use drive::dpp::identity::TimestampMillis;
use drive::fee::epoch::CreditsPerEpoch;
use drive::fee::result::FeeResult;
use serde::{Deserialize, Serialize};
use tenderdash_abci::proto::abci::RequestInitChain;
use tenderdash_abci::proto::serializers::timestamp::ToMilis;

use super::AbciError;

/// A struct for handling chain initialization requests
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestInitChainCleanedParams {
    /// The genesis time in milliseconds
    pub genesis_time: TimestampMillis,

    /// Initial core chain lock height.
    pub initial_core_height: Option<u32>,
}

impl TryFrom<RequestInitChain> for RequestInitChainCleanedParams {
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
