use crate::bls_signatures::PublicKey as BlsPublicKey;
use crate::core_types::validator::v0::ValidatorV0;
#[cfg(feature = "core-types-serialization")]
use bincode::de::Decoder;
#[cfg(feature = "core-types-serialization")]
use bincode::enc::Encoder;
#[cfg(feature = "core-types-serialization")]
use bincode::error::EncodeError;
#[cfg(feature = "core-types-serialization")]
use bincode::{BorrowDecode, Decode, Encode};
use dashcore::hashes::Hash;
use dashcore::{ProTxHash, QuorumHash};
#[cfg(feature = "core-types-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};

/// The validator set is only slightly different from a quorum as it does not contain non valid
/// members
#[derive(Clone, Eq, PartialEq)]
#[cfg_attr(
    feature = "core-types-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub struct ValidatorSetV0 {
    /// The quorum hash
    pub quorum_hash: QuorumHash,
    /// Rotation quorum index is available only for DIP24 quorums
    pub quorum_index: Option<u32>,
    /// Active height
    pub core_height: u32,
    /// The list of masternodes
    pub members: BTreeMap<ProTxHash, ValidatorV0>,
    /// The threshold quorum public key
    pub threshold_public_key: BlsPublicKey,
}

#[cfg(feature = "core-types-serialization")]
impl Encode for ValidatorSetV0 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        // Encode each field in the order they appear in the struct
        let quorum_hash_bytes = self.quorum_hash.as_byte_array().to_vec();
        quorum_hash_bytes.encode(encoder)?;
        self.quorum_index.encode(encoder)?;
        self.core_height.encode(encoder)?;

        // Convert BTreeMap<ProTxHash, ValidatorV0> to Vec<(Vec<u8>, ValidatorV0)> and encode it
        let members_as_vec: Vec<(Vec<u8>, ValidatorV0)> = self
            .members
            .iter()
            .map(|(key, value)| (key.as_byte_array().to_vec(), value.clone()))
            .collect();
        members_as_vec.encode(encoder)?;

        // Custom encoding for BlsPublicKey if needed
        // Assuming BlsPublicKey can be serialized to a byte slice
        let public_key_bytes = self.threshold_public_key.to_bytes();
        public_key_bytes.encode(encoder)?;

        Ok(())
    }
}

#[cfg(feature = "core-types-serialization")]
impl Decode for ValidatorSetV0 {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, bincode::error::DecodeError> {
        // Decode each field in the same order as they were encoded
        let quorum_hash = Vec::<u8>::decode(decoder)?;
        let quorum_index = Option::<u32>::decode(decoder)?;
        let core_height = u32::decode(decoder)?;

        // Decode Vec<(Vec<u8>, ValidatorV0)> and convert it back to BTreeMap<ProTxHash, ValidatorV0>
        let members_as_vec = Vec::<(Vec<u8>, ValidatorV0)>::decode(decoder)?;
        let members: BTreeMap<ProTxHash, ValidatorV0> = members_as_vec
            .into_iter()
            .map(|(key_bytes, value)| {
                let key = ProTxHash::from_slice(&key_bytes).map_err(|_| {
                    bincode::error::DecodeError::OtherString(
                        "Failed to decode ProTxHash".to_string(),
                    )
                })?;
                Ok((key, value))
            })
            .collect::<Result<_, bincode::error::DecodeError>>()?;

        // Custom decoding for BlsPublicKey if needed
        // Assuming BlsPublicKey can be deserialized from a byte slice
        let public_key_bytes = Vec::<u8>::decode(decoder)?;
        let threshold_public_key = BlsPublicKey::from_bytes(&public_key_bytes).map_err(|_| {
            bincode::error::DecodeError::OtherString("Failed to decode BlsPublicKey".to_string())
        })?;

        Ok(ValidatorSetV0 {
            quorum_hash: QuorumHash::from_slice(&quorum_hash).map_err(|_| {
                bincode::error::DecodeError::OtherString("Failed to decode QuorumHash".to_string())
            })?,
            quorum_index,
            core_height,
            members,
            threshold_public_key,
        })
    }
}

