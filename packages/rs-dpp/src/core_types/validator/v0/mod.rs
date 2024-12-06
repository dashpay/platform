use dashcore::{ProTxHash, PubkeyHash};
use std::fmt::{Debug, Formatter};

use crate::bls_signatures::{Bls12381G2Impl, PublicKey as BlsPublicKey};
#[cfg(feature = "core-types-serde-conversion")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "core-types-serialization")]
use bincode::de::Decoder;
#[cfg(feature = "core-types-serialization")]
use bincode::enc::Encoder;
#[cfg(feature = "core-types-serialization")]
use bincode::error::{DecodeError, EncodeError};
#[cfg(feature = "core-types-serialization")]
use bincode::{Decode, Encode};
#[cfg(feature = "core-types-serialization")]
use dashcore::hashes::Hash;

/// A validator in the context of a quorum
#[derive(Clone, Eq, PartialEq)]
#[cfg_attr(
    feature = "core-types-serde-conversion",
    derive(Serialize, Deserialize)
)]
pub struct ValidatorV0 {
    /// The proTxHash
    pub pro_tx_hash: ProTxHash,
    /// The public key share of this validator for this quorum
    pub public_key: Option<BlsPublicKey<Bls12381G2Impl>>,
    /// The node address
    pub node_ip: String,
    /// The node id
    pub node_id: PubkeyHash,
    /// Core port
    pub core_port: u16,
    /// Http port
    pub platform_http_port: u16,
    /// Tenderdash port
    pub platform_p2p_port: u16,
    /// Is the validator banned
    pub is_banned: bool,
}

#[cfg(feature = "core-types-serialization")]
impl Encode for ValidatorV0 {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        // Encode each field in the order they appear in the struct

        // Encode ProTxHash
        let pro_tx_hash_bytes = self.pro_tx_hash.as_byte_array();
        pro_tx_hash_bytes.encode(encoder)?;

        // Encode Option<BlsPublicKey>
        match &self.public_key {
            Some(public_key) => {
                true.encode(encoder)?; // Indicate that public_key is present
                public_key.0.to_compressed().encode(encoder)?;
            }
            None => {
                false.encode(encoder)?; // Indicate that public_key is not present
            }
        }

        // Encode node_ip as a string
        self.node_ip.encode(encoder)?;

        // Encode node_id
        let node_id_bytes = self.node_id.as_byte_array();
        node_id_bytes.encode(encoder)?;

        // Encode core_port, platform_http_port, and platform_p2p_port as u16
        self.core_port.encode(encoder)?;
        self.platform_http_port.encode(encoder)?;
        self.platform_p2p_port.encode(encoder)?;

        // Encode is_banned as a boolean
        self.is_banned.encode(encoder)?;

        Ok(())
    }
}

#[cfg(feature = "core-types-serialization")]
impl Decode for ValidatorV0 {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
        // Decode each field in the same order as they were encoded

        // Decode ProTxHash
        let pro_tx_hash_bytes = <[u8; 32]>::decode(decoder)?;
        let pro_tx_hash = ProTxHash::from_slice(&pro_tx_hash_bytes)
            .map_err(|_| DecodeError::OtherString("Failed to decode ProTxHash".to_string()))?;

        // Decode Option<BlsPublicKey>
        let has_public_key = bool::decode(decoder)?;
        let public_key = if has_public_key {
            let public_key_bytes = <[u8; 48]>::decode(decoder)?;

            Some(
                BlsPublicKey::try_from(public_key_bytes.as_slice()).map_err(|_| {
                    DecodeError::OtherString("Failed to decode BlsPublicKey".to_string())
                })?,
            )
        } else {
            None
        };

        // Decode node_ip as a string
        let node_ip = String::decode(decoder)?;

        // Decode node_id
        let node_id_bytes = <[u8; 20]>::decode(decoder)?;
        let node_id = PubkeyHash::from_slice(&node_id_bytes)
            .map_err(|_| DecodeError::OtherString("Failed to decode NodeId".to_string()))?;

        // Decode core_port, platform_http_port, and platform_p2p_port as u16
        let core_port = u16::decode(decoder)?;
        let platform_http_port = u16::decode(decoder)?;
        let platform_p2p_port = u16::decode(decoder)?;

        // Decode is_banned as a boolean
        let is_banned = bool::decode(decoder)?;

        Ok(ValidatorV0 {
            pro_tx_hash,
            public_key,
            node_ip,
            node_id,
            core_port,
            platform_http_port,
            platform_p2p_port,
            is_banned,
        })
    }
}

impl Debug for ValidatorV0 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatorV0")
            .field("pro_tx_hash", &self.pro_tx_hash.to_string())
            .field("public_key", &self.public_key)
            .field("node_ip", &self.node_ip)
            .field("node_id", &self.node_id)
            .field("core_port", &self.core_port)
            .field("platform_http_port", &self.platform_http_port)
            .field("platform_p2p_port", &self.platform_p2p_port)
            .field("is_banned", &self.is_banned)
            .finish()
    }
}

