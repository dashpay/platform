use crate::bls_signatures::{Bls12381G2Impl, PublicKey as BlsPublicKey};
use crate::core_types::validator::v0::ValidatorV0;
use crate::core_types::validator_set::v0::{
    ValidatorSetV0, ValidatorSetV0Getters, ValidatorSetV0Setters,
};
#[cfg(feature = "core-types-serialization")]
use crate::ProtocolError;
#[cfg(feature = "core-types-serialization")]
use bincode::{Decode, Encode};
use dashcore::{ProTxHash, QuorumHash};
#[cfg(feature = "core-types-serialization")]
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

/// Version 0
pub mod v0;

/// The validator set is only slightly different from a quorum as it does not contain non valid
/// members
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde-conversion", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "core-types-serialization",
    derive(Encode, Decode, PlatformDeserialize, PlatformSerialize),
    platform_serialize(limit = 15000, unversioned)
)]
pub enum ValidatorSet {
    /// Version 0
    V0(ValidatorSetV0),
}

#[cfg(all(feature = "json-conversion", feature = "serde-conversion"))]
impl crate::serialization::JsonConvertible for ValidatorSet {}

#[cfg(all(feature = "value-conversion", feature = "serde-conversion"))]
impl crate::serialization::ValueConvertible for ValidatorSet {}

impl Display for ValidatorSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidatorSet::V0(v0) => write!(f, "{}", v0),
        }
    }
}

impl ValidatorSetV0Getters for ValidatorSet {
    fn quorum_hash(&self) -> &QuorumHash {
        match self {
            ValidatorSet::V0(v0) => v0.quorum_hash(),
        }
    }

    fn quorum_index(&self) -> Option<u32> {
        match self {
            ValidatorSet::V0(v0) => v0.quorum_index(),
        }
    }

    fn core_height(&self) -> u32 {
        match self {
            ValidatorSet::V0(v0) => v0.core_height(),
        }
    }

    fn members(&self) -> &BTreeMap<ProTxHash, ValidatorV0> {
        match self {
            ValidatorSet::V0(v0) => v0.members(),
        }
    }

    fn members_mut(&mut self) -> &mut BTreeMap<ProTxHash, ValidatorV0> {
        match self {
            ValidatorSet::V0(v0) => v0.members_mut(),
        }
    }

    fn members_owned(self) -> BTreeMap<ProTxHash, ValidatorV0> {
        match self {
            ValidatorSet::V0(v0) => v0.members_owned(),
        }
    }

    fn threshold_public_key(&self) -> &BlsPublicKey<Bls12381G2Impl> {
        match self {
            ValidatorSet::V0(v0) => v0.threshold_public_key(),
        }
    }
}

impl ValidatorSetV0Setters for ValidatorSet {
    fn set_quorum_hash(&mut self, quorum_hash: QuorumHash) {
        match self {
            ValidatorSet::V0(v0) => v0.set_quorum_hash(quorum_hash),
        }
    }

    fn set_quorum_index(&mut self, index: Option<u32>) {
        match self {
            ValidatorSet::V0(v0) => v0.set_quorum_index(index),
        }
    }

    fn set_core_height(&mut self, core_height: u32) {
        match self {
            ValidatorSet::V0(v0) => v0.set_core_height(core_height),
        }
    }

    fn set_members(&mut self, members: BTreeMap<ProTxHash, ValidatorV0>) {
        match self {
            ValidatorSet::V0(v0) => v0.set_members(members),
        }
    }

    fn set_threshold_public_key(&mut self, threshold_public_key: BlsPublicKey<Bls12381G2Impl>) {
        match self {
            ValidatorSet::V0(v0) => v0.set_threshold_public_key(threshold_public_key),
        }
    }
}

#[cfg(all(test, feature = "json-conversion", feature = "value-conversion", feature = "serde-conversion"))]
mod json_convertible_tests {
    use super::*;
    use crate::core_types::validator::v0::ValidatorV0;
    use crate::core_types::validator_set::v0::ValidatorSetV0;
    use dashcore::blsful::{Bls12381G2Impl, SecretKey};
    use dashcore::hashes::Hash;
    use dashcore::{ProTxHash, PubkeyHash, QuorumHash};
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::BTreeMap;

    fn fixture() -> ValidatorSet {
        let mut rng = StdRng::seed_from_u64(42);
        let pro_tx_hash = ProTxHash::from_byte_array([0x11; 32]);
        let validator_v0 = ValidatorV0 {
            pro_tx_hash,
            public_key: Some(SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key()),
            node_ip: "127.0.0.1".to_string(),
            node_id: PubkeyHash::from_byte_array([0x22; 20]),
            core_port: 9999,
            platform_http_port: 443,
            platform_p2p_port: 26656,
            is_banned: false,
        };
        let mut members = BTreeMap::new();
        members.insert(pro_tx_hash, validator_v0);

        ValidatorSet::V0(ValidatorSetV0 {
            quorum_hash: QuorumHash::from_byte_array([0x33; 32]),
            quorum_index: Some(7),
            core_height: 1234,
            members,
            threshold_public_key: SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key(),
        })
    }

    fn assert_v0_fields(v: &ValidatorSet) {
        let ValidatorSet::V0(rec) = v;
        assert_eq!(rec.quorum_hash.as_byte_array(), &[0x33; 32], "quorum_hash");
        assert_eq!(rec.quorum_index, Some(7), "quorum_index");
        assert_eq!(rec.core_height, 1234, "core_height");
        assert_eq!(rec.members.len(), 1, "members count");
    }

    #[test]
    fn json_round_trip_with_per_property_assertions() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        let recovered = ValidatorSet::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }

    #[test]
    #[ignore = "Distinct bug: BTreeMap<ProTxHash, ValidatorV0> map-key asymmetry. \
                platform_value's MapKeySerializer reports is_human_readable=true (forces \
                ProTxHash through its hex-string serialize path → Value::Text key), but \
                platform_value's Deserializer reports is_human_readable=false (forces \
                ProTxHash through its bytes-expecting BytesVisitor on the deserialize side). \
                Round-trip fails with 'invalid type: string ..., expected bytes'. The BlsPublicKey \
                borrowed-string bug — the original reason this test was ignored — is now fixed \
                (json_round_trip passes); see core_types::bls_pubkey_serde."]
    fn value_round_trip_with_per_property_assertions() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        let recovered = ValidatorSet::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
        assert_v0_fields(&recovered);
    }
}
