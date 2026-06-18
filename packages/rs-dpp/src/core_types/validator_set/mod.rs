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
#[cfg_attr(
    feature = "serde-conversion",
    derive(Serialize, Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(
    feature = "core-types-serialization",
    derive(Encode, Decode, PlatformDeserialize, PlatformSerialize),
    platform_serialize(limit = 15000, unversioned)
)]
pub enum ValidatorSet {
    /// Version 0
    #[cfg_attr(feature = "serde-conversion", serde(rename = "0"))]
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

#[cfg(all(
    test,
    feature = "json-conversion",
    feature = "value-conversion",
    feature = "serde-conversion"
))]
mod json_convertible_tests {
    use super::*;
    use crate::core_types::validator::v0::ValidatorV0;
    use crate::core_types::validator_set::v0::ValidatorSetV0;
    use dashcore::blsful::{Bls12381G2Impl, SecretKey};
    use dashcore::hashes::Hash;
    use dashcore::{ProTxHash, PubkeyHash, QuorumHash};
    use platform_value::{platform_value, Value};
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use serde_json::json;
    use std::collections::BTreeMap;

    /// Build the fixture with deterministic BLS keys (seeded RNG) plus the
    /// derived public-key wire forms — both as `serde_json::Value` (HR: 96-char
    /// hex) and `platform_value::Value` (non-HR: typed-bytes variant). The BLS
    /// keys ARE deterministic, but the 96-char hex / 48-byte literal is too
    /// unwieldy to inline as a string constant, so we interpolate the actual
    /// `to_value`/`to_json` of the same pubkey objects we put in the fixture.
    /// (The dedicated `serialization::dashcore::bls_pubkey` unit tests
    /// independently cover the pubkey round-trip.)
    fn build_fixture() -> (
        ValidatorSet,
        BlsPublicKey<Bls12381G2Impl>,
        BlsPublicKey<Bls12381G2Impl>,
    ) {
        let mut rng = StdRng::seed_from_u64(42);
        let pro_tx_hash = ProTxHash::from_byte_array([0x11; 32]);
        let validator_pubkey = SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key();
        let validator_v0 = ValidatorV0 {
            pro_tx_hash,
            public_key: Some(validator_pubkey),
            node_ip: "127.0.0.1".to_string(),
            node_id: PubkeyHash::from_byte_array([0x22; 20]),
            core_port: 9999,
            platform_http_port: 443,
            platform_p2p_port: 26656,
            is_banned: false,
        };
        let mut members = BTreeMap::new();
        members.insert(pro_tx_hash, validator_v0);

        let threshold_pubkey = SecretKey::<Bls12381G2Impl>::random(&mut rng).public_key();
        let set = ValidatorSet::V0(ValidatorSetV0 {
            quorum_hash: QuorumHash::from_byte_array([0x33; 32]),
            quorum_index: Some(7),
            core_height: 1234,
            members,
            threshold_public_key: threshold_pubkey,
        });
        (set, validator_pubkey, threshold_pubkey)
    }

    // Deterministic compressed-G1 BLS pubkeys for the seeded `StdRng(42)`
    // fixture, spelled out as hex (96 chars = 48 bytes) so the wire format is
    // visible and verifiable in-place — neither path interpolates `to_json` /
    // `to_value` of the object under test. JSON (HR) uses the hex string
    // directly; the non-HR `Value` form is the *same* 48 bytes decoded into an
    // `Array` of `U8` (blstrs serializes the pubkey through a `u8` tuple, which
    // platform_value collects as `Array[U8]` — NOT a typed `Bytes` variant).
    const VALIDATOR_PK_HEX: &str =
        "85d81dd12c73cca83f7d1bf8b78fadb695e3a2bc21d53b35ff2f74eaa28c6e163c98d3d5f9bb7252b4d836e484c7cc60";
    const THRESHOLD_PK_HEX: &str =
        "969c5d5873f49aa994c5f6a850924ca1840c4ad1791aaaecd90093d4a5c0c3799f2d98540f5366cfa0a33f143fd69263";

