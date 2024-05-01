use std::convert::{TryFrom, TryInto};

use dashcore::consensus::{deserialize, Encodable};
use dashcore::transaction::special_transaction::TransactionPayload;
use dashcore::{InstantLock, OutPoint, Transaction, TxIn, TxOut};
use platform_value::{BinaryData, Value};

#[cfg(feature = "validation")]
use crate::identity::state_transition::asset_lock_proof::instant::methods;
#[cfg(feature = "validation")]
use platform_version::version::PlatformVersion;
use serde::de::Error as DeError;
use serde::ser::Error as SerError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::prelude::Identifier;
#[cfg(feature = "cbor")]
use crate::util::cbor_value::CborCanonicalMap;
use crate::util::hash::hash_double;
#[cfg(feature = "validation")]
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

/// Instant Asset Lock Proof is a part of Identity Create and Identity Topup
/// transitions. It is a proof that specific output of dash is locked in credits
/// pull and the transitions can mint credits and populate identity's balance.
/// To prove that the output is locked, an Instant Lock is provided.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstantAssetLockProof {
    /// The transaction's Instant Lock
    pub instant_lock: InstantLock,
    /// Asset Lock Special Transaction
    pub transaction: Transaction,
    /// Index of the output in the transaction payload
    pub output_index: u32,
}

impl Serialize for InstantAssetLockProof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let raw =
            RawInstantLockProof::try_from(self).map_err(|e| S::Error::custom(e.to_string()))?;

        raw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InstantAssetLockProof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawInstantLockProof::deserialize(deserializer)?;
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
            instant_lock,
            transaction,
            output_index,
        }
    }

    pub fn to_object(&self) -> Result<Value, ProtocolError> {
        platform_value::to_value(self).map_err(ProtocolError::ValueError)
    }

    pub fn to_cleaned_object(&self) -> Result<Value, ProtocolError> {
        self.to_object()
    }

    pub fn instant_lock(&self) -> &InstantLock {
        &self.instant_lock
    }

    pub fn transaction(&self) -> &Transaction {
        &self.transaction
    }

    pub fn output_index(&self) -> u32 {
        self.output_index
    }

    pub fn out_point(&self) -> Option<OutPoint> {
        self.output()
            .map(|_| OutPoint::new(self.transaction.txid(), self.output_index))
    }

    pub fn output(&self) -> Option<&TxOut> {
        if let Some(TransactionPayload::AssetLockPayloadType(ref payload)) =
            self.transaction.special_transaction_payload
        {
            payload.credit_outputs.get(self.output_index() as usize)
        } else {
            None
        }
    }

    pub fn create_identifier(&self) -> Result<Identifier, ProtocolError> {
        let outpoint = self.out_point().ok_or_else(|| {
            ProtocolError::IdentifierError(String::from("No output at a given index"))
        })?;

        let outpoint_bytes: [u8; 36] = outpoint.into();

        let hash = hash_double(outpoint_bytes.as_slice());

        Ok(Identifier::new(hash))
    }

    #[cfg(feature = "cbor")]
    pub fn to_cbor_buffer(&self) -> Result<Vec<u8>, ProtocolError> {
        let mut map = CborCanonicalMap::new();
        let mut is_lock_buffer = Vec::<u8>::new();
        let mut transaction_buffer = Vec::<u8>::new();
        self.instant_lock
            .consensus_encode(&mut is_lock_buffer)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;
        self.transaction
            .consensus_encode(&mut transaction_buffer)
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))?;

        map.insert("outputIndex", self.output_index);
        map.insert("transaction", transaction_buffer);
        map.insert("instantLock", is_lock_buffer);

        map.to_bytes()
            .map_err(|e| ProtocolError::EncodingError(e.to_string()))
    }

    /// Validate Instant Asset Lock Proof structure
    #[cfg(feature = "validation")]
    pub fn validate_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .identities
            .asset_locks
            .validate_instant_asset_lock_proof_structure
        {
            0 => methods::validate_structure::validate_instant_asset_lock_proof_structure_v0(
                self,
                platform_version,
            ),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "validate_instant_asset_lock_proof_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
/// "Raw" instant lock for serialization
pub struct RawInstantLockProof {
    instant_lock: BinaryData,
    transaction: BinaryData,
    output_index: u32,
}

impl TryFrom<RawInstantLockProof> for InstantAssetLockProof {
    type Error = ProtocolError;

    fn try_from(raw_instant_lock: RawInstantLockProof) -> Result<Self, Self::Error> {
        let transaction = deserialize(raw_instant_lock.transaction.as_slice())
            .map_err(|e| ProtocolError::DecodingError(e.to_string()))?;
        let instant_lock = deserialize(raw_instant_lock.instant_lock.as_slice())
            .map_err(|e| ProtocolError::DecodingError(e.to_string()))?;

        Ok(Self {
            transaction,
            instant_lock,
            output_index: raw_instant_lock.output_index,
        })
    }
}

impl TryFrom<&InstantAssetLockProof> for RawInstantLockProof {
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
            instant_lock: BinaryData::new(is_lock_buffer),
            transaction: BinaryData::new(transaction_buffer),
            output_index: instant_asset_lock_proof.output_index,
        })
    }
}
