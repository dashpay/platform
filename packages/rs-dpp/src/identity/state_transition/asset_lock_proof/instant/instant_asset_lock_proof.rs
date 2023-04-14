use std::convert::{TryFrom, TryInto};

use dashcore::consensus::{Decodable, Encodable};
use dashcore::{InstantLock, Transaction, TxIn, TxOut};
use platform_value::{BinaryData, Value};

use serde::de::Error as DeError;
use serde::ser::Error as SerError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::prelude::Identifier;
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::hash::hash_to_vec;
use crate::util::vec::vec_to_array;
use crate::{NonConsensusError, ProtocolError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstantAssetLockProof {
    asset_lock_type: u8,
    instant_lock: InstantLock,
    transaction: Transaction,
    output_index: u32,
}

impl Serialize for InstantAssetLockProof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let raw = RawInstantLock::try_from(self).map_err(|e| S::Error::custom(e.to_string()))?;

        raw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InstantAssetLockProof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawInstantLock::deserialize(deserializer)?;
        raw.try_into()
            .map_err(|e: ProtocolError| D::Error::custom(e.to_string()))
    }
}

impl TryFrom<Value> for InstantAssetLockProof {
    type Error = platform_value::Error;
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        platform_value::from_value(value)
    }
}

impl Default for InstantAssetLockProof {
    fn default() -> Self {
        Self {
            // TODO: change to a const
            asset_lock_type: 0,
            instant_lock: InstantLock::default(),
            transaction: Transaction {
                version: 0,
                lock_time: 0,
                input: vec![TxIn::default()],
                output: vec![TxOut::default()],
                special_transaction_payload: None,
            },
            output_index: 0,
        }
    }
}

impl InstantAssetLockProof {
    pub fn new(instant_lock: InstantLock, transaction: Transaction, output_index: u32) -> Self {
        Self {
            // TODO: change the type to a const
            instant_lock,
            transaction,
            output_index,
            asset_lock_type: 0,
        }
    }

    pub fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }
    pub fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        self.to_object()
    }

    pub fn asset_lock_type(&self) -> u8 {
        self.asset_lock_type
    }

    pub fn instant_lock(&self) -> &InstantLock {
        &self.instant_lock
    }

    pub fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    pub fn output_index(&self) -> usize {
        self.output_index as usize
    }

    pub fn out_point(&self) -> Option<[u8; 36]> {
        let out_point_buffer = self.transaction.out_point_buffer(self.output_index());

        out_point_buffer.map(|mut buffer| {
            let (tx_id, _) = buffer.split_at_mut(32);
            tx_id.reverse();
            buffer
        })
    }

    pub fn output(&self) -> Option<&TxOut> {
        self.transaction.output.get(self.output_index())
    }

    pub fn create_identifier(&self) -> Result<Identifier, NonConsensusError> {
        let out_point = self.out_point().ok_or_else(|| {
            NonConsensusError::IdentifierCreateError(String::from("No output at a given index"))
        })?;

        let buffer = hash_to_vec(out_point);
        Ok(Identifier::new(vec_to_array(&buffer)?))
    }

    pub fn to_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut map = CborCanonicalMap::new();
        let mut is_lock_buffer = Vec::<u8>::new();
        let mut transaction_buffer = Vec::<u8>::new();
        self.instant_lock
            .consensus_encode(&mut is_lock_buffer)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
        self.transaction
            .consensus_encode(&mut transaction_buffer)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        map.insert("type", self.asset_lock_type);
        map.insert("outputIndex", self.output_index);
        map.insert("transaction", transaction_buffer);
        map.insert("instantLock", is_lock_buffer);

        map.to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
/// "Raw" instant lock for serialization
pub struct RawInstantLock {
    #[serde(rename = "type")]
    lock_type: u8,
    instant_lock: BinaryData,
    transaction: BinaryData,
    output_index: u32,
}

impl TryFrom<RawInstantLock> for InstantAssetLockProof {
    type Error = ProtocolError;

    fn try_from(raw_instant_lock: RawInstantLock) -> Result<Self, Self::Error> {
        let transaction = Transaction::consensus_decode(raw_instant_lock.transaction.as_slice())
            .map_err(|e| ProtocolError::DecodingError(e.to_string()))?;
        let instant_lock = InstantLock::consensus_decode(raw_instant_lock.instant_lock.as_slice())
            .map_err(|e| ProtocolError::DecodingError(e.to_string()))?;

        Ok(Self {
            asset_lock_type: raw_instant_lock.lock_type,
            transaction,
            instant_lock,
            output_index: raw_instant_lock.output_index,
        })
    }
}

impl TryFrom<&InstantAssetLockProof> for RawInstantLock {
    type Error = ProtocolError;

    fn try_from(instant_asset_lock_proof: &InstantAssetLockProof) -> Result<Self, Self::Error> {
        let mut is_lock_buffer = Vec::<u8>::new();
        let mut transaction_buffer = Vec::<u8>::new();
        instant_asset_lock_proof
            .instant_lock
            .consensus_encode(&mut is_lock_buffer)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
        instant_asset_lock_proof
            .transaction
            .consensus_encode(&mut transaction_buffer)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        Ok(Self {
            lock_type: instant_asset_lock_proof.asset_lock_type,
            instant_lock: BinaryData::new(is_lock_buffer),
            transaction: BinaryData::new(transaction_buffer),
            output_index: instant_asset_lock_proof.output_index,
        })
    }
}
