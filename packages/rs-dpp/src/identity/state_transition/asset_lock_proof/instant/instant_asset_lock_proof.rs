#[cfg(feature = "json-conversion")]
use crate::serialization::json_safe_fields;
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
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
#[cfg_attr(feature = "json-conversion", json_safe_fields)]
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstantAssetLockProof {
    /// The transaction's Instant Lock
    pub instant_lock: InstantLock,
    /// Asset Lock Special Transaction
    pub transaction: Transaction,
    /// Index of the output in the transaction payload
    pub output_index: u32,
}

// Manual Serialize/Deserialize via the `RawInstantLockProof` DTO bridge:
// `InstantLock`/`Transaction` are consensus-encoded byte blobs on the wire
// (base64 in HR JSON, bytes otherwise), not serde-shaped structs, so the
// derive can't express the shape. Not a tagging customization.
impl Serialize for InstantAssetLockProof {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let raw = RawInstantLockProof::try_from(self).map_err(|e| {
            S::Error::custom(format!(
                "expected to be able to convert raw instant lock proof in serialization: {}",
                e
            ))
        })?;

        raw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for InstantAssetLockProof {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawInstantLockProof::deserialize(deserializer)?;
        raw.try_into().map_err(|e: ProtocolError| {
            D::Error::custom(format!(
                "expected to be able to convert raw instant lock proof in deserialization: {}",
                e
            ))
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::fixtures::raw_instant_asset_lock_proof_fixture;

    // ---------------------------------------------------------------
    // Default
    // ---------------------------------------------------------------

    #[test]
    fn test_default_instant_asset_lock_proof() {
        let proof = InstantAssetLockProof::default();
        assert_eq!(proof.output_index(), 0);
        assert_eq!(proof.transaction().version, 0);
        assert_eq!(proof.transaction().lock_time, 0);
        assert_eq!(proof.transaction().input.len(), 1);
        assert_eq!(proof.transaction().output.len(), 1);
        assert!(proof.transaction().special_transaction_payload.is_none());
    }

    // ---------------------------------------------------------------
    // Constructor and accessors
    // ---------------------------------------------------------------

    #[test]
    fn test_new_stores_fields_correctly() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        assert_eq!(proof.output_index(), 0);
        // Verify the instant lock and transaction are accessible
        let _il = proof.instant_lock();
        let _tx = proof.transaction();
    }

    #[test]
    fn test_instant_lock_accessor() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        let il = proof.instant_lock();
        assert_eq!(il.version, 1);
        assert_eq!(il.inputs.len(), 1);
    }

    #[test]
    fn test_transaction_accessor() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        let tx = proof.transaction();
        // DIP-0002 special transactions (AssetLock) use version 3.
        assert_eq!(tx.version, 3);
        assert_eq!(tx.lock_time, 0);
        assert_eq!(tx.input.len(), 1);
    }

