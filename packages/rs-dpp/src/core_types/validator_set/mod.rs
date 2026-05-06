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
    /// (The dedicated `bls_pubkey_serde` unit tests independently cover the
    /// pubkey round-trip.)
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

    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let (original, validator_pubkey, threshold_pubkey) = build_fixture();
        let json = original.to_json().expect("to_json");

        // BLS public keys serialize as 96-char compressed-G1 hex on the HR
        // path. We interpolate `serde_json::to_value` of the same pubkeys
        // rather than baking in the literal hex — the values are deterministic
        // for the seeded `StdRng(42)` but inlining a 96-char string per key
        // hurts readability (and the `bls_pubkey_serde` module has its own
        // dedicated tests for the BLS round-trip). The rest of the wire
        // structure is fully asserted: `tag = "$formatVersion"` convention,
        // snake_case inner fields (no `rename_all`), `BTreeMap` members
        // emitted as a struct keyed by ProTxHash hex, hash fields as lowercase
        // hex strings, sized-int fields preserved. The inner Validator's
        // own `$formatVersion` tag (now applied) appears alongside its
        // other snake_case fields.
        let validator_pk_json = serde_json::to_value(&validator_pubkey).expect("pk to json");
        let threshold_pk_json = serde_json::to_value(&threshold_pubkey).expect("pk to json");
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
                        "public_key": validator_pk_json,
                        "node_ip": "127.0.0.1",
                        "node_id": "2222222222222222222222222222222222222222",
                        "core_port": 9999,
                        "platform_http_port": 443,
                        "platform_p2p_port": 26656,
                        "is_banned": false,
                    }
                },
                "threshold_public_key": threshold_pk_json,
            })
        );
        let recovered = ValidatorSet::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    #[ignore = "Pending dashcore PR https://github.com/dashpay/rust-dashcore/pull/729 \
                (adds dual-shape visitor to `hashes::serde_macros::SerdeHash` — \
                companion to #708 which fixed the same root cause for `OutPoint`'s \
                separate `serde_struct_human_string_impl!` macro). Wrapping \
                `ValidatorSet` in `tag = \"$formatVersion\"` routes deserialization \
                through serde's `ContentDeserializer` which always reports \
                `is_human_readable=true`; the bytes from a non-HR \
                `platform_value::Value` source are then replayed into the HR \
                branch and the old `HexVisitor::visit_str` sees a 32-byte \
                sequence (interpreted as 32 UTF-8 chars) instead of the \
                expected 64-char hex form. Affects \
                `ProTxHash`/`PubkeyHash`/`QuorumHash`. Once #729 lands and \
                we bump dashcore, drop this `#[ignore]`."]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let (original, validator_pubkey, threshold_pubkey) = build_fixture();
        let value = original.to_object().expect("to_object");

        // On the non-HR path BLS pubkeys serialize as a 48-byte tuple, which
        // platform_value collapses into a typed bytes variant. Same as for
        // JSON: we interpolate the canonical Value form of the actual
        // fixture pubkeys rather than spelling out 48 bytes inline. Hash
        // fields (`Bytes32`/`Bytes20`) are explicit.
        // ProTxHash on the BTreeMap-key side serializes through dashcore as
        // a `Value::Bytes32` (32-byte sized variant) on the non-HR path.
        // The `platform_value!` macro doesn't accept non-string keys (it only
        // takes literal/parenthesized-expression keys that implement
        // `Into<Value>` from a string-like form), so we build the inner
        // members map by hand for the typed-bytes key.
        let validator_pk_value = platform_value::to_value(&validator_pubkey).expect("pk to value");
        let threshold_pk_value = platform_value::to_value(&threshold_pubkey).expect("pk to value");
        // Note: members are typed `BTreeMap<ProTxHash, ValidatorV0>` (not
        // `BTreeMap<ProTxHash, Validator>`), so the inner is the bare V0
        // struct without its enum's `$formatVersion` tag.
        let inner_validator = platform_value!({
            "pro_tx_hash": Value::Bytes32([0x11; 32]),
            "public_key": validator_pk_value,
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
            "threshold_public_key": threshold_pk_value,
        });
        assert_eq!(value, expected);
        let recovered = ValidatorSet::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }
}
