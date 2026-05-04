use crate::bls_signatures::{Bls12381G2Impl, PublicKey as BlsPublicKey};
use crate::core_types::validator::v0::{ValidatorV0, ValidatorV0Getters, ValidatorV0Setters};
use dashcore::{ProTxHash, PubkeyHash};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};

/// Version 0
pub mod v0;

/// A validator in the context of a quorum
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
pub enum Validator {
    /// Version 0
    V0(ValidatorV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for Validator {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for Validator {}

impl ValidatorV0Getters for Validator {
    fn pro_tx_hash(&self) -> &ProTxHash {
        match self {
            Validator::V0(v0) => v0.pro_tx_hash(),
        }
    }

    fn public_key(&self) -> &Option<BlsPublicKey<Bls12381G2Impl>> {
        match self {
            Validator::V0(v0) => v0.public_key(),
        }
    }

    fn node_ip(&self) -> &String {
        match self {
            Validator::V0(v0) => v0.node_ip(),
        }
    }

    fn node_id(&self) -> &PubkeyHash {
        match self {
            Validator::V0(v0) => v0.node_id(),
        }
    }

    fn core_port(&self) -> u16 {
        match self {
            Validator::V0(v0) => v0.core_port(),
        }
    }

    fn platform_http_port(&self) -> u16 {
        match self {
            Validator::V0(v0) => v0.platform_http_port(),
        }
    }

    fn platform_p2p_port(&self) -> u16 {
        match self {
            Validator::V0(v0) => v0.platform_p2p_port(),
        }
    }

    fn is_banned(&self) -> bool {
        match self {
            Validator::V0(v0) => v0.is_banned(),
        }
    }
}

impl ValidatorV0Setters for Validator {
    fn set_pro_tx_hash(&mut self, pro_tx_hash: ProTxHash) {
        match self {
            Validator::V0(v0) => v0.set_pro_tx_hash(pro_tx_hash),
        }
    }

    fn set_public_key(&mut self, public_key: Option<BlsPublicKey<Bls12381G2Impl>>) {
        match self {
            Validator::V0(v0) => v0.set_public_key(public_key),
        }
    }

    fn set_node_ip(&mut self, node_ip: String) {
        match self {
            Validator::V0(v0) => v0.set_node_ip(node_ip),
        }
    }

    fn set_node_id(&mut self, node_id: PubkeyHash) {
        match self {
            Validator::V0(v0) => v0.set_node_id(node_id),
        }
    }

    fn set_core_port(&mut self, core_port: u16) {
        match self {
            Validator::V0(v0) => v0.set_core_port(core_port),
        }
    }

    fn set_platform_http_port(&mut self, platform_http_port: u16) {
        match self {
            Validator::V0(v0) => v0.set_platform_http_port(platform_http_port),
        }
    }

    fn set_platform_p2p_port(&mut self, platform_p2p_port: u16) {
        match self {
            Validator::V0(v0) => v0.set_platform_p2p_port(platform_p2p_port),
        }
    }

    fn set_is_banned(&mut self, is_banned: bool) {
        match self {
            Validator::V0(v0) => v0.set_is_banned(is_banned),
        }
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::core_types::validator::v0::ValidatorV0;
    use dashcore::hashes::Hash;
    use dashcore::{ProTxHash, PubkeyHash};

    fn fixture() -> Validator {
        Validator::V0(ValidatorV0 {
            pro_tx_hash: ProTxHash::from_byte_array([0x11; 32]),
            public_key: None,
            node_ip: "127.0.0.1".to_string(),
            node_id: PubkeyHash::from_byte_array([0x22; 20]),
            core_port: 9999,
            platform_http_port: 443,
            platform_p2p_port: 26656,
            is_banned: false,
        })
    }

    fn assert_v0_fields(v: &Validator) {
        let Validator::V0(v0) = v;
        assert_eq!(v0.pro_tx_hash, ProTxHash::from_byte_array([0x11; 32]), "pro_tx_hash");
        assert_eq!(v0.public_key, None, "public_key");
        assert_eq!(v0.node_ip, "127.0.0.1", "node_ip");
        assert_eq!(v0.node_id, PubkeyHash::from_byte_array([0x22; 20]), "node_id");
        assert_eq!(v0.core_port, 9999, "core_port");
        assert_eq!(v0.platform_http_port, 443, "platform_http_port");
        assert_eq!(v0.platform_p2p_port, 26656, "platform_p2p_port");
        assert!(!v0.is_banned, "is_banned");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = Validator::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = Validator::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}