    #[test]
    fn test_output_index_accessor() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        assert_eq!(proof.output_index(), 0);
    }

    #[test]
    fn test_output_index_with_custom_value() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        // Create a new proof with a different output_index
        let custom_proof =
            InstantAssetLockProof::new(proof.instant_lock.clone(), proof.transaction.clone(), 5);
        assert_eq!(custom_proof.output_index(), 5);
    }

    // ---------------------------------------------------------------
    // output()
    // ---------------------------------------------------------------

    #[test]
    fn test_output_returns_some_for_valid_asset_lock_transaction() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        // The fixture creates a transaction with AssetLockPayloadType containing one credit output
        let output = proof.output();
        assert!(output.is_some());
    }

    #[test]
    fn test_output_returns_none_for_default_transaction() {
        let proof = InstantAssetLockProof::default();
        // Default transaction has no special_transaction_payload
        assert!(proof.output().is_none());
    }

    #[test]
    fn test_output_returns_none_for_out_of_range_index() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        // Fixture has output_index 0 and only 1 credit output, so index 99 should be out of range
        let modified_proof =
            InstantAssetLockProof::new(proof.instant_lock.clone(), proof.transaction.clone(), 99);
        assert!(modified_proof.output().is_none());
    }

    // ---------------------------------------------------------------
    // out_point()
    // ---------------------------------------------------------------

    #[test]
    fn test_out_point_returns_some_for_valid_proof() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        let outpoint = proof.out_point();
        assert!(outpoint.is_some());
        let outpoint = outpoint.unwrap();
        assert_eq!(outpoint.txid, proof.transaction.txid());
        assert_eq!(outpoint.vout, 0);
    }

    #[test]
    fn test_out_point_returns_none_for_default() {
        let proof = InstantAssetLockProof::default();
        assert!(proof.out_point().is_none());
    }

    // ---------------------------------------------------------------
    // create_identifier()
    // ---------------------------------------------------------------

    #[test]
    fn test_create_identifier_succeeds_for_valid_proof() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        let result = proof.create_identifier();
        assert!(result.is_ok());
        let identifier = result.unwrap();
        // Identifier should be 32 bytes
        assert_eq!(identifier.as_slice().len(), 32);
    }

    #[test]
    fn test_create_identifier_deterministic() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        let id1 = proof.create_identifier().unwrap();
        let id2 = proof.create_identifier().unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_create_identifier_fails_for_default() {
        let proof = InstantAssetLockProof::default();
        let result = proof.create_identifier();
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // to_object() — canonical ValueConvertible
    // ---------------------------------------------------------------

    #[test]
    fn test_to_object_succeeds() {
        use crate::serialization::ValueConvertible;
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        let result = proof.to_object();
        assert!(result.is_ok());
    }

    // ---------------------------------------------------------------
    // RawInstantLockProof round-trip
    // ---------------------------------------------------------------

    #[test]
    fn test_raw_instant_lock_proof_round_trip() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        let raw = RawInstantLockProof::try_from(&proof).unwrap();
        let recovered = InstantAssetLockProof::try_from(raw).unwrap();

        assert_eq!(recovered.output_index, proof.output_index);
        assert_eq!(recovered.instant_lock, proof.instant_lock);
        assert_eq!(recovered.transaction.txid(), proof.transaction.txid());
    }

    #[test]
    fn test_raw_instant_lock_proof_preserves_output_index() {
        let base = raw_instant_asset_lock_proof_fixture(None, None);
        let proof =
            InstantAssetLockProof::new(base.instant_lock.clone(), base.transaction.clone(), 7);
        let raw = RawInstantLockProof::try_from(&proof).unwrap();
        assert_eq!(raw.output_index, 7);
        let recovered = InstantAssetLockProof::try_from(raw).unwrap();
        assert_eq!(recovered.output_index(), 7);
    }

    // ---------------------------------------------------------------
    // Eq / Clone
    // ---------------------------------------------------------------

    #[test]
    fn test_clone_equals_original() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        let cloned = proof.clone();
        assert_eq!(proof, cloned);
    }

    #[test]
    fn test_different_output_index_not_equal() {
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        let mut modified = proof.clone();
        modified.output_index = 1;
        assert_ne!(proof, modified);
    }

    // ---------------------------------------------------------------
    // TryFrom<Value>
    // ---------------------------------------------------------------

    #[test]
    fn test_try_from_value_round_trip() {
        use crate::serialization::ValueConvertible;
        let proof = raw_instant_asset_lock_proof_fixture(None, None);
        let value = proof.to_object().unwrap();
        let recovered = InstantAssetLockProof::try_from(value).unwrap();
        assert_eq!(proof.output_index, recovered.output_index);
        assert_eq!(proof.instant_lock, recovered.instant_lock);
        assert_eq!(proof.transaction.txid(), recovered.transaction.txid());
    }
}

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests_instantassetlockproof {
    use super::*;

    #[test]
    fn json_round_trip_instantassetlockproof() {
        use crate::serialization::JsonConvertible;
        let original = InstantAssetLockProof::default();
        let json = original.to_json().expect("to_json");
        let recovered = InstantAssetLockProof::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_instantassetlockproof() {
        use crate::serialization::ValueConvertible;
        let original = InstantAssetLockProof::default();
        let value = original.to_object().expect("to_object");
        let recovered = InstantAssetLockProof::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
