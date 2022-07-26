use crate::error::serialization::SerializationError;
use crate::error::Error;
use crate::execution::fee_pools::epoch::EpochInfo;
use crate::execution::fee_pools::process_block_fees::ProcessedBlockFeesResult;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitChainRequest {}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitChainResponse {}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockBeginRequest {
    pub block_height: u64,
    pub block_time_ms: u64,
    pub previous_block_time_ms: Option<u64>,
    pub proposer_pro_tx_hash: [u8; 32],
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockBeginResponse {}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockEndRequest {
    pub fees: FeesAggregate,
}

pub type EpochRefund = (u16, u64);

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeesAggregate {
    pub processing_fees: u64,
    pub storage_fees: u64,
    // pub refunds_by_epoch: Vec<EpochRefund>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockEndResponse {
    pub current_epoch_index: u16,
    pub is_epoch_change: bool,
    pub proposers_paid_count: Option<u16>,
    pub paid_epoch_index: Option<u16>,
}

impl BlockEndResponse {
    pub(crate) fn from_epoch_info_and_process_block_fees_result(
        epoch_info: &EpochInfo,
        process_block_fees_result: &ProcessedBlockFeesResult,
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
            current_epoch_index: epoch_info.current_epoch_index,
            is_epoch_change: epoch_info.is_epoch_change,
            proposers_paid_count,
            paid_epoch_index,
        }
    }
}

impl<'a> Serializable<'a> for InitChainRequest {}
impl<'a> Serializable<'a> for InitChainResponse {}
impl<'a> Serializable<'a> for BlockBeginRequest {}
impl<'a> Serializable<'a> for BlockBeginResponse {}
impl<'a> Serializable<'a> for BlockEndRequest {}
impl<'a> Serializable<'a> for BlockEndResponse {}

pub trait Serializable<'a>: Serialize + Deserialize<'a> {
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {
        let mut bytes = vec![];

        ciborium::ser::into_writer(&self, &mut bytes).map_err(|_| {
            Error::Serialization(SerializationError::CorruptedSerialization(
                "can't serialize ABCI message",
            ))
        })?;

        Ok(bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        ciborium::de::from_reader(bytes).map_err(|_| {
            Error::Serialization(SerializationError::CorruptedDeserialization(
                "can't deserialize ABCI message",
            ))
        })
    }
}
