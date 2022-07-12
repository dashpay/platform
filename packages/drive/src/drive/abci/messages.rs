use crate::error;
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
    pub block_time: i64,
    pub previous_block_time: Option<i64>,
    pub proposer_pro_tx_hash: [u8; 32],
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockBeginResponse {}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockEndRequest {
    pub fees: Fees,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Fees {
    pub processing_fees: u64,
    pub storage_fees: i64,
    pub fee_multiplier: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockEndResponse {
    pub current_epoch_index: u16,
    pub is_epoch_change: bool,
    pub masternodes_paid_count: u16,
    pub paid_epoch_index: Option<u16>,
}

impl<'a> Serializable<'a> for InitChainRequest {}
impl<'a> Serializable<'a> for InitChainResponse {}
impl<'a> Serializable<'a> for BlockBeginRequest {}
impl<'a> Serializable<'a> for BlockBeginResponse {}
impl<'a> Serializable<'a> for BlockEndRequest {}
impl<'a> Serializable<'a> for BlockEndResponse {}

pub trait Serializable<'a>: Serialize + Deserialize<'a> {
    fn to_bytes(&self) -> Result<Vec<u8>, error::Error> {
        let mut bytes = vec![];

        ciborium::ser::into_writer(&self, &mut bytes).map_err(|_| {
            error::Error::Drive(error::drive::DriveError::CorruptedSerialization(
                "can't serialize ABCI message",
            ))
        })?;

        Ok(bytes)
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, error::Error> {
        ciborium::de::from_reader(bytes).map_err(|_| {
            error::Error::Drive(error::drive::DriveError::CorruptedSerialization(
                "can't deserialize ABCI message",
            ))
        })
    }
}
