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
use dashcore::blsful::Bls12381G2Impl;
#[cfg(feature = "core-types-serialization")]
use dashcore::hashes::Hash;
use dashcore::{ProTxHash, QuorumHash};
use itertools::Itertools;
#[cfg(feature = "core-types-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};

/// The validator set is only slightly different from a quorum as it does not contain non-valid
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
    pub threshold_public_key: BlsPublicKey<Bls12381G2Impl>,
}

impl Display for ValidatorSetV0 {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ValidatorSet {{
    quorum_hash: {},
    quorum_index: {},
    core_height: {},
    members: [{}],
    threshold_public_key: {}
}}",
            hex::encode(self.quorum_hash), // Assuming QuorumHash is a byte array and should be in hex format
            match self.quorum_index {
                Some(index) => index.to_string(),
                None => "None".to_string(),
            },
            self.core_height,
            self.members
                .iter()
                .map(|(pro_tx_hash, validator)| format!(
                    "{{{}: {}}}",
                    pro_tx_hash, validator.node_ip
                ))
                .join(", "),
            hex::encode(self.threshold_public_key.0.to_compressed()) // Assuming BlsPublicKey is a byte array
        )
    }
}

#[cfg(feature = "core-types-serialization")]
impl Encode for ValidatorSetV0 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        // Encode each field in the order they appear in the struct
        let quorum_hash_bytes = self.quorum_hash.as_byte_array();
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
        let public_key_bytes = self.threshold_public_key.0.to_compressed();
        public_key_bytes.encode(encoder)?;

        Ok(())
    }
}

#[cfg(feature = "core-types-serialization")]
impl Decode for ValidatorSetV0 {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, bincode::error::DecodeError> {
        // Decode the quorum hash directly as a [u8; 32] array
        let quorum_hash = <[u8; 32]>::decode(decoder)?;
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

        // Decode the [u8; 48] directly
        let mut public_key_bytes = [0u8; 48];
        let bytes = <[u8; 48]>::decode(decoder)?;
        public_key_bytes.copy_from_slice(&bytes);
        let threshold_public_key =
            BlsPublicKey::try_from(public_key_bytes.as_slice()).map_err(|e| {
                bincode::error::DecodeError::OtherString(format!(
                    "Failed to decode BlsPublicKey: {}",
                    e
                ))
            })?;

        Ok(ValidatorSetV0 {
            quorum_hash: QuorumHash::from_byte_array(quorum_hash),
            quorum_index,
            core_height,
            members,
            threshold_public_key,
        })
    }
}

#[cfg(feature = "core-types-serialization")]
impl BorrowDecode<'_> for ValidatorSetV0 {
    fn borrow_decode<D: Decoder>(decoder: &mut D) -> Result<Self, bincode::error::DecodeError> {
        // Decode each field in the same order as they were encoded

        // Decode the quorum hash directly as a [u8; 32] array
        let quorum_hash = <[u8; 32]>::decode(decoder)?;
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
        let mut public_key_bytes = [0u8; 48];
        let bytes = <[u8; 48]>::decode(decoder)?;
        public_key_bytes.copy_from_slice(&bytes);
        let threshold_public_key =
            BlsPublicKey::try_from(public_key_bytes.as_slice()).map_err(|e| {
                bincode::error::DecodeError::OtherString(format!(
                    "Failed to decode BlsPublicKey in borrow decode: {}",
                    e
                ))
            })?;

        Ok(ValidatorSetV0 {
            quorum_hash: QuorumHash::from_byte_array(quorum_hash),
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
    fn threshold_public_key(&self) -> &BlsPublicKey<Bls12381G2Impl>;
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
    fn set_threshold_public_key(&mut self, threshold_public_key: BlsPublicKey<Bls12381G2Impl>);
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

    fn threshold_public_key(&self) -> &BlsPublicKey<Bls12381G2Impl> {
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

    fn set_threshold_public_key(&mut self, threshold_public_key: BlsPublicKey<Bls12381G2Impl>) {
        self.threshold_public_key = threshold_public_key;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::config;
    use dashcore::blsful::SecretKey;
    use dashcore::PubkeyHash;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    #[test]
    fn test_serialize_deserialize_validator_set_v0() {
        // Sample data for testing
        let quorum_hash = QuorumHash::from_slice(&[1; 32]).unwrap();
        let quorum_index = Some(42);
        let core_height = 1000;

        // Create a sample ProTxHash and ValidatorV0 instance
        let pro_tx_hash = ProTxHash::from_slice(&[2; 32]).unwrap();
        let mut rng = StdRng::seed_from_u64(0);
        let public_key = Some(SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key());
        let node_ip = "192.168.1.1".to_string();
        let node_id = PubkeyHash::from_slice(&[4; 20]).unwrap();
        let validator = ValidatorV0 {
            pro_tx_hash,
            public_key,
            node_ip,
            node_id,
            core_port: 8080,
            platform_http_port: 9090,
            platform_p2p_port: 10010,
            is_banned: false,
        };

        // Create a BTreeMap with one entry for the ValidatorSetV0
        let mut members = BTreeMap::new();
        members.insert(pro_tx_hash, validator);

        // Create a sample threshold public key
        let threshold_public_key = SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key();

        // Create the ValidatorSetV0 instance
        let validator_set = ValidatorSetV0 {
            quorum_hash,
            quorum_index,
            core_height,
            members,
            threshold_public_key,
        };

        // Serialize the ValidatorSetV0 instance
        let encoded = bincode::encode_to_vec(&validator_set, config::standard()).unwrap();

        // Deserialize the data back into a ValidatorSetV0 instance
        let decoded: ValidatorSetV0 = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        // Verify that the deserialized instance matches the original instance
        assert_eq!(validator_set, decoded);
    }
}