/// Traits to get properties of a validator.
pub trait ValidatorV0Getters {
    /// Returns the proTxHash of the validator.
    fn pro_tx_hash(&self) -> &ProTxHash;
    /// Returns the public key share of this validator for this quorum.
    fn public_key(&self) -> &Option<BlsPublicKey<Bls12381G2Impl>>;
    /// Returns the node address of the validator.
    fn node_ip(&self) -> &String;
    /// Returns the node id of the validator.
    fn node_id(&self) -> &PubkeyHash;
    /// Returns the core port of the validator.
    fn core_port(&self) -> u16;
    /// Returns the Http port of the validator.
    fn platform_http_port(&self) -> u16;
    /// Returns the Tenderdash port of the validator.
    fn platform_p2p_port(&self) -> u16;
    /// Returns the status of the validator whether it's banned or not.
    fn is_banned(&self) -> bool;
}

/// Traits to set properties of a validator.
pub trait ValidatorV0Setters {
    /// Sets the proTxHash of the validator.
    fn set_pro_tx_hash(&mut self, pro_tx_hash: ProTxHash);
    /// Sets the public key share of this validator for this quorum.
    fn set_public_key(&mut self, public_key: Option<BlsPublicKey<Bls12381G2Impl>>);
    /// Sets the node address of the validator.
    fn set_node_ip(&mut self, node_ip: String);
    /// Sets the node id of the validator.
    fn set_node_id(&mut self, node_id: PubkeyHash);
    /// Sets the core port of the validator.
    fn set_core_port(&mut self, core_port: u16);
    /// Sets the Http port of the validator.
    fn set_platform_http_port(&mut self, platform_http_port: u16);
    /// Sets the Tenderdash port of the validator.
    fn set_platform_p2p_port(&mut self, platform_p2p_port: u16);
    /// Sets the status of the validator whether it's banned or not.
    fn set_is_banned(&mut self, is_banned: bool);
}

impl ValidatorV0Getters for ValidatorV0 {
    fn pro_tx_hash(&self) -> &ProTxHash {
        &self.pro_tx_hash
    }

    fn public_key(&self) -> &Option<BlsPublicKey<Bls12381G2Impl>> {
        &self.public_key
    }

    fn node_ip(&self) -> &String {
        &self.node_ip
    }

    fn node_id(&self) -> &PubkeyHash {
        &self.node_id
    }

    fn core_port(&self) -> u16 {
        self.core_port
    }

    fn platform_http_port(&self) -> u16 {
        self.platform_http_port
    }

    fn platform_p2p_port(&self) -> u16 {
        self.platform_p2p_port
    }

    fn is_banned(&self) -> bool {
        self.is_banned
    }
}

impl ValidatorV0Setters for ValidatorV0 {
    fn set_pro_tx_hash(&mut self, pro_tx_hash: ProTxHash) {
        self.pro_tx_hash = pro_tx_hash;
    }

    fn set_public_key(&mut self, public_key: Option<BlsPublicKey<Bls12381G2Impl>>) {
        self.public_key = public_key;
    }

    fn set_node_ip(&mut self, node_ip: String) {
        self.node_ip = node_ip;
    }

    fn set_node_id(&mut self, node_id: PubkeyHash) {
        self.node_id = node_id;
    }

    fn set_core_port(&mut self, core_port: u16) {
        self.core_port = core_port;
    }

    fn set_platform_http_port(&mut self, platform_http_port: u16) {
        self.platform_http_port = platform_http_port;
    }

    fn set_platform_p2p_port(&mut self, platform_p2p_port: u16) {
        self.platform_p2p_port = platform_p2p_port;
    }

    fn set_is_banned(&mut self, is_banned: bool) {
        self.is_banned = is_banned;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bincode::config;
    use dashcore::blsful::SecretKey;
    use rand::prelude::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_serialize_deserialize_validator_v0() {
        // Sample data for testing
        let pro_tx_hash = ProTxHash::from_slice(&[1; 32]).unwrap();
        let mut rng = StdRng::seed_from_u64(0);
        let public_key = Some(SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key());
        let node_ip = "127.0.0.1".to_string();
        let node_id = PubkeyHash::from_slice(&[3; 20]).unwrap();
        let core_port = 9999;
        let platform_http_port = 8888;
        let platform_p2p_port = 7777;
        let is_banned = false;

        // Create a ValidatorV0 instance
        let validator = ValidatorV0 {
            pro_tx_hash,
            public_key,
            node_ip,
            node_id,
            core_port,
            platform_http_port,
            platform_p2p_port,
            is_banned,
        };

        // Serialize the ValidatorV0 instance
        let encoded = bincode::encode_to_vec(&validator, config::standard()).unwrap();

        // Deserialize the data back into a ValidatorV0 instance
        let decoded: ValidatorV0 = bincode::decode_from_slice(&encoded, config::standard())
            .unwrap()
            .0;

        // Verify that the deserialized instance matches the original instance
        assert_eq!(validator, decoded);
    }
}