#[cfg(feature = "core-types-serialization")]
impl<'de> BorrowDecode<'de> for ValidatorSetV0 {
    fn borrow_decode<D: Decoder>(decoder: &mut D) -> Result<Self, bincode::error::DecodeError> {
        // Decode each field in the same order as they were encoded

        // Decode quorum_hash as Vec<u8>
        let quorum_hash = Vec::<u8>::decode(decoder)?;
        // Decode quorum_index as Option<u32>
        let quorum_index = Option::<u32>::decode(decoder)?;
        // Decode core_height as u32
        let core_height = u32::decode(decoder)?;

        // Decode Vec<(Vec<u8>, ValidatorV0)> and convert it back to BTreeMap<ProTxHash, ValidatorV0>
        let members_as_vec = Vec::<(Vec<u8>, ValidatorV0)>::decode(decoder)?;
        let members: BTreeMap<ProTxHash, ValidatorV0> = members_as_vec
            .into_iter()
            .map(|(key_bytes, value)| {
                let key = ProTxHash::from_slice(&key_bytes).map_err(|_| {
                    bincode::error::DecodeError::OtherString(
                        "Failed to decode ProTxHash".to_string(),
                    )
                })?;
                Ok((key, value))
            })
            .collect::<Result<_, bincode::error::DecodeError>>()?;

        // Custom decoding for BlsPublicKey if needed
        let public_key_bytes = Vec::<u8>::decode(decoder)?;
        let threshold_public_key = BlsPublicKey::from_bytes(&public_key_bytes).map_err(|_| {
            bincode::error::DecodeError::OtherString("Failed to decode BlsPublicKey".to_string())
        })?;

        Ok(ValidatorSetV0 {
            quorum_hash: QuorumHash::from_slice(&quorum_hash).map_err(|_| {
                bincode::error::DecodeError::OtherString("Failed to decode QuorumHash".to_string())
            })?,
            quorum_index,
            core_height,
            members,
            threshold_public_key,
        })
    }
}

impl Debug for ValidatorSetV0 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatorSetV0")
            .field("quorum_hash", &self.quorum_hash.to_string())
            .field("core_height", &self.core_height)
            .field(
                "members",
                &self
                    .members
                    .iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect::<BTreeMap<String, &ValidatorV0>>(),
            )
            .field("threshold_public_key", &self.threshold_public_key)
            .finish()
    }
}

/// Trait providing getter methods for `ValidatorSetV0` struct
pub trait ValidatorSetV0Getters {
    /// Returns the quorum hash of the validator set.
    fn quorum_hash(&self) -> &QuorumHash;
    /// Returns rotation quorum index. It's available only for DIP24 quorums
    fn quorum_index(&self) -> Option<u32>;
    /// Returns the active height of the validator set.
    fn core_height(&self) -> u32;
    /// Returns the members of the validator set.
    fn members(&self) -> &BTreeMap<ProTxHash, ValidatorV0>;
    /// Returns the members of the validator set.
    fn members_mut(&mut self) -> &mut BTreeMap<ProTxHash, ValidatorV0>;
    /// Returns the members of the validator set.
    fn members_owned(self) -> BTreeMap<ProTxHash, ValidatorV0>;
    /// Returns the threshold public key of the validator set.
    fn threshold_public_key(&self) -> &BlsPublicKey;
}

/// Trait providing setter methods for `ValidatorSetV0` struct
pub trait ValidatorSetV0Setters {
    /// Sets the quorum hash of the validator set.
    fn set_quorum_hash(&mut self, quorum_hash: QuorumHash);
    /// Sets the quorum index of the validator set.
    fn set_quorum_index(&mut self, index: Option<u32>);
    /// Sets the active height of the validator set.
    fn set_core_height(&mut self, core_height: u32);
    /// Sets the members of the validator set.
    fn set_members(&mut self, members: BTreeMap<ProTxHash, ValidatorV0>);
    /// Sets the threshold public key of the validator set.
    fn set_threshold_public_key(&mut self, threshold_public_key: BlsPublicKey);
}

impl ValidatorSetV0Getters for ValidatorSetV0 {
    fn quorum_hash(&self) -> &QuorumHash {
        &self.quorum_hash
    }

    fn quorum_index(&self) -> Option<u32> {
        self.quorum_index
    }

    fn core_height(&self) -> u32 {
        self.core_height
    }

    fn members(&self) -> &BTreeMap<ProTxHash, ValidatorV0> {
        &self.members
    }

    fn members_mut(&mut self) -> &mut BTreeMap<ProTxHash, ValidatorV0> {
        &mut self.members
    }

    fn members_owned(self) -> BTreeMap<ProTxHash, ValidatorV0> {
        self.members
    }

    fn threshold_public_key(&self) -> &BlsPublicKey {
        &self.threshold_public_key
    }
}

impl ValidatorSetV0Setters for ValidatorSetV0 {
    fn set_quorum_hash(&mut self, quorum_hash: QuorumHash) {
        self.quorum_hash = quorum_hash;
    }

    fn set_quorum_index(&mut self, index: Option<u32>) {
        self.quorum_index = index;
    }

    fn set_core_height(&mut self, core_height: u32) {
        self.core_height = core_height;
    }

    fn set_members(&mut self, members: BTreeMap<ProTxHash, ValidatorV0>) {
        self.members = members;
    }

    fn set_threshold_public_key(&mut self, threshold_public_key: BlsPublicKey) {
        self.threshold_public_key = threshold_public_key;
    }
}
