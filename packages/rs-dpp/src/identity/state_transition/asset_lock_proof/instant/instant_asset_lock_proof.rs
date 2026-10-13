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

    // ---------------------------------------------------------------
    // Consensus decoding of a real proof
    // ---------------------------------------------------------------

    /// An asset lock with one credit output, as Dash Core relayed it.
    const CORE_ASSET_LOCK: &str = "03000800014d6d36c50d484aa79f7db080f971c3f6845407f652c7d5865756017fa06969c1010000006a47304402200136894a2ebb4967cf2766c10e238d69c53c24bf330758e4432eb4753def03de02202a2afb05475a064a419a6cc1c582e3504fcb36c2e22b610b5d320f7656573f7f0121028fdb0a3f730bb20f477536d98ca830efa56412dd05992c801219ba0ff35ad530ffffffff028801030000000000026a00288d9500000000001976a9148d40dfe30494080a1c1187c74066956043ff13fb88ac0000000024010188010300000000001976a914aa85a9fb4f84bc63046a574ac4f2ce3361f0db0d88ac";
    /// The InstantSend lock Dash Core signed for it.
    const CORE_INSTANT_LOCK: &str = "01014d6d36c50d484aa79f7db080f971c3f6845407f652c7d5865756017fa06969c1010000008155cc5d9fe5da3b0508c28d02c88fb6d3d4cf44ef4ffcd77162afa338d1a181ad7300e92255a7a7cf031d6de6bac99df9f1b94735ea603b3f03060c3ebf1f37acc4c1d8ddea77f3d4d816e467571f51ae216715fb3e47d68831adeee6aa1640b26cdf085bb8dd0b4920d15eed83e8c50de8b4b0508db47f08451f7807194d68758a92b367ef6074b516336f689c75c5e22b87aa71d50157875f1018a305a957";

    /// Every node decodes an instant proof's transaction from its bytes, and
    /// the transition's outcome follows from what it decodes. A rust-dashcore
    /// upgrade must decode a real asset lock to the same transaction: the
    /// same bytes back, the txid the InstantSend lock signs, and the same
    /// credit output.
    #[test]
    fn test_raw_instant_lock_proof_decodes_a_core_asset_lock_unchanged() {
        let transaction_bytes = hex::decode(CORE_ASSET_LOCK).unwrap();
        let raw = RawInstantLockProof {
            instant_lock: BinaryData::new(hex::decode(CORE_INSTANT_LOCK).unwrap()),
            transaction: BinaryData::new(transaction_bytes.clone()),
            output_index: 0,
        };

        let proof = InstantAssetLockProof::try_from(raw).expect("a Core asset lock decodes");

        assert_eq!(
            dashcore::consensus::serialize(proof.transaction()),
            transaction_bytes
        );
        assert_eq!(
            proof.transaction().txid().to_string(),
            "81a1d138a3af6271d7fc4fef44cfd4d3b68fc8028dc208053bdae59f5dcc5581"
        );
        assert_eq!(proof.instant_lock().txid, proof.transaction().txid());
        let Some(TransactionPayload::AssetLockPayloadType(payload)) =
            &proof.transaction().special_transaction_payload
        else {
            panic!("an asset lock payload");
        };
        assert_eq!(payload.version, 1);
        assert_eq!(payload.credit_outputs.len(), 1);
        let output = proof.output().expect("credit output 0");
        assert_eq!(output.value, 197_000);
        assert_eq!(
            hex::encode(output.script_pubkey.as_bytes()),
            "76a914aa85a9fb4f84bc63046a574ac4f2ce3361f0db0d88ac"
        );
        // The identity the proof funds: sha256d of its outpoint.
        assert_eq!(
            proof.create_identifier().unwrap(),
            Identifier::from_string(
                "HY8jYYsTJxHPcuYreupv4Fb4nvdPRZfV6GoxybLt6vhu",
                platform_value::string_encoding::Encoding::Base58,
            )
            .unwrap()
        );
    }

    /// Core reads a special transaction payload only from transaction
    /// version 3 (a signed 16-bit version), so below it, or negative, the
    /// asset lock's payload is not part of the transaction. Its bytes, with
    /// the payload still appended, are not a transaction a proof can carry.
    #[test]
    fn test_raw_instant_lock_proof_refuses_an_asset_lock_payload_below_version_3() {
        for version in [1u16, 2, 0x8000, 0xffff] {
            let mut transaction_bytes = hex::decode(CORE_ASSET_LOCK).unwrap();
            transaction_bytes[0..2].copy_from_slice(&version.to_le_bytes());
            let raw = RawInstantLockProof {
                instant_lock: BinaryData::new(hex::decode(CORE_INSTANT_LOCK).unwrap()),
                transaction: BinaryData::new(transaction_bytes),
                output_index: 0,
            };
            assert!(
                InstantAssetLockProof::try_from(raw).is_err(),
                "version {version:#06x}"
            );
        }
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

// This graph decodes raw proof fields through Serde before validating the
// consensus bytes. The untrusted adapter suppresses allocation hints.
impl<'de> bincode::serde::DeserializeUntrusted<'de> for InstantAssetLockProof {}