    fn bls_pubkey_value(hex: &str) -> Value {
        Value::Array(
            (0..hex.len() / 2)
                .map(|i| Value::U8(u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).expect("hex")))
                .collect(),
        )
    }

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let (original, ..) = build_fixture();
        let json = original.to_json().expect("to_json");

        // BLS public keys serialize as 96-char compressed-G1 hex on the HR
        // path. The keys are deterministic (seeded `StdRng(42)`), so the exact
        // hex is baked in below — the on-wire format is directly visible and
        // verifiable in-place. The rest of the wire structure is fully
        // asserted too: `tag = "$formatVersion"` convention, snake_case inner
        // fields (no `rename_all`), `BTreeMap` members emitted as a struct
        // keyed by ProTxHash hex, hash fields as lowercase hex strings,
        // sized-int fields preserved. The inner Validator's own
        // `$formatVersion` tag (now applied) appears alongside its other
        // snake_case fields. (`serialization::dashcore::bls_pubkey` additionally
        // has its own dedicated BLS round-trip tests.)
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "quorum_hash": "3333333333333333333333333333333333333333333333333333333333333333",
                "quorum_index": 7,
                "core_height": 1234,
                "members": {
                    "1111111111111111111111111111111111111111111111111111111111111111": {
                        // Note: members are typed `BTreeMap<ProTxHash, ValidatorV0>` (not
                        // `BTreeMap<ProTxHash, Validator>`), so the inner is the bare V0
                        // struct without its enum's `$formatVersion` tag.
                        "pro_tx_hash": "1111111111111111111111111111111111111111111111111111111111111111",
                        "public_key": VALIDATOR_PK_HEX,
                        "node_ip": "127.0.0.1",
                        "node_id": "2222222222222222222222222222222222222222",
                        "core_port": 9999,
                        "platform_http_port": 443,
                        "platform_p2p_port": 26656,
                        "is_banned": false,
                    }
                },
                "threshold_public_key": THRESHOLD_PK_HEX,
            })
        );
        let recovered = ValidatorSet::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let (original, ..) = build_fixture();
        let value = original.to_object().expect("to_object");

        // BLS pubkeys on the non-HR path = the 48 compressed-G1 bytes as an
        // `Array` of `U8`, decoded from the *same* hex consts the JSON test
        // uses (one source of truth, no interpolation of the object under
        // test). Hash fields (`Bytes32`/`Bytes20`) are explicit. The
        // `platform_value!` macro doesn't accept non-string keys, so the
        // members map (keyed by a typed-bytes `ProTxHash`) is built by hand.
        // Note: members are typed `BTreeMap<ProTxHash, ValidatorV0>` (not
        // `BTreeMap<ProTxHash, Validator>`), so the inner is the bare V0
        // struct without its enum's `$formatVersion` tag.
        let inner_validator = platform_value!({
            "pro_tx_hash": Value::Bytes32([0x11; 32]),
            "public_key": bls_pubkey_value(VALIDATOR_PK_HEX),
            "node_ip": "127.0.0.1",
            "node_id": Value::Bytes20([0x22; 20]),
            "core_port": 9999u16,
            "platform_http_port": 443u16,
            "platform_p2p_port": 26656u16,
            "is_banned": false,
        });
        let members_value = Value::Map(vec![(Value::Bytes32([0x11; 32]), inner_validator)]);
        let expected = platform_value!({
            "$formatVersion": "0",
            "quorum_hash": Value::Bytes32([0x33; 32]),
            "quorum_index": 7u32,
            "core_height": 1234u32,
            "members": members_value,
            "threshold_public_key": bls_pubkey_value(THRESHOLD_PK_HEX),
        });
        assert_eq!(value, expected);
        let recovered = ValidatorSet::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
