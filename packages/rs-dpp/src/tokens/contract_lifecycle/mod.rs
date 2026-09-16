//! Per-issuer token lifecycle: the supply rollup every contract that issues tokens carries and
//! the marker that records its destruction.
//!
//! Drive keeps one [`ContractTokenLifecycle`] per contract that issues tokens. The record's
//! `issued_supply` is the sum of the contract's token supplies, kept current by every native
//! supply write in the same batch, so the total supply of an issuer is one read instead of a
//! walk over its tokens. Destroying an issuer sets `wiped` and moves the rollup into the
//! destroyed supply ledger; the record itself is permanent, which is what makes the issuer's
//! token ids unusable for ever.

#[cfg(all(
    feature = "json-conversion",
    any(feature = "fixtures-and-mocks", feature = "serde-conversion")
))]
use crate::serialization::JsonConvertible;
#[cfg(any(feature = "fixtures-and-mocks", feature = "value-conversion"))]
use crate::serialization::ValueConvertible;
use crate::tokens::contract_lifecycle::v0::{ContractTokenLifecycleV0, ContractWipeV0};
use crate::ProtocolError;
use bincode::Encode;
use derive_more::From;
use platform_serialization::de::Decode;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_version::version::PlatformVersion;
use platform_versioning::PlatformVersioned;

mod methods;
pub mod v0;

/// The lifecycle record of a contract that issues tokens.
#[cfg_attr(
    all(
        feature = "json-conversion",
        any(feature = "fixtures-and-mocks", feature = "serde-conversion")
    ),
    derive(JsonConvertible)
)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformVersioned,
    From,
    PartialEq,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[cfg_attr(
    any(feature = "fixtures-and-mocks", feature = "serde-conversion"),
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(
    any(feature = "fixtures-and-mocks", feature = "value-conversion"),
    derive(ValueConvertible)
)]
pub enum ContractTokenLifecycle {
    #[cfg_attr(
        any(feature = "fixtures-and-mocks", feature = "serde-conversion"),
        serde(rename = "0")
    )]
    V0(ContractTokenLifecycleV0),
}

/// The moment an issuer was destroyed.
#[cfg_attr(
    all(
        feature = "json-conversion",
        any(feature = "fixtures-and-mocks", feature = "serde-conversion")
    ),
    derive(JsonConvertible)
)]
#[derive(
    Debug,
    Clone,
    Encode,
    Decode,
    PlatformDeserialize,
    PlatformSerialize,
    PlatformVersioned,
    From,
    PartialEq,
)]
#[platform_serialize(unversioned)] //versioned directly, no need to use platform_version
#[cfg_attr(
    any(feature = "fixtures-and-mocks", feature = "serde-conversion"),
    derive(serde::Serialize, serde::Deserialize),
    serde(tag = "$formatVersion")
)]
#[cfg_attr(
    any(feature = "fixtures-and-mocks", feature = "value-conversion"),
    derive(ValueConvertible)
)]
pub enum ContractWipe {
    #[cfg_attr(
        any(feature = "fixtures-and-mocks", feature = "serde-conversion"),
        serde(rename = "0")
    )]
    V0(ContractWipeV0),
}

/// What a token's issuer state means for the token: live tokens behave as before, tokens of a
/// destroyed issuer are unusable on every path and reported as absent by every query and proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenLifecycle {
    /// The issuer is live.
    Live,
    /// The issuer was destroyed at this block height.
    Wiped {
        /// The height of the block that destroyed the issuer.
        block_height: u64,
    },
}

impl TokenLifecycle {
    /// Whether the token's issuer was destroyed.
    pub fn is_wiped(&self) -> bool {
        matches!(self, TokenLifecycle::Wiped { .. })
    }
}

impl ContractTokenLifecycle {
    /// Creates a live record with the given supply rollup.
    pub fn new(
        issued_supply: u128,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .token_versions
            .contract_token_lifecycle_default_structure_version
        {
            0 => Ok(ContractTokenLifecycle::V0(ContractTokenLifecycleV0 {
                issued_supply,
                wiped: None,
            })),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ContractTokenLifecycle::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

impl ContractWipe {
    /// Records a destruction at the given block.
    pub fn new(
        block_height: u64,
        block_time_ms: u64,
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        match platform_version
            .dpp
            .token_versions
            .contract_token_lifecycle_default_structure_version
        {
            0 => Ok(ContractWipe::V0(ContractWipeV0 {
                block_height,
                block_time_ms,
            })),
            version => Err(ProtocolError::UnknownVersionMismatch {
                method: "ContractWipe::new".to_string(),
                known_versions: vec![0],
                received: version,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serialization::{PlatformDeserializable, PlatformSerializable};
    use crate::tokens::contract_lifecycle::v0::{
        ContractTokenLifecycleV0Accessors, ContractWipeV0Accessors,
    };

    #[test]
    fn should_round_trip_a_live_record_through_bytes() {
        let platform_version = PlatformVersion::latest();
        let record = ContractTokenLifecycle::new(u128::MAX - 7, platform_version)
            .expect("expected a record");

        let bytes = record.serialize_to_bytes().expect("expected bytes");
        let restored =
            ContractTokenLifecycle::deserialize_from_bytes(&bytes).expect("expected a record");

        assert_eq!(restored, record);
        assert_eq!(restored.issued_supply(), u128::MAX - 7);
        assert!(!restored.is_wiped());
        assert_eq!(restored.token_lifecycle(), TokenLifecycle::Live);
    }

    #[test]
    fn should_round_trip_a_wiped_record_through_bytes() {
        let platform_version = PlatformVersion::latest();
        let mut record =
            ContractTokenLifecycle::new(42, platform_version).expect("expected a record");
        let wipe = ContractWipe::new(1_000, 2_000, platform_version).expect("expected a wipe");
        record.set_wiped(wipe);

        let bytes = record.serialize_to_bytes().expect("expected bytes");
        let restored =
            ContractTokenLifecycle::deserialize_from_bytes(&bytes).expect("expected a record");

        assert_eq!(restored, record);
        assert!(restored.is_wiped());
        let wiped = restored.wiped().expect("expected the wipe marker");
        assert_eq!(wiped.block_height(), 1_000);
        assert_eq!(wiped.block_time_ms(), 2_000);
        assert_eq!(
            restored.token_lifecycle(),
            TokenLifecycle::Wiped {
                block_height: 1_000
            }
        );
    }

    #[test]
    fn should_move_the_rollup_with_checked_arithmetic() {
        let platform_version = PlatformVersion::latest();
        let mut record = ContractTokenLifecycle::new(u128::MAX - 1, platform_version)
            .expect("expected a record");

        record
            .checked_add_issued_supply(1)
            .expect("expected the addition to fit");
        assert_eq!(record.issued_supply(), u128::MAX);
        assert!(record.checked_add_issued_supply(1).is_err());

        record
            .checked_sub_issued_supply(u128::MAX)
            .expect("expected the subtraction to fit");
        assert_eq!(record.issued_supply(), 0);
        assert!(record.checked_sub_issued_supply(1).is_err());
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
    use platform_value::platform_value;
    use serde_json::json;

    fn fixture() -> ContractTokenLifecycle {
        ContractTokenLifecycle::V0(ContractTokenLifecycleV0 {
            issued_supply: 1_234_567,
            wiped: Some(ContractWipe::V0(ContractWipeV0 {
                block_height: 77,
                block_time_ms: 88_000,
            })),
        })
    }

    // The rollup is a `u128` inside an internally tagged enum: it is a number while it fits the
    // JavaScript safe range and a string above it, and the `Value` form keeps it as a `u64`
    // while it fits one. Drive stores the bincode bytes, where it is a native `u128`.
    #[test]
    fn json_round_trip_with_full_wire_shape() {
        use crate::serialization::JsonConvertible;
        let original = fixture();
        let json = original.to_json().expect("to_json");
        assert_eq!(
            json,
            json!({
                "$formatVersion": "0",
                "issuedSupply": 1_234_567,
                "wiped": {
                    "$formatVersion": "0",
                    "blockHeight": 77,
                    "blockTimeMs": 88_000,
                },
            })
        );
        let recovered = ContractTokenLifecycle::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }

    #[test]
    fn value_round_trip_with_full_wire_shape() {
        use crate::serialization::ValueConvertible;
        let original = fixture();
        let value = original.to_object().expect("to_object");
        assert_eq!(
            value,
            platform_value!({
                "$formatVersion": "0",
                "issuedSupply": 1_234_567u64,
                "wiped": {
                    "$formatVersion": "0",
                    "blockHeight": 77u64,
                    "blockTimeMs": 88_000u64,
                },
            })
        );
        let recovered = ContractTokenLifecycle::from_object(value).expect("from_object");
        assert_eq!(original, recovered);
    }

    #[test]
    fn json_stringifies_a_rollup_above_the_safe_integer_range() {
        use crate::serialization::JsonConvertible;
        let original = ContractTokenLifecycle::V0(ContractTokenLifecycleV0 {
            issued_supply: u128::MAX,
            wiped: None,
        });
        let json = original.to_json().expect("to_json");
        assert_eq!(json["issuedSupply"], json!(u128::MAX.to_string()));
        assert!(json["wiped"].is_null());
        let recovered = ContractTokenLifecycle::from_json(json).expect("from_json");
        assert_eq!(original, recovered);
    }
}
